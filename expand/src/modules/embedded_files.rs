use std::cmp::Ordering;

use crate::core::ModulePath;

pub struct EmbeddedModules;

impl EmbeddedModules {
    const FILES_IN_PAGE_OWNER_ORDER: [(&str, &str); 29] = [
        ("prelude", include_str!("../../prelude/prelude.knv")),
        (
            "prelude/internal/macros",
            include_str!("../../prelude/internal/macros.knv"),
        ),
        ("prelude/macros", include_str!("../../prelude/macros.knv")),
        ("prelude/logic", include_str!("../../prelude/logic.knv")),
        (
            "prelude/internal/syntax",
            include_str!("../../prelude/internal/syntax.knv"),
        ),
        ("prelude/syntax", include_str!("../../prelude/syntax.knv")),
        ("prelude/number", include_str!("../../prelude/number.knv")),
        ("prelude/text", include_str!("../../prelude/text.knv")),
        ("prelude/struct", include_str!("../../prelude/struct.knv")),
        (
            "prelude/definers",
            include_str!("../../prelude/definers.knv"),
        ),
        (
            "prelude/ownership",
            include_str!("../../prelude/ownership.knv"),
        ),
        ("prelude/types", include_str!("../../prelude/types.knv")),
        ("prelude/pair", include_str!("../../prelude/pair.knv")),
        ("prelude/assoc", include_str!("../../prelude/assoc.knv")),
        ("prelude/place", include_str!("../../prelude/place.knv")),
        ("prelude/vec", include_str!("../../prelude/vec.knv")),
        (
            "prelude/iterator",
            include_str!("../../prelude/iterator.knv"),
        ),
        ("prelude/option", include_str!("../../prelude/option.knv")),
        ("prelude/result", include_str!("../../prelude/result.knv")),
        ("prelude/io", include_str!("../../prelude/io.knv")),
        ("prelude/map", include_str!("../../prelude/map.knv")),
        (
            "prelude/sequence",
            include_str!("../../prelude/sequence.knv"),
        ),
        (
            "prelude/internal/docstrings",
            include_str!("../../prelude/internal/docstrings.knv"),
        ),
        ("prelude/test", include_str!("../../prelude/test.knv")),
        ("prelude/task", include_str!("../../prelude/task.knv")),
        (
            "prelude/project-file",
            include_str!("../../prelude/project-file.knv"),
        ),
        (
            "prelude/documentation",
            include_str!("../../prelude/documentation.knv"),
        ),
        ("prelude/lint", include_str!("../../prelude/lint.knv")),
        (
            "prelude/internal/links",
            include_str!("../../prelude/internal/links.knv"),
        ),
    ];
    const FILE_COUNT: usize = Self::FILES_IN_PAGE_OWNER_ORDER.len();
    const POSITIONS_IN_SEARCH_ORDER: [usize; Self::FILE_COUNT] = Self::positions_in_search_order();

    pub fn paths() -> impl Iterator<Item = &'static str> {
        Self::FILES_IN_PAGE_OWNER_ORDER
            .iter()
            .map(|(path, _)| *path)
    }

    pub fn contains(path: &ModulePath) -> bool {
        Self::position(path).is_some()
    }

    pub fn text(path: &ModulePath) -> Option<&'static str> {
        let (_, text) = Self::FILES_IN_PAGE_OWNER_ORDER[Self::position(path)?];
        Some(text)
    }

    fn position(path: &ModulePath) -> Option<usize> {
        let found = Self::POSITIONS_IN_SEARCH_ORDER
            .binary_search_by(|position| {
                Self::search_order(Self::path_at(*position), path.as_str())
            })
            .ok()?;
        Some(Self::POSITIONS_IN_SEARCH_ORDER[found])
    }

    fn search_order(listed: &str, sought: &str) -> Ordering {
        listed
            .len()
            .cmp(&sought.len())
            .then_with(|| listed.cmp(sought))
    }

    const fn positions_in_search_order() -> [usize; Self::FILE_COUNT] {
        let mut positions = Self::table_positions();
        let mut sorted = 1;
        while sorted < Self::FILE_COUNT {
            Self::insert_in_search_order(&mut positions, sorted);
            sorted += 1;
        }
        positions
    }

    const fn table_positions() -> [usize; Self::FILE_COUNT] {
        let mut positions = [0; Self::FILE_COUNT];
        let mut position = 0;
        while position < Self::FILE_COUNT {
            positions[position] = position;
            position += 1;
        }
        positions
    }

    const fn insert_in_search_order(positions: &mut [usize; Self::FILE_COUNT], end: usize) {
        let mut slot = end;
        while slot > 0 && Self::precedes_in_search_order(positions[slot], positions[slot - 1]) {
            let moved = positions[slot];
            positions[slot] = positions[slot - 1];
            positions[slot - 1] = moved;
            slot -= 1;
        }
    }

    const fn precedes_in_search_order(left: usize, right: usize) -> bool {
        let (left, right) = (Self::path_at(left), Self::path_at(right));
        if left.len() != right.len() {
            return left.len() < right.len();
        }
        Self::text_precedes(left.as_bytes(), right.as_bytes())
    }

    const fn text_precedes(left: &[u8], right: &[u8]) -> bool {
        let mut at = 0;
        while at < left.len() && at < right.len() && left[at] == right[at] {
            at += 1;
        }
        if at == left.len() || at == right.len() {
            return left.len() < right.len();
        }
        left[at] < right[at]
    }

    const fn path_at(position: usize) -> &'static str {
        Self::FILES_IN_PAGE_OWNER_ORDER[position].0
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::EmbeddedModules;
    use crate::core::ModulePath;
    use crate::language::Language;
    use crate::modules::{ModuleSources, ReservedModulePaths};

    struct PreludeFolder {
        root: PathBuf,
    }

    impl PreludeFolder {
        const DIRECTORY: &str = "prelude";
        const TESTS: &str = "prelude/tests";
        const PRELUDE_FILE: &str = "prelude/prelude";

        fn new() -> Self {
            Self {
                root: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."),
            }
        }

        fn module_files(&self) -> Vec<PathBuf> {
            let mut files = Vec::new();
            self.collect(&self.root.join(Self::DIRECTORY), &mut files);
            files
        }

        fn collect(&self, directory: &Path, files: &mut Vec<PathBuf>) {
            for entry in fs::read_dir(directory).expect("the prelude folder reads") {
                let path = entry.expect("a prelude entry reads").path();
                if path.is_dir() && !path.ends_with(Self::TESTS) {
                    self.collect(&path, files);
                } else if path.extension() == Some(Language::EXTENSION.as_ref()) {
                    files.push(path);
                }
            }
        }

        fn module_path(&self, file: &Path) -> ModulePath {
            let relative = file
                .strip_prefix(&self.root)
                .expect("a prelude file lies under the root");
            let path = ModulePath::of_file(relative).expect("a prelude file name is text");
            if path.as_str() == Self::PRELUDE_FILE {
                return ModulePath::new(ReservedModulePaths::PRELUDE);
            }
            path
        }
    }

    #[test]
    fn the_path_index_lists_every_file_once_in_search_order() {
        let mut expected: Vec<usize> = (0..EmbeddedModules::FILE_COUNT).collect();
        expected.sort_by(|left, right| {
            EmbeddedModules::search_order(
                EmbeddedModules::path_at(*left),
                EmbeddedModules::path_at(*right),
            )
        });
        assert_eq!(
            EmbeddedModules::positions_in_search_order().to_vec(),
            expected
        );
    }

    #[test]
    fn text_order_agrees_with_the_order_of_strings() {
        let pairs = [
            ("a", "b"),
            ("b", "a"),
            ("ab", "a"),
            ("a", "ab"),
            ("a", "a"),
            ("ba", "ab"),
        ];
        for (left, right) in pairs {
            let precedes = EmbeddedModules::text_precedes(left.as_bytes(), right.as_bytes());
            assert_eq!(precedes, left < right, "{left} {right}");
        }
    }

    #[test]
    fn text_finds_every_listed_file_and_contains_agrees() {
        for (position, path) in EmbeddedModules::paths().enumerate() {
            let path = ModulePath::new(path);
            let (_, listed) = EmbeddedModules::FILES_IN_PAGE_OWNER_ORDER[position];
            assert_eq!(EmbeddedModules::text(&path), Some(listed), "{path}");
            assert!(EmbeddedModules::contains(&path), "{path}");
        }
    }

    #[test]
    fn each_prelude_file_outside_its_tests_is_embedded_with_its_text() {
        let folder = PreludeFolder::new();
        let files = folder.module_files();
        for file in &files {
            let path = folder.module_path(file);
            let text = fs::read_to_string(file).expect("a prelude file reads");
            assert_eq!(EmbeddedModules::text(&path), Some(text.as_str()), "{path}");
            assert!(
                EmbeddedModules::paths().any(|listed| listed == path.as_str()),
                "{path}"
            );
        }
        assert_eq!(files.len(), EmbeddedModules::paths().count());
    }

    #[test]
    fn a_module_outside_the_prelude_is_not_embedded() {
        assert!(!EmbeddedModules::contains(&ModulePath::new(
            "prelude/tests/text"
        )));
        assert_eq!(EmbeddedModules::text(&ModulePath::new("geometry")), None);
        assert!(EmbeddedModules::contains(&ModulePath::new(
            ModuleSources::LINT
        )));
    }
}
