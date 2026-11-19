use std::fs;
use std::path::{Path, PathBuf};

use crate::checks::Check;
use crate::failure::TaskFailure;
use crate::git::Git;

pub struct TemplateHistory {
    templates: Vec<String>,
    forgotten: Vec<String>,
    release: Option<String>,
}

impl TemplateHistory {
    pub const HISTORY_ROOT: &str = "src/cli/earlier_texts";
    const RELEASE_TAGS: [&str; 4] = ["tag", "--list", "v*", "--sort=-v:refname"];

    pub fn check(root: &Path) -> Result<Self, TaskFailure> {
        let templates = Self::templates(&root.join(Self::HISTORY_ROOT))?;
        let release = Self::latest_release(root)?;
        let mut forgotten = Vec::new();
        if let Some(release) = &release {
            for template in &templates {
                if Self::forgets_its_released_text(root, release, template)? {
                    forgotten.push(template.clone());
                }
            }
        }
        Ok(Self {
            templates,
            forgotten,
            release,
        })
    }

    fn latest_release(root: &Path) -> Result<Option<String>, TaskFailure> {
        let output = Git::new(root).run(&Self::RELEASE_TAGS)?;
        if !output.status.success() {
            return Err(TaskFailure::tool_failed(Git::PROGRAM, &output.stderr));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .map(str::to_string))
    }

    fn templates(history_root: &Path) -> Result<Vec<String>, TaskFailure> {
        let mut templates = Vec::new();
        let mut pending = vec![history_root.to_path_buf()];
        while let Some(folder) = pending.pop() {
            let (folders, holds_texts) = Self::entries(&folder)?;
            pending.extend(folders);
            if holds_texts {
                templates.push(Self::template_of(history_root, &folder));
            }
        }
        templates.sort();
        Ok(templates)
    }

    fn entries(folder: &Path) -> Result<(Vec<PathBuf>, bool), TaskFailure> {
        let unreadable = |error| TaskFailure::unreadable(folder, error);
        let mut folders = Vec::new();
        let mut holds_texts = false;
        for entry in fs::read_dir(folder).map_err(unreadable)? {
            let path = entry.map_err(unreadable)?.path();
            if path.is_dir() {
                folders.push(path);
            } else {
                holds_texts = true;
            }
        }
        Ok((folders, holds_texts))
    }

    fn template_of(history_root: &Path, folder: &Path) -> String {
        folder
            .strip_prefix(history_root)
            .expect("a history folder lies under the history root")
            .display()
            .to_string()
    }

    fn forgets_its_released_text(
        root: &Path,
        release: &str,
        template: &str,
    ) -> Result<bool, TaskFailure> {
        let Some(earlier) = Self::text_at(root, release, template)? else {
            return Ok(false);
        };
        let path = root.join(template);
        let current =
            fs::read_to_string(&path).map_err(|error| TaskFailure::unreadable(&path, error))?;
        if current == earlier {
            return Ok(false);
        }
        Ok(!Self::kept_texts(root, template)?.contains(&earlier))
    }

    fn text_at(root: &Path, revision: &str, template: &str) -> Result<Option<String>, TaskFailure> {
        let output = Git::new(root).run(&["show", &format!("{revision}:{template}")])?;
        Ok(output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).into_owned()))
    }

    fn kept_texts(root: &Path, template: &str) -> Result<Vec<String>, TaskFailure> {
        let folder = root.join(Self::HISTORY_ROOT).join(template);
        let unreadable = |error| TaskFailure::unreadable(&folder, error);
        fs::read_dir(&folder)
            .map_err(unreadable)?
            .map(|entry| {
                let path = entry.map_err(unreadable)?.path();
                fs::read_to_string(&path).map_err(|error| TaskFailure::unreadable(&path, error))
            })
            .collect()
    }
}

impl Check for TemplateHistory {
    fn passes(&self) -> bool {
        self.forgotten.is_empty()
    }

    fn summary(&self) -> String {
        format!(
            "{} owned templates keep each earlier text under `{}`",
            self.templates.len(),
            Self::HISTORY_ROOT
        )
    }

    fn details(&self) -> String {
        self.forgotten
            .iter()
            .map(|template| {
                let release = self.release.as_deref().unwrap_or_default();
                format!(
                    "  {template} differs from its text at {release}, and that text is missing: copy it to `{}/{template}/{}`\n",
                    Self::HISTORY_ROOT,
                    release.trim_start_matches('v')
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::TemplateHistory;
    use crate::checks::Check;
    use crate::temporary_repository::TemporaryRepository;

    const WORKFLOW: &str = "src/cli/github_workflow.yml";
    const EARLIER_WORKFLOW: &str = "steps:\n  - run: knave test\n";
    const KEPT_WORKFLOW: &str = "src/cli/earlier_texts/src/cli/github_workflow.yml/0.18.0";

    struct HistoryCase {
        repository: TemporaryRepository,
    }

    impl HistoryCase {
        fn released(name: &str) -> Self {
            let repository = TemporaryRepository::initialized(name);
            repository.write(WORKFLOW, EARLIER_WORKFLOW);
            repository.write(KEPT_WORKFLOW, "steps: []\n");
            repository.commit_all();
            repository.git(&["tag", "v0.9.0"]);
            repository.git(&["tag", "v0.18.0"]);
            Self { repository }
        }

        fn checked(&self) -> TemplateHistory {
            TemplateHistory::check(self.repository.root()).expect("the history reads")
        }
    }

    #[test]
    fn a_changed_template_without_its_earlier_text_fails() {
        let case = HistoryCase::released("history-forgotten");
        case.repository
            .write(WORKFLOW, "steps:\n  - run: knave lint\n");
        let history = case.checked();
        assert!(!history.passes());
        assert_eq!(
            history.details(),
            format!(
                "  {WORKFLOW} differs from its text at v0.18.0, and that text is missing: copy it to `src/cli/earlier_texts/{WORKFLOW}/0.18.0`\n"
            )
        );
    }

    #[test]
    fn a_template_changed_and_committed_after_the_release_still_needs_its_released_text() {
        let case = HistoryCase::released("history-after-release");
        case.repository
            .write(WORKFLOW, "steps:\n  - run: knave lint\n");
        case.repository.commit_all();
        assert!(!case.checked().passes());
    }

    #[test]
    fn a_template_changed_between_releases_needs_no_entry_for_its_unreleased_text() {
        let case = HistoryCase::released("history-between-releases");
        case.repository.write(
            "src/cli/earlier_texts/src/cli/github_workflow.yml/0.18.0",
            EARLIER_WORKFLOW,
        );
        case.repository
            .write(WORKFLOW, "steps:\n  - run: knave lint\n");
        case.repository.commit_all();
        case.repository
            .write(WORKFLOW, "steps:\n  - run: knave fmt --check\n");
        assert!(case.checked().passes());
    }

    #[test]
    fn a_repository_without_a_release_passes() {
        let repository = TemporaryRepository::initialized("history-unreleased");
        repository.write(WORKFLOW, EARLIER_WORKFLOW);
        repository.write(KEPT_WORKFLOW, "steps: []\n");
        repository.commit_all();
        repository.write(WORKFLOW, "steps:\n  - run: knave lint\n");
        let history = TemplateHistory::check(repository.root()).expect("the history reads");
        assert!(history.passes());
    }

    #[test]
    fn a_changed_template_that_keeps_its_earlier_text_passes() {
        let case = HistoryCase::released("history-kept");
        case.repository
            .write(WORKFLOW, "steps:\n  - run: knave lint\n");
        case.repository.write(
            "src/cli/earlier_texts/src/cli/github_workflow.yml/0.18.0",
            EARLIER_WORKFLOW,
        );
        assert!(case.checked().passes());
    }

    #[test]
    fn an_unchanged_template_passes() {
        let history = HistoryCase::released("history-unchanged").checked();
        assert!(history.passes());
        assert_eq!(
            history.summary(),
            "1 owned templates keep each earlier text under `src/cli/earlier_texts`"
        );
    }

    #[test]
    fn a_template_new_since_the_release_passes() {
        let case = HistoryCase::released("history-new");
        case.repository.write("skills/fresh/SKILL.md", "fresh\n");
        case.repository.write(
            "src/cli/earlier_texts/skills/fresh/SKILL.md/0.18.0",
            "older\n",
        );
        assert!(case.checked().passes());
    }
}
