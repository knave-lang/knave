use std::path::Path;

pub struct ModuleNames;

impl ModuleNames {
    const SEPARATOR: &'static str = "/";

    pub fn of_source(relative: &Path) -> Option<String> {
        Self::of_folder(&relative.with_extension(""))
    }

    pub fn of_folder(relative: &Path) -> Option<String> {
        let parts: Vec<&str> = relative
            .components()
            .map(|component| component.as_os_str().to_str())
            .collect::<Option<_>>()?;
        Some(parts.join(Self::SEPARATOR))
    }
}
