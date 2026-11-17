use std::fmt;
use std::path::Path;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ModulePath(Rc<str>);

impl ModulePath {
    pub const SEPARATOR: &str = "/";

    pub fn new(path: impl Into<Rc<str>>) -> Self {
        Self(path.into())
    }

    pub fn of_file(relative: &Path) -> Option<Self> {
        Self::of_folder(&relative.with_extension(""))
    }

    pub fn of_folder(relative: &Path) -> Option<Self> {
        let names: Vec<&str> = relative
            .components()
            .map(|component| component.as_os_str().to_str())
            .collect::<Option<_>>()?;
        Some(Self::new(names.join(Self::SEPARATOR)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ModulePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    use super::ModulePath;

    #[test]
    fn a_file_is_its_path_without_the_extension() {
        let file = Path::new("text/marks.knv");
        assert_eq!(
            ModulePath::of_file(file),
            Some(ModulePath::new("text/marks"))
        );
    }

    #[test]
    fn a_file_whose_name_is_not_text_has_no_module_path() {
        let name = OsStr::from_bytes(b"marks\xff.knv");
        assert_eq!(ModulePath::of_file(Path::new(name)), None);
    }
}
