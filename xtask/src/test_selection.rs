use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TestRun {
    arguments: Vec<String>,
}

impl TestRun {
    pub fn new(arguments: &[&str]) -> Self {
        Self {
            arguments: arguments
                .iter()
                .map(|argument| argument.to_string())
                .collect(),
        }
    }

    pub fn parse(text: &str) -> Self {
        Self::new(&text.split_whitespace().collect::<Vec<_>>())
    }

    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn runs_stage_fixtures(&self) -> bool {
        self.arguments
            .starts_with(&StageTests::STAGES_TARGET.map(String::from))
    }
}

impl fmt::Display for TestRun {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.arguments.join(" "))
    }
}

struct StageTests {
    prefix: &'static str,
    fixtures: &'static str,
    broader: Option<&'static [&'static str]>,
}

impl StageTests {
    const STAGES_TARGET: [&'static str; 2] = ["--test", "stages"];

    fn run(&self) -> TestRun {
        TestRun::new(&[&Self::STAGES_TARGET[..], &[self.fixtures]].concat())
    }
}

pub struct TestSelection;

impl TestSelection {
    const SOURCE_DIRECTORY: &str = "src/";
    const RUST_EXTENSION: &str = ".rs";
    const MODULE_FILE: &str = "/mod";
    const CRATE_ROOTS: [&str; 2] = ["lib", "main"];
    const MANIFEST: &str = "Cargo.toml";
    const STAGES: [StageTests; 9] = [
        StageTests {
            prefix: "expand/src/reader/",
            fixtures: "reader_fixtures",
            broader: Some(&["--test", "stages", "expand_fixtures"]),
        },
        StageTests {
            prefix: "expand/src/project/",
            fixtures: "project_fixtures",
            broader: Some(&["--test", "stages", "run_fixtures"]),
        },
        StageTests {
            prefix: "expand/src/format/",
            fixtures: "format_fixtures",
            broader: Some(&["--test", "cli", "fmt"]),
        },
        StageTests {
            prefix: "expand/src/",
            fixtures: "expand_fixtures",
            broader: Some(&["--test", "stages", "check_fixtures"]),
        },
        StageTests {
            prefix: "src/lint/",
            fixtures: "lint_fixtures",
            broader: Some(&["--test", "cli", "lint"]),
        },
        StageTests {
            prefix: "src/emit/",
            fixtures: "emit_fixtures",
            broader: Some(&["--test", "stages", "run_fixtures"]),
        },
        StageTests {
            prefix: "src/build/",
            fixtures: "run_fixtures",
            broader: None,
        },
        StageTests {
            prefix: "src/cli/",
            fixtures: "run_fixtures",
            broader: None,
        },
        StageTests {
            prefix: "src/",
            fixtures: "check_fixtures",
            broader: Some(&["--test", "stages", "emit_fixtures"]),
        },
    ];

    pub fn runs<'a>(files: impl Iterator<Item = &'a str>) -> BTreeSet<TestRun> {
        files
            .flat_map(|file| {
                Self::stage_run(file)
                    .into_iter()
                    .chain(Self::unit_run(file))
            })
            .collect()
    }

    pub fn broader_run(file: &str) -> Option<TestRun> {
        Self::stage(file)?.broader.map(TestRun::new)
    }

    fn stage_run(file: &str) -> Option<TestRun> {
        Self::stage(file).map(StageTests::run)
    }

    fn stage(file: &str) -> Option<&'static StageTests> {
        Self::STAGES
            .iter()
            .find(|stage| file.starts_with(stage.prefix))
    }

    fn unit_run(file: &str) -> Option<TestRun> {
        let (crate_directory, module_file) = file.split_once(Self::SOURCE_DIRECTORY)?;
        let module = Self::module_path(module_file)?;
        let manifest = format!("{crate_directory}{}", Self::MANIFEST);
        Some(TestRun::new(&[
            "--manifest-path",
            &manifest,
            "--lib",
            &module,
        ]))
    }

    fn module_path(module_file: &str) -> Option<String> {
        let module = module_file.strip_suffix(Self::RUST_EXTENSION)?;
        let module = module.strip_suffix(Self::MODULE_FILE).unwrap_or(module);
        let is_crate_root = Self::CRATE_ROOTS.contains(&module);
        (!is_crate_root).then(|| module.replace('/', "::"))
    }
}

#[cfg(test)]
mod tests {
    use super::{TestRun, TestSelection};

    struct SelectionCase;

    impl SelectionCase {
        fn runs(files: &[&str]) -> Vec<String> {
            TestSelection::runs(files.iter().copied())
                .iter()
                .map(TestRun::to_string)
                .collect()
        }
    }

    #[test]
    fn a_stage_file_runs_its_stage_fixtures_and_its_unit_tests() {
        assert_eq!(
            SelectionCase::runs(&["src/types/checker.rs"]),
            [
                "--manifest-path Cargo.toml --lib types::checker",
                "--test stages check_fixtures"
            ]
        );
    }

    #[test]
    fn an_expand_file_runs_the_expand_crate_unit_tests() {
        assert_eq!(
            SelectionCase::runs(&["expand/src/reader/mod.rs"]),
            [
                "--manifest-path expand/Cargo.toml --lib reader",
                "--test stages reader_fixtures"
            ]
        );
    }

    #[test]
    fn the_first_matching_stage_wins() {
        assert_eq!(
            SelectionCase::runs(&["src/emit/a.rs", "src/cli/b.rs"]),
            [
                "--manifest-path Cargo.toml --lib cli::b",
                "--manifest-path Cargo.toml --lib emit::a",
                "--test stages emit_fixtures",
                "--test stages run_fixtures"
            ]
        );
    }

    #[test]
    fn a_crate_root_runs_no_unit_tests_by_module() {
        assert_eq!(
            SelectionCase::runs(&["src/lib.rs", "src/main.rs"]),
            ["--test stages check_fixtures"]
        );
    }

    #[test]
    fn a_rust_file_outside_a_source_directory_runs_nothing() {
        assert!(SelectionCase::runs(&["build.rs"]).is_empty());
    }

    #[test]
    fn a_broader_run_follows_a_stage() {
        let broader = TestSelection::broader_run("expand/src/expand/mod.rs");
        assert_eq!(
            broader.map(|run| run.to_string()),
            Some("--test stages check_fixtures".to_string())
        );
    }

    #[test]
    fn the_last_stage_has_no_broader_run() {
        assert_eq!(TestSelection::broader_run("src/cli/main.rs"), None);
    }

    #[test]
    fn a_parsed_run_splits_on_whitespace() {
        assert_eq!(
            TestRun::parse(" --test  cli fmt").arguments(),
            ["--test", "cli", "fmt"]
        );
    }
}
