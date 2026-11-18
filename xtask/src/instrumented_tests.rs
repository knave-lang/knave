use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::ValueEnum;

use crate::cargo::Cargo;
use crate::failure::TaskFailure;
use crate::lcov::LcovReport;
use crate::test_selection::TestRun;
use crate::test_shard::TestShard;
use crate::workspace::CargoWorkspace;

pub struct InstrumentedTests<'a> {
    root: &'a Path,
}

impl<'a> InstrumentedTests<'a> {
    const REPORT: &'static str = "target/coverage/lcov.info";
    const SHARD_REPORTS: &'static str = "target/coverage/shards";
    const UNMEASURED_PACKAGE: &'static str = "xtask";
    const FORGET_PREVIOUS_RUNS: [&'static str; 3] = ["llvm-cov", "clean", "--workspace"];
    const KEEP_BUILD_ARTIFACTS: &'static str = "--no-clean";
    const SELECTED_REPORT: &'static str = "target/coverage/selected.info";
    const FORGET_PREVIOUS_PROFILES: [&'static str; 3] = ["llvm-cov", "clean", "--profraw-only"];
    const RUN_WITHOUT_REPORT: [&'static str; 3] = ["llvm-cov", "test", "--no-report"];
    const WRITE_REPORT: [&'static str; 3] = ["llvm-cov", "report", "--lcov"];
    const PACKAGE: &'static str = "--package";
    const OUTPUT_PATH: &'static str = "--output-path";

    pub fn new(root: &'a Path) -> Self {
        Self { root }
    }

    pub fn run(&self) -> Result<LcovReport, TaskFailure> {
        let report = self.root.join(Self::REPORT);
        self.measure(&report, &[])?;
        self.parsed(&report)
    }

    pub fn run_selected(&self, runs: &[TestRun]) -> Result<LcovReport, TaskFailure> {
        if runs.is_empty() {
            return Ok(LcovReport::parse("", self.root));
        }
        let report = self.root.join(Self::SELECTED_REPORT);
        Self::create_parent(&report)?;
        let packages = CargoWorkspace::read(self.root)?.package_names();
        for arguments in Self::selected_commands(runs, &packages, &report) {
            self.cargo(|command| command.args(&arguments))?;
        }
        self.parsed(&report)
    }

    fn selected_commands(
        runs: &[TestRun],
        packages: &BTreeSet<String>,
        report: &Path,
    ) -> Vec<Vec<OsString>> {
        let forget = Self::os_strings(&Self::FORGET_PREVIOUS_PROFILES);
        let tests = runs.iter().map(|run| {
            let mut arguments = Self::os_strings(&Self::RUN_WITHOUT_REPORT);
            arguments.extend(run.arguments().iter().map(OsString::from));
            arguments
        });
        let mut write = Self::os_strings(&Self::WRITE_REPORT);
        for package in packages {
            write.extend([OsString::from(Self::PACKAGE), OsString::from(package)]);
        }
        write.extend([
            OsString::from(Self::OUTPUT_PATH),
            report.as_os_str().to_owned(),
        ]);
        std::iter::once(forget)
            .chain(tests)
            .chain([write])
            .collect()
    }

    fn os_strings(arguments: &[&str]) -> Vec<OsString> {
        arguments.iter().map(OsString::from).collect()
    }

    fn parsed(&self, report: &Path) -> Result<LcovReport, TaskFailure> {
        Ok(LcovReport::parse(&Self::read(report)?, self.root))
    }

    pub fn run_shard(&self, shard: TestShard) -> Result<(), TaskFailure> {
        self.save_shard_report(&self.shard_report(shard), |report| {
            self.measure(report, &shard.cargo_arguments())
        })
    }

    pub fn merge_shards(&self) -> Result<LcovReport, TaskFailure> {
        let texts = TestShard::value_variants()
            .iter()
            .map(|shard| Self::read(&self.shard_report(*shard)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(LcovReport::parse(&texts.concat(), self.root))
    }

    fn shard_report(&self, shard: TestShard) -> PathBuf {
        self.root
            .join(Self::SHARD_REPORTS)
            .join(shard.report_file())
    }

    fn save_shard_report(
        &self,
        report: &Path,
        measure: impl FnOnce(&Path) -> Result<(), TaskFailure>,
    ) -> Result<(), TaskFailure> {
        measure(report)?;
        self.write_relative_paths(report)
    }

    fn write_relative_paths(&self, report: &Path) -> Result<(), TaskFailure> {
        let text = LcovReport::with_relative_paths(&Self::read(report)?, self.root);
        fs::write(report, text).map_err(|error| TaskFailure::unwritable(report, error))
    }

    fn read(report: &Path) -> Result<String, TaskFailure> {
        fs::read_to_string(report).map_err(|error| TaskFailure::unreadable(report, error))
    }

    fn measure(&self, report: &Path, selection: &[&str]) -> Result<(), TaskFailure> {
        Self::create_parent(report)?;
        self.cargo(|command| command.args(Self::FORGET_PREVIOUS_RUNS))?;
        self.cargo(|command| Self::instrumented_run(command, report, selection))
    }

    fn create_parent(report: &Path) -> Result<(), TaskFailure> {
        let Some(directory) = report.parent() else {
            return Ok(());
        };
        fs::create_dir_all(directory).map_err(|error| TaskFailure::unwritable(directory, error))
    }

    fn instrumented_run<'command>(
        command: &'command mut Command,
        report: &Path,
        selection: &[&str],
    ) -> &'command mut Command {
        command
            .args(["llvm-cov", Self::KEEP_BUILD_ARTIFACTS, "--workspace"])
            .args(["--exclude-from-report", Self::UNMEASURED_PACKAGE])
            .args(["--lcov", "--output-path"])
            .arg(report)
            .args(selection)
    }

    fn cargo(
        &self,
        arguments: impl FnOnce(&mut Command) -> &mut Command,
    ) -> Result<(), TaskFailure> {
        let mut command = Cargo::command();
        command.current_dir(self.root);
        let output = arguments(&mut command)
            .output()
            .map_err(|error| TaskFailure::tool_missing("cargo llvm-cov", error))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(TaskFailure::tests_failed(&output))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::ffi::OsStr;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use super::InstrumentedTests;
    use crate::failure::TaskFailure;
    use crate::test_selection::TestRun;

    struct ShardCheckout {
        root: PathBuf,
    }

    impl ShardCheckout {
        const REGISTRY_FILE: &str = "/home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/serde-1.0.219/src/lib.rs";

        fn new(name: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("xtask-shard-{name}-{}", std::process::id()));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn report(&self) -> PathBuf {
            self.root.join("shard.info")
        }

        fn measured_text(&self) -> String {
            format!(
                "SF:{}/src/lib.rs\nDA:1,1\nend_of_record\nSF:{}\nDA:1,0\nend_of_record\n",
                self.root.display(),
                Self::REGISTRY_FILE
            )
        }

        fn write_measured_report(&self, report: &Path) -> Result<(), TaskFailure> {
            fs::write(report, self.measured_text()).unwrap();
            Ok(())
        }

        fn saved_text(&self) -> String {
            fs::read_to_string(self.report()).unwrap()
        }

        fn expected_text() -> String {
            format!(
                "SF:src/lib.rs\nDA:1,1\nend_of_record\nSF:{}\nDA:1,0\nend_of_record\n",
                Self::REGISTRY_FILE
            )
        }
    }

    impl Drop for ShardCheckout {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn writing_relative_paths_strips_only_the_shard_root() {
        let checkout = ShardCheckout::new("relative");
        checkout.write_measured_report(&checkout.report()).unwrap();
        InstrumentedTests::new(&checkout.root)
            .write_relative_paths(&checkout.report())
            .unwrap();
        assert_eq!(checkout.saved_text(), ShardCheckout::expected_text());
    }

    #[test]
    fn a_saved_shard_report_names_workspace_files_relative_to_the_shard() {
        let checkout = ShardCheckout::new("saved");
        InstrumentedTests::new(&checkout.root)
            .save_shard_report(&checkout.report(), |report| {
                checkout.write_measured_report(report)
            })
            .unwrap();
        assert_eq!(checkout.saved_text(), ShardCheckout::expected_text());
    }

    #[test]
    fn an_instrumented_run_ends_with_the_shard_selection() {
        let mut command = Command::new("cargo");
        let selection = ["--test", "stages"];
        InstrumentedTests::instrumented_run(&mut command, Path::new("report.info"), &selection);
        let arguments: Vec<_> = command.get_args().collect();
        assert_eq!(arguments[arguments.len() - selection.len()..], selection);
    }

    #[test]
    fn a_selected_run_forgets_old_profiles_runs_each_test_and_reports_each_package() {
        let runs = [
            TestRun::parse("--lib emit::"),
            TestRun::parse("--test stages emit_fixtures"),
        ];
        let packages = BTreeSet::from(["knave".to_string(), "knave-expand".to_string()]);
        let commands: Vec<String> =
            InstrumentedTests::selected_commands(&runs, &packages, Path::new("selected.info"))
                .iter()
                .map(|arguments| {
                    arguments
                        .join(OsStr::new(" "))
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();
        assert_eq!(
            commands,
            [
                "llvm-cov clean --profraw-only",
                "llvm-cov test --no-report --lib emit::",
                "llvm-cov test --no-report --test stages emit_fixtures",
                "llvm-cov report --lcov --package knave --package knave-expand --output-path selected.info"
            ]
        );
    }

    #[test]
    fn no_selected_runs_measure_nothing_and_run_no_cargo() {
        let tests = InstrumentedTests::new(Path::new("/nonexistent/workspace"));
        let report = tests.run_selected(&[]).expect("nothing runs");
        assert_eq!(report.total().percent(), 0.0);
    }

    #[test]
    fn merging_refuses_a_missing_shard_report() {
        let tests = InstrumentedTests::new(Path::new("/nonexistent/workspace"));
        assert!(tests.merge_shards().is_err());
    }
}
