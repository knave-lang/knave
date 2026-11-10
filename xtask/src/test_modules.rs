use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use syn::{Expr, ExprLit, Item, ItemMod, Lit, Meta};

use crate::failure::TaskFailure;
use crate::source_file::SourceFile;
use crate::test_code::TestCode;

pub struct TestModuleFiles {
    files: BTreeSet<PathBuf>,
    directories: BTreeSet<PathBuf>,
}

impl TestModuleFiles {
    const PATH_ATTRIBUTE: &str = "path";
    const DIRECTORY_OWNERS: [&str; 3] = ["mod.rs", "lib.rs", "main.rs"];
    const EXTENSION: &str = "rs";

    pub fn find(sources: &[SourceFile]) -> Result<Self, TaskFailure> {
        let mut found = Self {
            files: BTreeSet::new(),
            directories: BTreeSet::new(),
        };
        for source in sources {
            for module in Self::declared_test_modules(&source.syntax()?) {
                found.add(source.path(), module);
            }
        }
        Ok(found)
    }

    pub fn contains(&self, source: &SourceFile) -> bool {
        let path = source.path();
        self.files.contains(path)
            || self
                .directories
                .iter()
                .any(|directory| path.starts_with(directory))
    }

    fn declared_test_modules(syntax: &syn::File) -> Vec<&ItemMod> {
        syntax
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Mod(module) if module.content.is_none() && TestCode::marks(&module.attrs) => {
                    Some(module)
                }
                _ => None,
            })
            .collect()
    }

    fn add(&mut self, parent: &Path, module: &ItemMod) {
        let beside_parent = parent.parent().unwrap_or(parent);
        if let Some(written) = Self::written_path(module) {
            self.files.insert(beside_parent.join(written));
            return;
        }
        let directory =
            Self::module_directory(parent, beside_parent).join(module.ident.to_string());
        self.files.insert(directory.with_extension(Self::EXTENSION));
        self.directories.insert(directory);
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
            SourceFile::new(PathBuf::from(path), path.to_string(), text.to_string())
        }

        fn test_files(declaring: &SourceFile, candidates: &[&str]) -> Vec<String> {
            let found =
                TestModuleFiles::find(std::slice::from_ref(declaring)).expect("the sources parse");
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
    fn a_written_path_names_the_test_module_file() {
        let parent = Tree::file(
            "src/cli/run.rs",
            "/// Cases.\n#[cfg(test)]\n#[path = \"run_cases.rs\"]\nmod cases;",
        );
        assert_eq!(
            Tree::test_files(&parent, &["src/cli/run_cases.rs", "src/cli/run/cases.rs"]),
            ["src/cli/run_cases.rs"]
        );
    }

    #[test]
    fn inline_and_production_modules_declare_no_test_file() {
        let parent = Tree::file("src/lib.rs", "#[cfg(test)]\nmod inline {}\nmod production;");
        assert!(Tree::test_files(&parent, &["src/inline.rs", "src/production.rs"]).is_empty());
    }
}
