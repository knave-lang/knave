use std::path::Path;

use super::new_file::NewFile;
use super::project_component::ProjectComponent;
use super::versioned_text::VersionedText;

pub struct AgentSkill;

impl AgentSkill {
    const NAME: &str = "writing-knave";
    const TEXT: VersionedText =
        VersionedText::new(include_str!("../../skills/writing-knave/SKILL.md"));
    const FILE: &str = "SKILL.md";
    const SKILL_FOLDERS: [&str; 2] = [".agents/skills", ".claude/skills"];
}

impl ProjectComponent for AgentSkill {
    fn files(&self, root: &Path) -> Vec<NewFile> {
        let text = Self::TEXT.filled();
        Self::SKILL_FOLDERS
            .iter()
            .map(|folder| {
                NewFile::new(
                    root.join(folder).join(Self::NAME).join(Self::FILE),
                    text.clone(),
                )
            })
            .collect()
    }
}
