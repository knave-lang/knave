use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::rc::Rc;

#[derive(Clone)]
pub struct ModulePath(PathText);

#[derive(Clone)]
enum PathText {
    Built(Rc<str>),
    Saved(&'static str),
}

impl ModulePath {
    pub const SEPARATOR: &str = "/";

    pub fn new(path: impl Into<Rc<str>>) -> Self {
        Self(PathText::Built(path.into()))
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
        match &self.0 {
            PathText::Built(text) => text,
            PathText::Saved(text) => text,
        }
    }

    pub fn head(&self) -> &str {
        self.as_str()
            .split(Self::SEPARATOR)
            .next()
            .unwrap_or_default()
    }
}

impl PartialEq for ModulePath {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for ModulePath {}

impl Hash for ModulePath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl fmt::Debug for ModulePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ModulePath")
            .field(&self.as_str())
            .finish()
    }
}

impl fmt::Display for ModulePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl serde::Serialize for ModulePath {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de: 'static> serde::Deserialize<'de> for ModulePath {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        <&'static str>::deserialize(deserializer).map(|text| Self(PathText::Saved(text)))
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    use super::ModulePath;

    struct SavedPathFixture;

    impl SavedPathFixture {
        fn loaded(path: &ModulePath) -> ModulePath {
            let bytes = bincode::serialize(path).expect("a module path saves");
            bincode::deserialize(bytes.leak()).expect("a module path loads")
        }
    }

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

    #[test]
    fn a_loaded_path_equals_and_hashes_as_its_built_path() {
        let built = ModulePath::new("prelude/option");
        let loaded = SavedPathFixture::loaded(&built);
        assert_eq!(loaded, built);
        let paths: rustc_hash::FxHashSet<ModulePath> = [built].into_iter().collect();
        assert!(paths.contains(&loaded));
        assert_eq!(format!("{loaded:?}"), "ModulePath(\"prelude/option\")");
        assert_eq!(loaded.head(), "prelude");
    }
}
