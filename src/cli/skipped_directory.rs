use std::path::Path;

use super::terminal::Terminal;

pub struct SkippedDirectory;

impl SkippedDirectory {
    const HIDDEN_PREFIX: char = '.';

    pub fn is_skipped(directory: &Path) -> bool {
        directory
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.starts_with(Self::HIDDEN_PREFIX) || name == Terminal::BUILD_PARENT
            })
    }
}
