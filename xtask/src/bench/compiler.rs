use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use super::benchmark_profile::BenchmarkProfile;
use crate::cargo::Cargo;
use crate::failure::TaskFailure;
use crate::workspace::CargoWorkspace;

#[derive(Debug, PartialEq, Eq)]
pub struct BuiltCompiler {
    pub executable: PathBuf,
    pub crate_names: BTreeSet<String>,
}

pub struct CompilerBuild<'a> {
    workspace: &'a Path,
    target_directory: &'a Path,
    variant: BuildVariant,
    profile: BenchmarkProfile,
}

#[derive(Clone, Copy)]
enum BuildVariant {
    Plain,
    CountersOverPlain,
}

impl BuildVariant {
    const COUNTERS_FEATURE: [&'static str; 2] = ["--features", "counters"];

    fn features(self) -> &'static [&'static str] {
        match self {
            Self::Plain => &[],
            Self::CountersOverPlain => &Self::COUNTERS_FEATURE,
        }
    }

    fn cleans_first(self) -> bool {
        match self {
            Self::Plain => true,
            Self::CountersOverPlain => false,
        }
    }
}

impl<'a> CompilerBuild<'a> {
    const BUILD: [&'static str; 3] = ["build", "--quiet", "--message-format=json"];
    const CLEAN: [&'static str; 2] = ["clean", "--quiet"];
    const PACKAGE_FLAG: &'static str = "--package";
    const TARGET_DIRECTORY_FLAG: &'static str = "--target-dir";
    const MANIFEST: &'static str = "Cargo.toml";

    pub fn new(workspace: &'a Path, target_directory: &'a Path) -> Self {
        Self {
            workspace,
            target_directory,
            variant: BuildVariant::Plain,
            profile: BenchmarkProfile,
        }
    }

    pub fn with_counters_over_plain_build(self) -> Self {
        Self {
            variant: BuildVariant::CountersOverPlain,
            ..self
        }
    }

    pub fn run(&self) -> Result<BuiltCompiler, TaskFailure> {
        let workspace = CargoWorkspace::read(self.workspace)?;
        self.prepare_target(&workspace.package_names())?;
        let messages = self.build_compiler()?;
        let executable = self
            .read_executable(&messages)
            .ok_or_else(|| TaskFailure::compiler_missing(self.workspace))?;
        Ok(BuiltCompiler {
            executable,
            crate_names: workspace.crate_names(),
        })
    }

    fn prepare_target(&self, package_names: &BTreeSet<String>) -> Result<(), TaskFailure> {
        self.preparation(package_names)
            .into_iter()
            .try_for_each(|mut clean| {
                Cargo::run(&mut clean, |stderr| {
                    TaskFailure::tool_failed("cargo clean", stderr)
                })
                .map(drop)
            })
    }

    fn preparation(&self, package_names: &BTreeSet<String>) -> Option<Command> {
        self.variant
            .cleans_first()
            .then(|| self.benchmark_clean(package_names))
    }

    fn benchmark_clean(&self, package_names: &BTreeSet<String>) -> Command {
        let packages = package_names
            .iter()
            .flat_map(|name| [Self::PACKAGE_FLAG, name.as_str()]);
        let mut clean = self.benchmark_cargo(Self::CLEAN);
        clean.args(packages);
        clean
    }

    fn build_compiler(&self) -> Result<String, TaskFailure> {
        let mut build = self.compiler_build();
        let stdout = Cargo::run(&mut build, |stderr| {
            TaskFailure::build_failed("the compiler", stderr)
        })?;
        Ok(String::from_utf8_lossy(&stdout).into_owned())
    }

    fn compiler_build(&self) -> Command {
        let mut build = self.benchmark_cargo(Self::BUILD);
        build.args(self.variant.features());
        build
    }

    fn benchmark_cargo<'b>(&self, arguments: impl IntoIterator<Item = &'b str>) -> Command {
        let mut command = self.cargo_in_checkout();
        command
            .args(arguments)
            .args(self.profile.cargo_arguments())
            .arg(Self::TARGET_DIRECTORY_FLAG)
            .arg(self.target_directory);
        command
    }

    fn cargo_in_checkout(&self) -> Command {
        let mut command = Cargo::command();
        command.current_dir(self.workspace);
        command
    }

    fn read_executable(&self, messages: &str) -> Option<PathBuf> {
        let manifest = self.workspace.join(Self::MANIFEST);
        messages
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .map(Artifact::new)
            .filter(|artifact| artifact.belongs_to(&manifest))
            .find_map(|artifact| artifact.executable())
    }
}

struct Artifact {
    message: Value,
}

impl Artifact {
    const ARTIFACT_REASON: &'static str = "compiler-artifact";
    const BINARY_KIND: &'static str = "bin";

    fn new(message: Value) -> Self {
        Self { message }
    }

    fn belongs_to(&self, manifest: &Path) -> bool {
        self.message["reason"] == Self::ARTIFACT_REASON
            && self.message["manifest_path"].as_str().map(Path::new) == Some(manifest)
    }

    fn executable(&self) -> Option<PathBuf> {
        let is_binary = self.has_kind(Self::BINARY_KIND);
        let executable = self.message["executable"].as_str().map(PathBuf::from);
        executable.filter(|_| is_binary)
    }

    fn has_kind(&self, kind: &str) -> bool {
        self.message["target"]["kind"]
            .as_array()
            .is_some_and(|kinds| kinds.iter().any(|candidate| candidate == kind))
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;
    use crate::workspace::WorkspaceFixture;

    const MESSAGES: &str = include_str!("../../fixtures/bench/cargo-build.json");

    struct ProfileArguments {
        arguments: Vec<String>,
    }

    impl ProfileArguments {
        fn new(command: &Command) -> Self {
            let arguments = command
                .get_args()
                .map(|argument| argument.to_string_lossy().into_owned())
                .collect();
            Self { arguments }
        }

        fn assert_selects_the_defined_profile(&self) {
            let selected: BTreeSet<_> = self.values_of("--profile").collect();
            let defined: BTreeSet<_> = self
                .values_of("--config")
                .filter_map(|setting| setting.strip_prefix("profile."))
                .filter_map(|setting| setting.split_once('.'))
                .map(|(profile, _)| profile)
                .collect();
            assert_eq!(selected.len(), 1);
            assert_eq!(selected, defined);
        }

        fn values_of<'a>(&'a self, flag: &'a str) -> impl Iterator<Item = &'a str> {
            self.arguments
                .windows(2)
                .filter(move |pair| pair[0] == flag)
                .map(|pair| pair[1].as_str())
        }
    }

    #[test]
    fn takes_the_binary_of_the_checkout_package() {
        let executable = CompilerBuild::new(Path::new("/checkout"), Path::new("/checkout/target"))
            .read_executable(MESSAGES);
        assert_eq!(
            executable,
            Some(PathBuf::from("/checkout/target/benchmark/ledger-lang"))
        );
    }

    #[test]
    fn cleans_every_package_of_the_workspace() {
        let package_names = WorkspaceFixture::workspace().package_names();
        let clean = CompilerBuild::new(Path::new("/checkout"), Path::new("/checkout/target"))
            .benchmark_clean(&package_names);
        let arguments: Vec<_> = clean.get_args().collect();
        for package in ["ledger-lang", "ledger-syntax"] {
            assert!(
                arguments
                    .windows(2)
                    .any(|pair| pair == ["--package", package])
            );
        }
    }

    #[test]
    fn builds_the_profile_that_its_configuration_defines() {
        let build = CompilerBuild::new(Path::new("/checkout"), Path::new("/checkout/target"))
            .compiler_build();
        ProfileArguments::new(&build).assert_selects_the_defined_profile();
    }

    #[test]
    fn cleans_the_profile_that_its_configuration_defines() {
        let package_names = WorkspaceFixture::workspace().package_names();
        let clean = CompilerBuild::new(Path::new("/checkout"), Path::new("/checkout/target"))
            .benchmark_clean(&package_names);
        ProfileArguments::new(&clean).assert_selects_the_defined_profile();
    }

    #[test]
    fn builds_the_plain_compiler_without_features() {
        let build = CompilerBuild::new(Path::new("/checkout"), Path::new("/checkout/target"))
            .compiler_build();
        assert!(!build.get_args().any(|argument| argument == "--features"));
    }

    #[test]
    fn builds_the_counters_compiler_with_the_counters_feature() {
        let build = CompilerBuild::new(Path::new("/checkout"), Path::new("/checkout/target"))
            .with_counters_over_plain_build()
            .compiler_build();
        let arguments: Vec<_> = build.get_args().collect();
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--features", "counters"])
        );
    }

    #[test]
    fn cleans_before_the_plain_build() {
        let package_names = WorkspaceFixture::workspace().package_names();
        let preparation = CompilerBuild::new(Path::new("/checkout"), Path::new("/checkout/target"))
            .preparation(&package_names);
        let arguments: Vec<_> = preparation.iter().flat_map(Command::get_args).collect();
        assert!(arguments.contains(&OsStr::new("clean")));
    }

    #[test]
    fn builds_the_counters_compiler_without_a_clean() {
        let package_names = WorkspaceFixture::workspace().package_names();
        let preparation = CompilerBuild::new(Path::new("/checkout"), Path::new("/checkout/target"))
            .with_counters_over_plain_build()
            .preparation(&package_names);
        assert!(preparation.is_none());
    }

    #[test]
    fn finds_no_compiler_outside_the_checkout() {
        let executable =
            CompilerBuild::new(Path::new("/elsewhere"), Path::new("/elsewhere/target"))
                .read_executable(MESSAGES);
        assert_eq!(executable, None);
    }
}
