use super::name_key::NameKey;
use super::saved_keys::{SavedKeys, SavedKeysBuilder};
use crate::core::Namespace;

#[derive(serde::Serialize)]
pub struct ModulePathBytes {
    keys: SavedKeysBuilder,
}

impl ModulePathBytes {
    pub fn of<'a>(paths: impl ExactSizeIterator<Item = &'a str>) -> Self {
        let mut keys = SavedKeysBuilder::for_count(paths.len());
        for path in paths {
            keys.add(SavedModulePaths::key(path));
        }
        Self { keys }
    }
}

#[derive(Clone, Copy, Default, serde::Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct SavedModulePaths {
    keys: SavedKeys,
}

impl SavedModulePaths {
    const PATH_NAMESPACE: Namespace = Namespace::Value;

    pub fn position(&self, path: &str) -> Option<usize> {
        self.keys.position(Self::key(path))
    }

    pub fn path(&self, position: usize) -> &'static str {
        let (path, _) = self.keys.key(position);
        path
    }

    pub fn count(&self) -> usize {
        self.keys.count()
    }

    fn key(path: &str) -> NameKey<'_> {
        NameKey::new(path, Self::PATH_NAMESPACE)
    }
}

#[cfg(test)]
mod tests {
    use super::{ModulePathBytes, SavedModulePaths};

    struct PathsFixture;

    impl PathsFixture {
        const PATHS: [&str; 3] = ["prelude", "prelude/option", "prelude/internal/macros"];

        fn saved() -> SavedModulePaths {
            let bytes = bincode::serialize(&ModulePathBytes::of(Self::PATHS.into_iter()))
                .expect("the paths save");
            bincode::deserialize(bytes.leak()).expect("the paths load")
        }
    }

    #[test]
    fn a_saved_path_is_found_at_its_position() {
        let saved = PathsFixture::saved();
        for (position, path) in PathsFixture::PATHS.into_iter().enumerate() {
            assert_eq!(saved.position(path), Some(position));
            assert_eq!(saved.path(position), path);
        }
        assert_eq!(saved.count(), PathsFixture::PATHS.len());
        assert_eq!(saved.position("prelude/result"), None);
    }
}
