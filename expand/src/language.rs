pub struct Language;

impl Language {
    pub const NAME: &str = "knave";
    pub const EXTENSION: &str = "knv";
    pub const EDITION: i128 = 2026;
    pub const PROJECT_FILE_STEM: &str = "project";

    pub fn project_file_name() -> String {
        Self::file_name(Self::PROJECT_FILE_STEM)
    }

    pub fn file_name(stem: &str) -> String {
        format!("{stem}.{}", Self::EXTENSION)
    }
}
