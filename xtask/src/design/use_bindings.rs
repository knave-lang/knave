use std::collections::BTreeMap;

use syn::visit::Visit;
use syn::{ItemUse, UseTree};

#[derive(Default)]
pub struct UseBindings {
    paths: BTreeMap<String, Vec<String>>,
}

impl UseBindings {
    const SELF: &str = "self";

    pub fn of(syntax: &syn::File) -> Self {
        let mut bindings = Self::default();
        bindings.visit_file(syntax);
        bindings
    }

    pub fn path_of(&self, name: &str) -> Option<&[String]> {
        self.paths.get(name).map(Vec::as_slice)
    }

    fn bind(&mut self, tree: &UseTree, prefix: &[String]) {
        match tree {
            UseTree::Path(path) => self.bind(&path.tree, &Self::joined(prefix, &path.ident)),
            UseTree::Name(name) if name.ident == Self::SELF => {
                if let Some(module) = prefix.last() {
                    self.paths.insert(module.clone(), prefix.to_vec());
                }
            }
            UseTree::Name(name) => {
                self.paths
                    .insert(name.ident.to_string(), Self::joined(prefix, &name.ident));
            }
            UseTree::Rename(rename) => {
                self.paths.insert(
                    rename.rename.to_string(),
                    Self::joined(prefix, &rename.ident),
                );
            }
            UseTree::Glob(_) => {}
            UseTree::Group(group) => group.items.iter().for_each(|item| self.bind(item, prefix)),
        }
    }

    fn joined(prefix: &[String], name: &syn::Ident) -> Vec<String> {
        [prefix, &[name.to_string()]].concat()
    }
}

impl<'ast> Visit<'ast> for UseBindings {
    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        self.bind(&item.tree, &[]);
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
}
