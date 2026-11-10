use syn::UseTree;

pub struct UsePath {
    pub path: Vec<String>,
    pub binding: Option<String>,
}

pub struct UseTreeWalk {
    found: Vec<UsePath>,
}

impl UseTreeWalk {
    const SELF: &str = "self";

    pub fn paths(tree: &UseTree) -> Vec<UsePath> {
        let mut walk = Self { found: Vec::new() };
        walk.walk(tree, &[]);
        walk.found
    }

    fn walk(&mut self, tree: &UseTree, prefix: &[String]) {
        match tree {
            UseTree::Path(path) => self.walk(&path.tree, &Self::joined(prefix, &path.ident)),
            UseTree::Name(name) if name.ident == Self::SELF => {
                self.add(prefix.to_vec(), prefix.last().cloned());
            }
            UseTree::Name(name) => {
                self.add(
                    Self::joined(prefix, &name.ident),
                    Some(name.ident.to_string()),
                );
            }
            UseTree::Rename(rename) => self.add(
                Self::joined(prefix, &rename.ident),
                Some(rename.rename.to_string()),
            ),
            UseTree::Glob(_) => self.add(prefix.to_vec(), None),
            UseTree::Group(group) => group.items.iter().for_each(|item| self.walk(item, prefix)),
        }
    }

    fn add(&mut self, path: Vec<String>, binding: Option<String>) {
        self.found.push(UsePath { path, binding });
    }

    fn joined(prefix: &[String], name: &syn::Ident) -> Vec<String> {
        [prefix, &[name.to_string()]].concat()
    }
}

#[cfg(test)]
mod tests {
    use super::UseTreeWalk;

    struct Uses;

    impl Uses {
        fn walked(text: &str) -> Vec<(String, Option<String>)> {
            let item: syn::ItemUse = syn::parse_str(text).expect("the use parses");
            UseTreeWalk::paths(&item.tree)
                .into_iter()
                .map(|found| (found.path.join("::"), found.binding))
                .collect()
        }
    }

    #[test]
    fn each_leaf_yields_its_whole_path_and_the_name_it_binds() {
        assert_eq!(
            Uses::walked("use super::shape::{Shape, Line as Stroke, self, edges::*};"),
            [
                ("super::shape::Shape".to_string(), Some("Shape".to_string())),
                ("super::shape::Line".to_string(), Some("Stroke".to_string())),
                ("super::shape".to_string(), Some("shape".to_string())),
                ("super::shape::edges".to_string(), None),
            ]
        );
    }
}
