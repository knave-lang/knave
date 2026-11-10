use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use syn::{Expr, ExprLit, Item, ItemMod, Lit, Meta};

use crate::source_file::SourceFile;
use crate::test_code::TestCode;

pub struct TestModuleFiles {
    files: BTreeSet<PathBuf>,
    directories: BTreeSet<PathBuf>,
    production_paths: BTreeSet<PathBuf>,
}

impl TestModuleFiles {
    const PATH_ATTRIBUTE: &str = "path";
    const DIRECTORY_OWNERS: [&str; 3] = ["mod.rs", "lib.rs", "main.rs"];

    pub fn find(sources: &[SourceFile]) -> Self {
        let mut found = Self {
            files: BTreeSet::new(),
            directories: BTreeSet::new(),
            production_paths: BTreeSet::new(),
        };
        for source in sources {
            let parent = source.path();
            let beside_parent = parent.parent().unwrap_or(parent);
            let directory = Self::module_directory(parent, beside_parent);
            found.collect(&source.syntax().items, &directory, beside_parent);
        }
        found
    }

    pub fn contains(&self, source: &SourceFile) -> bool {
        let path = source.path();
        let declared_for_tests = self.files.contains(path)
            || self
                .directories
                .iter()
                .any(|directory| path.starts_with(directory));
        declared_for_tests && !self.production_paths.contains(path)
    }

    fn collect(&mut self, items: &[Item], directory: &Path, beside_parent: &Path) {
        let modules: Vec<&ItemMod> = items
            .iter()
            .filter_map(|item| match item {
                Item::Mod(module) => Some(module),
                _ => None,
            })
            .collect();
        for module in &modules {
            let namesakes = modules
                .iter()
                .filter(|other| other.ident == module.ident)
                .count();
            let has_a_production_twin = namesakes > 1 && TestCode::marks(&module.attrs);
            if !has_a_production_twin {
                self.declare(module, directory, beside_parent);
            }
        }
    }

    fn declare(&mut self, module: &ItemMod, directory: &Path, beside_parent: &Path) {
        let child = directory.join(module.ident.to_string());
        let is_test = TestCode::marks(&module.attrs);
        match (&module.content, Self::written_path(module)) {
            (Some(_), _) if is_test => {
                self.directories.insert(child);
            }
            (Some((_, items)), _) => self.collect(items, &child, beside_parent),
            (None, Some(written)) if !is_test => {
                self.production_paths
                    .insert(Self::lexically_resolved(&beside_parent.join(written)));
            }
            (None, None) if is_test => {
                self.files
                    .insert(child.with_extension(SourceFile::EXTENSION));
                self.directories.insert(child);
            }
            (None, _) => {}
        }
    }

    fn lexically_resolved(path: &Path) -> PathBuf {
        let mut resolved = PathBuf::new();
        for component in path.components() {
            if component == Component::ParentDir {
                resolved.pop();
            } else {
                resolved.push(component);
            }
        }
        resolved
    }

    fn module_directory(parent: &Path, beside_parent: &Path) -> PathBuf {
        let owns_its_directory = parent
            .file_name()
            .is_some_and(|name| Self::DIRECTORY_OWNERS.iter().any(|owner| name == *owner));
        if owns_its_directory {
            beside_parent.to_path_buf()
        } else {
            parent.with_extension("")
        }
    }

    fn written_path(module: &ItemMod) -> Option<String> {
        module
            .attrs
            .iter()
            .find_map(|attribute| match &attribute.meta {
                Meta::NameValue(pair) if pair.path.is_ident(Self::PATH_ATTRIBUTE) => {
                    match &pair.value {
                        Expr::Lit(ExprLit {
                            lit: Lit::Str(path),
                            ..
                        }) => Some(path.value()),
                        _ => None,
                    }
                }
                _ => None,
            })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::TestModuleFiles;
    use crate::source_file::SourceFile;

    struct Tree;

    impl Tree {
        fn file(path: &str, text: &str) -> SourceFile {
            SourceFile::parse(PathBuf::from(path), path.to_string(), text)
                .expect("the source parses")
        }

        fn test_files(declaring: &SourceFile, candidates: &[&str]) -> Vec<String> {
            let found = TestModuleFiles::find(std::slice::from_ref(declaring));
            candidates
                .iter()
                .filter(|candidate| found.contains(&Self::file(candidate, "")))
                .map(|candidate| candidate.to_string())
                .collect()
        }
    }

    #[test]
    fn a_test_module_declared_by_a_file_module_lies_in_its_directory() {
        let parent = Tree::file(
            "src/types/infer.rs",
            "#[cfg(test)]\nmod typed_source;\nmod kept;",
        );
        assert_eq!(
            Tree::test_files(
                &parent,
                &[
                    "src/types/infer/typed_source.rs",
                    "src/types/infer/typed_source/mod.rs",
                    "src/types/infer/typed_source/deeper.rs",
                    "src/types/infer/kept.rs",
                    "src/types/typed_source.rs",
                ]
            ),
            [
                "src/types/infer/typed_source.rs",
                "src/types/infer/typed_source/mod.rs",
                "src/types/infer/typed_source/deeper.rs",
            ]
        );
    }

    #[test]
    fn a_test_module_declared_by_a_directory_owner_lies_beside_it() {
        for owner in ["src/types/mod.rs", "src/lib.rs"] {
            let parent = Tree::file(owner, "#[cfg(all(test, unix))]\nmod fixture;");
            let beside = owner
                .rsplit_once('/')
                .map_or("", |(directory, _)| directory);
            let expected = format!("{beside}/fixture.rs");
            assert_eq!(
                Tree::test_files(&parent, &[&expected, "src/other.rs"]),
                std::slice::from_ref(&expected)
            );
        }
    }

    #[test]
    fn a_test_module_with_a_written_path_stays_production() {
        let parent = Tree::file(
            "src/types/mod.rs",
            "mod checker;\n/// Cases.\n#[cfg(test)]\n#[path = \"checker.rs\"]\nmod checker_for_tests;",
        );
        assert!(Tree::test_files(&parent, &["src/types/checker.rs"]).is_empty());
    }

    #[test]
    fn a_production_module_with_a_written_path_keeps_its_file_production() {
        let parent = Tree::file(
            "src/types/mod.rs",
            "#[cfg(test)]\nmod fixture;\n#[path = \"fixture.rs\"]\nmod shared;",
        );
        assert!(Tree::test_files(&parent, &["src/types/fixture.rs"]).is_empty());
    }

    #[test]
    fn a_test_module_with_a_production_twin_stays_production() {
        let parent = Tree::file(
            "src/types/mod.rs",
            "#[cfg(test)]\nmod checker;\n#[cfg(not(test))]\nmod checker;",
        );
        assert!(Tree::test_files(&parent, &["src/types/checker.rs"]).is_empty());
    }

    #[test]
    fn a_production_path_through_the_parent_directory_keeps_its_file_production() {
        let parent = Tree::file(
            "src/types/mod.rs",
            "#[cfg(test)]\nmod fixture;\n#[path = \"../types/./fixture.rs\"]\nmod shared;",
        );
        assert!(Tree::test_files(&parent, &["src/types/fixture.rs"]).is_empty());
    }

    #[test]
    fn a_test_module_inside_an_inline_module_lies_in_its_directory() {
        let parent = Tree::file(
            "src/types/mod.rs",
            "mod outer {\n    #[cfg(test)]\n    mod nested;\n    mod kept;\n}",
        );
        assert_eq!(
            Tree::test_files(
                &parent,
                &["src/types/outer/nested.rs", "src/types/outer/kept.rs"]
            ),
            ["src/types/outer/nested.rs"]
        );
    }

    #[test]
    fn the_children_of_an_inline_test_module_are_test_files() {
        let parent = Tree::file(
            "src/lib.rs",
            "#[cfg(test)]\nmod support {\n    mod fixture;\n}",
        );
        assert_eq!(
            Tree::test_files(&parent, &["src/support/fixture.rs", "src/fixture.rs"]),
            ["src/support/fixture.rs"]
        );
    }

    #[test]
    fn inline_and_production_modules_declare_no_test_file() {
        let parent = Tree::file("src/lib.rs", "mod inline {}\nmod production;");
        assert!(Tree::test_files(&parent, &["src/inline.rs", "src/production.rs"]).is_empty());
    }
}
