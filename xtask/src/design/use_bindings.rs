use std::collections::BTreeMap;

use syn::ItemUse;
use syn::visit::Visit;

use super::use_paths::UseTreeWalk;

#[derive(Default)]
pub struct UseBindings {
    paths: BTreeMap<String, Vec<String>>,
    globs: Vec<Vec<String>>,
}

impl UseBindings {
    pub fn of(syntax: &syn::File) -> Self {
        let mut bindings = Self::default();
        bindings.visit_file(syntax);
        bindings
    }

    pub fn path_of(&self, name: &str) -> Option<&[String]> {
        self.paths.get(name).map(Vec::as_slice)
    }

    pub fn globs(&self) -> &[Vec<String>] {
        &self.globs
    }
}

impl<'ast> Visit<'ast> for UseBindings {
    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        for found in UseTreeWalk::paths(&item.tree) {
            match found.binding {
                Some(binding) => {
                    self.paths.insert(binding, found.path);
                }
                None => self.globs.push(found.path),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UseBindings;

    struct Uses;

    impl Uses {
        fn path_of(text: &str, name: &str) -> Option<Vec<String>> {
            let syntax = syn::parse_file(text).expect("the uses parse");
            UseBindings::of(&syntax)
                .path_of(name)
                .map(<[String]>::to_vec)
        }
    }

    #[test]
    fn a_name_is_bound_to_its_whole_path() {
        assert_eq!(
            Uses::path_of("use super::shape::{Shape, Other};", "Shape"),
            Some(vec!["super".into(), "shape".into(), "Shape".into()])
        );
    }

    #[test]
    fn a_rename_binds_the_new_name_to_the_original_path() {
        assert_eq!(
            Uses::path_of("use crate::shape::Shape as Outline;", "Outline"),
            Some(vec!["crate".into(), "shape".into(), "Shape".into()])
        );
        assert_eq!(
            Uses::path_of("use crate::shape::Shape as Outline;", "Shape"),
            None
        );
    }

    #[test]
    fn a_self_import_binds_its_module() {
        assert_eq!(
            Uses::path_of("use crate::shape::{self};", "shape"),
            Some(vec!["crate".into(), "shape".into()])
        );
    }

    #[test]
    fn a_glob_binds_nothing() {
        assert_eq!(Uses::path_of("use crate::shape::*;", "Shape"), None);
    }

    #[test]
    fn a_glob_is_kept_as_a_module_to_search() {
        let syntax = syn::parse_file("use crate::shape::*;").expect("the use parses");
        assert_eq!(
            UseBindings::of(&syntax).globs(),
            [vec!["crate".to_string(), "shape".to_string()]]
        );
    }
}
