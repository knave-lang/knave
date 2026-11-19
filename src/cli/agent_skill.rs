use std::path::Path;

use super::owned_file::OwnedFile;
use super::project_component::ProjectComponent;
use super::released_text::{ReleasedText, released_text};
use super::versioned_text::VersionedText;

pub struct AgentSkill;

impl AgentSkill {
    const NAME: &str = "writing-knave";
    const TEXT: VersionedText =
        VersionedText::new(include_str!("../../skills/writing-knave/SKILL.md"));
    const EARLIER_TEXTS: &[ReleasedText] = &[
        released_text!("skills/writing-knave/SKILL.md", "0.18.0"),
        released_text!("skills/writing-knave/SKILL.md", "0.19.0"),
    ];
    const FILE: &str = "SKILL.md";
    const SKILL_FOLDERS: [&str; 2] = [".agents/skills", ".claude/skills"];
}

impl ProjectComponent for AgentSkill {
    fn owned_files(&self) -> Vec<OwnedFile> {
        Self::SKILL_FOLDERS
            .iter()
            .map(|folder| {
                let path = Path::new(folder).join(Self::NAME).join(Self::FILE);
                OwnedFile::new(path, Self::TEXT, Self::EARLIER_TEXTS)
            })
            .collect()
    }
}
