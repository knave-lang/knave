use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::baseline_text::BaselineText;

pub struct MergeCommand {
    base: PathBuf,
    ours: PathBuf,
    theirs: PathBuf,
    file: String,
}

impl MergeCommand {
    const REFUSED: u8 = 2;
    const USAGE: &str =
        "usage: merge-baseline <base> <ours> <theirs> <file>, as git names them %O %A %B %P";
    const LEFT_UNMERGED: &str =
        "Git keeps our side with no conflict markers, so merge the file by hand before `git add`.";

    const PREFIX: &str = "merge-baseline: ";

    pub fn run(arguments: impl IntoIterator<Item = String>) -> ExitCode {
        let (report, exit_code) =
            Self::reported(Self::parse(arguments).and_then(|command| command.merge()));
        for line in report {
            eprintln!("{}{line}", Self::PREFIX);
        }
        exit_code
    }

    fn reported(outcome: Result<Vec<String>, String>) -> (Vec<String>, ExitCode) {
        match outcome {
            Ok(sums) => (sums, ExitCode::SUCCESS),
            Err(message) => (
                vec![format!("{message}. {}", Self::LEFT_UNMERGED)],
                ExitCode::from(Self::REFUSED),
            ),
        }
    }

    fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let arguments: Vec<String> = arguments.into_iter().collect();
        match <[String; 4]>::try_from(arguments) {
            Ok([base, ours, theirs, file]) => Ok(Self {
                base: base.into(),
                ours: ours.into(),
                theirs: theirs.into(),
                file,
            }),
            Err(_) => Err(Self::USAGE.to_string()),
        }
    }

    fn merge(&self) -> Result<Vec<String>, String> {
        let base = self.read(&self.base, Side::Base)?;
        let ours = self.read(&self.ours, Side::Ours)?;
        let theirs = self.read(&self.theirs, Side::Theirs)?;
        let merge = base.merged(&ours, &theirs);
        fs::write(&self.ours, merge.text.text())
            .map_err(|error| format!("cannot write `{}`: {error}", self.ours.display()))?;
        Ok(merge
            .sums
            .iter()
            .map(|sum| format!("{}: {sum}", self.file))
            .collect())
    }

    fn read(&self, path: &Path, side: Side) -> Result<BaselineText, String> {
        let name = format!("{} ({side})", self.file);
        let text =
            fs::read_to_string(path).map_err(|error| format!("cannot read {name}: {error}"))?;
        BaselineText::parse(&name, &text).map_err(|error| error.to_string())
    }
}

#[derive(Clone, Copy)]
enum Side {
    Base,
    Ours,
    Theirs,
}

impl fmt::Display for Side {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Side::Base => "base",
            Side::Ours => "ours",
            Side::Theirs => "theirs",
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::process::ExitCode;

    use super::{MergeCommand, Side};

    struct MergeDirectory {
        root: PathBuf,
    }

    impl MergeDirectory {
        fn new(name: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("merge-baseline-{name}-{}", std::process::id()));
            fs::create_dir_all(&root).expect("the directory is created");
            Self { root }
        }

        fn run(&self, base: &str, ours: &str, theirs: &str) -> ExitCode {
            let [base, ours, theirs] =
                [("base", base), ("ours", ours), ("theirs", theirs)].map(|(name, text)| {
                    let path = self.root.join(name);
                    fs::write(&path, text).expect("the file is written");
                    path.display().to_string()
                });
            MergeCommand::run([base, ours, theirs, "baseline.txt".to_string()])
        }

        fn merge(&self, base: &str, ours: &str, theirs: &str) -> Result<Vec<String>, String> {
            let [base, ours, theirs] =
                [("base", base), ("ours", ours), ("theirs", theirs)].map(|(name, text)| {
                    let path = self.root.join(name);
                    fs::write(&path, text).expect("the file is written");
                    path
                });
            let file = "baseline.txt".to_string();
            MergeCommand {
                base,
                ours,
                theirs,
                file,
            }
            .merge()
        }

        fn command(&self) -> MergeCommand {
            MergeCommand {
                base: self.root.join("absent"),
                ours: self.root.join("ours"),
                theirs: self.root.join("theirs"),
                file: "baseline.txt".to_string(),
            }
        }

        fn read(&self, name: &str) -> String {
            fs::read_to_string(self.root.join(name)).expect("the file reads")
        }
    }

    impl Drop for MergeDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn a_merge_writes_over_ours_and_succeeds() {
        let directory = MergeDirectory::new("written");
        assert_eq!(directory.run("a 383", "a 380", "a 381"), ExitCode::SUCCESS);
        assert_eq!(directory.read("ours"), "a 378\n");
        assert_eq!(directory.read("theirs"), "a 381");
    }

    #[test]
    fn a_malformed_input_is_refused_and_leaves_ours_alone() {
        let directory = MergeDirectory::new("malformed");
        let refused = ExitCode::from(MergeCommand::REFUSED);
        assert_eq!(directory.run("a 383", "a 380", "<<<<<<< HEAD"), refused);
        assert_eq!(directory.read("ours"), "a 380");
    }

    #[test]
    fn a_missing_input_is_refused() {
        let directory = MergeDirectory::new("missing");
        let paths = ["absent", "ours", "theirs", "file"]
            .map(|name| directory.root.join(name).display().to_string());
        assert_eq!(
            MergeCommand::run(paths),
            ExitCode::from(MergeCommand::REFUSED)
        );
    }

    #[test]
    fn a_missing_input_is_named_by_its_file_and_side() {
        let directory = MergeDirectory::new("named");
        let command = directory.command();
        let failure = command
            .read(&command.base, Side::Base)
            .err()
            .unwrap_or_default();
        assert!(
            failure.starts_with("cannot read baseline.txt (base): "),
            "{failure}"
        );
    }

    #[test]
    fn a_merge_reports_each_sum_with_its_file() {
        let directory = MergeDirectory::new("sums");
        let report = directory.merge("a 49\n\nb 5", "a 48\n\nb 5", "a 48\n\nb 4");
        assert_eq!(
            report,
            Ok(vec!["baseline.txt: a: 48 + 48 - 49 = 47".to_string()])
        );
    }

    #[test]
    fn a_malformed_side_is_named() {
        let directory = MergeDirectory::new("sides");
        for (base, ours, theirs, side) in [
            ("x", "a 1", "a 1", "(base)"),
            ("a 1", "x", "a 1", "(ours)"),
            ("a 1", "a 1", "x", "(theirs)"),
        ] {
            let refusal = directory
                .merge(base, ours, theirs)
                .err()
                .unwrap_or_default();
            assert!(
                refusal.starts_with(&format!("baseline.txt {side} has a malformed line")),
                "{refusal}"
            );
        }
    }

    #[test]
    fn a_refusal_warns_that_git_keeps_our_side_unmarked() {
        let (report, exit_code) = MergeCommand::reported(Err("bad".to_string()));
        assert_eq!(report, [format!("bad. {}", MergeCommand::LEFT_UNMERGED)]);
        assert_eq!(exit_code, ExitCode::from(MergeCommand::REFUSED));
    }

    #[test]
    fn a_merge_reports_its_sums_and_succeeds() {
        let (report, exit_code) = MergeCommand::reported(Ok(vec!["a sum".to_string()]));
        assert_eq!(
            (report, exit_code),
            (vec!["a sum".to_string()], ExitCode::SUCCESS)
        );
    }

    #[test]
    fn the_flake_driver_stops_at_a_refusal_instead_of_falling_back() {
        let flake = include_str!("../../../flake.nix");
        let stop = format!("[ $status -le {} ] && exit $status", MergeCommand::REFUSED);
        assert!(flake.contains(&stop), "flake.nix lacks `{stop}`");
    }

    #[test]
    fn three_paths_are_refused_with_the_usage() {
        let refusal = MergeCommand::parse(["base", "ours", "theirs"].map(String::from)).err();
        assert_eq!(refusal.as_deref(), Some(MergeCommand::USAGE));
    }
}
