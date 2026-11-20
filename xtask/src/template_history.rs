use std::fs;
use std::path::{Path, PathBuf};

use crate::checks::Check;
use crate::failure::TaskFailure;
use crate::git::Git;
use crate::release::ReleaseVersion;

pub struct TemplateHistory {
    templates: Vec<String>,
    comparison: Option<ReleaseComparison>,
}

struct ReleaseComparison {
    release: ReleaseVersion,
    forgotten: Vec<String>,
    altered: Vec<KeptText>,
}

struct KeptText {
    template: String,
    version: ReleaseVersion,
    text: String,
}

impl TemplateHistory {
    pub const HISTORY_ROOT: &str = "src/cli/earlier_texts";
    const LATEST_REACHABLE_RELEASE: [&str; 5] =
        ["describe", "--tags", "--match", "v*", "--abbrev=0"];

    pub fn check(root: &Path) -> Result<Self, TaskFailure> {
        let templates = Self::templates(&root.join(Self::HISTORY_ROOT))?;
        let comparison = Self::latest_release(root)?
            .map(|release| ReleaseComparison::of(root, release, &templates))
            .transpose()?;
        Ok(Self {
            templates,
            comparison,
        })
    }

    fn latest_release(root: &Path) -> Result<Option<ReleaseVersion>, TaskFailure> {
        let output = Git::new(root).run(&Self::LATEST_REACHABLE_RELEASE)?;
        output
            .status
            .success()
            .then(|| ReleaseVersion::parse_tag(String::from_utf8_lossy(&output.stdout).trim()))
            .transpose()
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

    fn text_at(root: &Path, revision: &str, template: &str) -> Result<Option<String>, TaskFailure> {
        let output = Git::new(root).run(&["show", &format!("{revision}:{template}")])?;
        Ok(output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).into_owned()))
    }

    fn kept_texts(root: &Path, template: &str) -> Result<Vec<KeptText>, TaskFailure> {
        let folder = root.join(Self::HISTORY_ROOT).join(template);
        let unreadable = |error| TaskFailure::unreadable(&folder, error);
        fs::read_dir(&folder)
            .map_err(unreadable)?
            .map(|entry| KeptText::read(template, &entry.map_err(unreadable)?.path()))
            .collect()
    }
}

impl ReleaseComparison {
    fn of(root: &Path, release: ReleaseVersion, templates: &[String]) -> Result<Self, TaskFailure> {
        let mut comparison = Self {
            release,
            forgotten: Vec::new(),
            altered: Vec::new(),
        };
        for template in templates {
            if comparison.forgets_its_released_text(root, template)? {
                comparison.forgotten.push(template.clone());
            }
            comparison
                .altered
                .extend(Self::altered_texts(root, template)?);
        }
        Ok(comparison)
    }

    fn forgets_its_released_text(&self, root: &Path, template: &str) -> Result<bool, TaskFailure> {
        let Some(earlier) = TemplateHistory::text_at(root, &self.release.tag(), template)? else {
            return Ok(false);
        };
        let path = root.join(template);
        let current =
            fs::read_to_string(&path).map_err(|error| TaskFailure::unreadable(&path, error))?;
        if current == earlier {
            return Ok(false);
        }
        let kept = TemplateHistory::kept_texts(root, template)?;
        Ok(!kept.iter().any(|kept| kept.text == earlier))
    }

    fn altered_texts(root: &Path, template: &str) -> Result<Vec<KeptText>, TaskFailure> {
        let mut altered = Vec::new();
        for kept in TemplateHistory::kept_texts(root, template)? {
            if !kept.matches_its_release(root)? {
                altered.push(kept);
            }
        }
        Ok(altered)
    }

    fn passes(&self) -> bool {
        self.forgotten.is_empty() && self.altered.is_empty()
    }

    fn details(&self) -> String {
        let forgotten = self
            .forgotten
            .iter()
            .map(|template| self.forgotten_line(template));
        let altered = self.altered.iter().map(KeptText::mismatch_line);
        forgotten.chain(altered).collect()
    }

    fn forgotten_line(&self, template: &str) -> String {
        format!(
            "  {template} differs from its text at {}, and that text is missing: copy it to `{}/{template}/{}`\n",
            self.release.tag(),
            TemplateHistory::HISTORY_ROOT,
            self.release
        )
    }
}

impl KeptText {
    fn read(template: &str, path: &Path) -> Result<Self, TaskFailure> {
        let text =
            fs::read_to_string(path).map_err(|error| TaskFailure::unreadable(path, error))?;
        let version = ReleaseVersion::parse(
            &path
                .file_name()
                .expect("a kept text has a file name")
                .to_string_lossy(),
        )?;
        Ok(Self {
            template: template.to_string(),
            version,
            text,
        })
    }

    fn matches_its_release(&self, root: &Path) -> Result<bool, TaskFailure> {
        let released = TemplateHistory::text_at(root, &self.release(), &self.template)?;
        Ok(released.as_deref() == Some(self.text.as_str()))
    }

    fn release(&self) -> String {
        self.version.tag()
    }

    fn mismatch_line(&self) -> String {
        format!(
            "  `{}/{}/{}` differs from the text {} released: replace it with the output of `git show {}:{}`\n",
            TemplateHistory::HISTORY_ROOT,
            self.template,
            self.version,
            self.release(),
            self.release(),
            self.template
        )
    }
}

impl Check for TemplateHistory {
    fn passes(&self) -> bool {
        self.comparison
            .as_ref()
            .is_some_and(ReleaseComparison::passes)
    }

    fn summary(&self) -> String {
        format!(
            "{} owned templates keep each earlier text under `{}`",
            self.templates.len(),
            Self::HISTORY_ROOT
        )
    }

    fn details(&self) -> String {
        match &self.comparison {
            Some(comparison) => comparison.details(),
            None => "  no `v*` tag is reachable from HEAD: run `git fetch --tags`\n".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TemplateHistory;
    use crate::checks::Check;
    use crate::temporary_repository::TemporaryRepository;

    const WORKFLOW: &str = "src/cli/github_workflow.yml";
    const FIRST_WORKFLOW: &str = "steps: []\n";
    const EARLIER_WORKFLOW: &str = "steps:\n  - run: knave test\n";
    const CHANGED_WORKFLOW: &str = "steps:\n  - run: knave lint\n";
    const KEPT_AT_FIRST_RELEASE: &str = "src/cli/earlier_texts/src/cli/github_workflow.yml/0.17.0";
    const KEPT_AT_LATEST_RELEASE: &str = "src/cli/earlier_texts/src/cli/github_workflow.yml/0.18.0";

    struct HistoryCase {
        repository: TemporaryRepository,
    }

    impl HistoryCase {
        fn released(name: &str) -> Self {
            let repository = TemporaryRepository::initialized(name);
            repository.write(WORKFLOW, FIRST_WORKFLOW);
            repository.commit_all();
            repository.git(&["tag", "v0.17.0"]);
            repository.write(WORKFLOW, EARLIER_WORKFLOW);
            repository.write(KEPT_AT_FIRST_RELEASE, FIRST_WORKFLOW);
            repository.commit_all();
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
        case.repository.write(WORKFLOW, CHANGED_WORKFLOW);
        let history = case.checked();
        assert!(!history.passes());
        assert_eq!(
            history.details(),
            format!(
                "  {WORKFLOW} differs from its text at v0.18.0, and that text is missing: copy it to `{KEPT_AT_LATEST_RELEASE}`\n"
            )
        );
    }

    #[test]
    fn a_template_changed_and_committed_after_the_release_still_needs_its_released_text() {
        let case = HistoryCase::released("history-after-release");
        case.repository.write(WORKFLOW, CHANGED_WORKFLOW);
        case.repository.commit_all();
        assert!(!case.checked().passes());
    }

    #[test]
    fn a_template_changed_between_releases_needs_no_entry_for_its_unreleased_text() {
        let case = HistoryCase::released("history-between-releases");
        case.repository
            .write(KEPT_AT_LATEST_RELEASE, EARLIER_WORKFLOW);
        case.repository.write(WORKFLOW, CHANGED_WORKFLOW);
        case.repository.commit_all();
        case.repository
            .write(WORKFLOW, "steps:\n  - run: knave fmt --check\n");
        assert!(case.checked().passes());
    }

    #[test]
    fn a_release_unreachable_from_head_is_not_compared() {
        let case = HistoryCase::released("history-unreachable");
        case.repository.git(&["checkout", "--quiet", "-b", "later"]);
        case.repository.write(WORKFLOW, CHANGED_WORKFLOW);
        case.repository.commit_all();
        case.repository.git(&["tag", "v0.19.0"]);
        case.repository.git(&["checkout", "--quiet", "-"]);
        assert!(case.checked().passes());
    }

    #[test]
    fn a_repository_without_a_release_fails_and_says_to_fetch_the_tags() {
        let repository = TemporaryRepository::initialized("history-unreleased");
        repository.write(WORKFLOW, EARLIER_WORKFLOW);
        repository.write(KEPT_AT_FIRST_RELEASE, FIRST_WORKFLOW);
        repository.commit_all();
        let history = TemplateHistory::check(repository.root()).expect("the history reads");
        assert!(!history.passes());
        assert_eq!(
            history.details(),
            "  no `v*` tag is reachable from HEAD: run `git fetch --tags`\n"
        );
    }

    #[test]
    fn a_changed_template_that_keeps_its_earlier_text_passes() {
        let case = HistoryCase::released("history-kept");
        case.repository.write(WORKFLOW, CHANGED_WORKFLOW);
        case.repository
            .write(KEPT_AT_LATEST_RELEASE, EARLIER_WORKFLOW);
        assert!(case.checked().passes());
    }

    #[test]
    fn a_kept_text_that_differs_from_its_release_fails() {
        let case = HistoryCase::released("history-altered");
        case.repository
            .write(KEPT_AT_FIRST_RELEASE, "steps: [edited]\n");
        let history = case.checked();
        assert!(!history.passes());
        assert_eq!(
            history.details(),
            format!(
                "  `{KEPT_AT_FIRST_RELEASE}` differs from the text v0.17.0 released: replace it with the output of `git show v0.17.0:{WORKFLOW}`\n"
            )
        );
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
    fn a_template_missing_from_the_latest_release_needs_no_kept_text_of_it() {
        let case = HistoryCase::released("history-missing");
        case.repository.remove(WORKFLOW);
        case.repository.commit_all();
        case.repository.git(&["tag", "v0.18.1"]);
        case.repository.write(WORKFLOW, CHANGED_WORKFLOW);
        assert!(case.checked().passes());
    }
}
