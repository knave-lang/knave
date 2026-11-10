use std::collections::{BTreeMap, BTreeSet};

use syn::visit::{self, Visit};
use syn::{ItemUse, Path, UseTree};

use crate::parsed_source::ParsedSource;
use crate::ratchet::Counts;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ModuleAddress {
    crate_directory: String,
    segments: Vec<String>,
}

impl ModuleAddress {
    const CRATE: &str = "crate";
    const SELF: &str = "self";
    const SUPER: &str = "super";

    fn of(source: &ParsedSource) -> Self {
        Self {
            crate_directory: source.crate_directory().to_string(),
            segments: source.module(),
        }
    }

    fn within(&self, segments: Vec<String>) -> Self {
        Self {
            crate_directory: self.crate_directory.clone(),
            segments,
        }
    }

    fn resolution(&self, path: &[String]) -> Option<Resolution> {
        let (first, rest) = path.split_first()?;
        match first.as_str() {
            Self::CRATE => Some(Resolution::anywhere(rest.to_vec())),
            Self::SELF => Some(self.below(rest)),
            Self::SUPER => self.above(path),
            _ => Some(self.below(path)),
        }
    }

    fn below(&self, path: &[String]) -> Resolution {
        Resolution {
            absolute: [&self.segments[..], path].concat(),
            shortest: self.segments.len() + 1,
        }
    }

    fn above(&self, path: &[String]) -> Option<Resolution> {
        let levels = path
            .iter()
            .take_while(|segment| *segment == Self::SUPER)
            .count();
        let base = self.segments.len().checked_sub(levels)?;
        Some(Resolution::anywhere(
            [&self.segments[..base], &path[levels..]].concat(),
        ))
    }
}

struct Resolution {
    absolute: Vec<String>,
    shortest: usize,
}

impl Resolution {
    const SHORTEST_MODULE_PATH: usize = 1;

    fn anywhere(absolute: Vec<String>) -> Self {
        Self {
            absolute,
            shortest: Self::SHORTEST_MODULE_PATH,
        }
    }
}

pub struct ModuleGraph {
    modules: BTreeMap<String, ModuleAddress>,
    edges: BTreeMap<String, BTreeSet<String>>,
}

impl ModuleGraph {
    pub fn build(sources: &[ParsedSource]) -> Self {
        let modules: BTreeMap<String, ModuleAddress> = sources
            .iter()
            .map(|source| (source.file().to_string(), ModuleAddress::of(source)))
            .collect();
        let files: BTreeMap<&ModuleAddress, &String> = modules
            .iter()
            .map(|(file, address)| (address, file))
            .collect();
        let edges = sources
            .iter()
            .map(|source| (source.file().to_string(), Self::targets(source, &files)))
            .collect();
        Self { modules, edges }
    }

    pub fn cycle_partners(&self) -> Counts {
        let reachable: BTreeMap<&String, BTreeSet<&String>> = self
            .modules
            .keys()
            .map(|module| (module, self.reachable_from(module)))
            .collect();
        reachable
            .iter()
            .map(|(module, targets)| {
                let partners = targets
                    .iter()
                    .filter(|target| *target != module && reachable[*target].contains(module))
                    .count();
                (module.to_string(), partners)
            })
            .collect()
    }

    pub fn later_stage_dependencies(&self, stage_order: &[&str]) -> Counts {
        self.modules
            .keys()
            .filter_map(|module| {
                let stage = self.stage(module, stage_order)?;
                let later = self.edges[module]
                    .iter()
                    .filter(|target| {
                        self.stage(target, stage_order)
                            .is_some_and(|other| other > stage)
                    })
                    .count();
                Some((module.clone(), later))
            })
            .collect()
    }

    fn stage(&self, module: &str, stage_order: &[&str]) -> Option<usize> {
        let top = self.modules[module].segments.first()?;
        stage_order.iter().position(|stage| stage == top)
    }

    fn reachable_from<'a>(&'a self, module: &'a String) -> BTreeSet<&'a String> {
        let mut reached = BTreeSet::new();
        let mut pending = vec![module];
        while let Some(next) = pending.pop() {
            for target in &self.edges[next] {
                if reached.insert(target) {
                    pending.push(target);
                }
            }
        }
        reached
    }

    fn targets(
        source: &ParsedSource,
        files: &BTreeMap<&ModuleAddress, &String>,
    ) -> BTreeSet<String> {
        let address = ModuleAddress::of(source);
        let mut paths = PathCollector::default();
        paths.visit_file(source.syntax());
        paths
            .paths
            .iter()
            .filter_map(|path| Self::resolve(&address, path, files))
            .filter(|target| **target != source.file())
            .cloned()
            .collect()
    }

    fn resolve<'a>(
        address: &ModuleAddress,
        path: &[String],
        files: &BTreeMap<&ModuleAddress, &'a String>,
    ) -> Option<&'a String> {
        let resolution = address.resolution(path)?;
        let absolute = &resolution.absolute;
        (resolution.shortest..=absolute.len())
            .rev()
            .find_map(|length| {
                files
                    .get(&address.within(absolute[..length].to_vec()))
                    .copied()
            })
    }
}

#[derive(Default)]
struct PathCollector {
    paths: Vec<Vec<String>>,
}

impl PathCollector {
    fn flatten(&mut self, tree: &UseTree, prefix: &[String]) {
        match tree {
            UseTree::Path(path) => {
                self.flatten(&path.tree, &[prefix, &[path.ident.to_string()]].concat())
            }
            UseTree::Name(name) => self
                .paths
                .push([prefix, &[name.ident.to_string()]].concat()),
            UseTree::Rename(rename) => self
                .paths
                .push([prefix, &[rename.ident.to_string()]].concat()),
            UseTree::Glob(_) => self.paths.push(prefix.to_vec()),
            UseTree::Group(group) => group
                .items
                .iter()
                .for_each(|item| self.flatten(item, prefix)),
        }
    }
}

impl<'ast> Visit<'ast> for PathCollector {
    fn visit_path(&mut self, path: &'ast Path) {
        self.paths.push(
            path.segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect(),
        );
        visit::visit_path(self, path);
    }

    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        self.flatten(&item.tree, &[]);
    }
}

#[cfg(test)]
mod tests {
    use super::ModuleGraph;
    use crate::parsed_source::ParsedSource;

    struct Modules;

    impl Modules {
        const STAGES: [&str; 2] = ["reader", "emit"];

        fn graph(files: &[(&str, &str)]) -> ModuleGraph {
            let sources: Vec<ParsedSource> = files
                .iter()
                .map(|(file, text)| ParsedSource::sample(file, text))
                .collect();
            ModuleGraph::build(&sources)
        }
    }

    #[test]
    fn modules_that_use_each_other_are_cycle_partners() {
        let partners = Modules::graph(&[
            ("src/a.rs", "use crate::b::Thing;"),
            ("src/b.rs", "fn make() -> crate::c::Other { todo!() }"),
            ("src/c.rs", "use super::a::{First, Second};"),
            ("src/d.rs", "use crate::a::First;"),
        ])
        .cycle_partners();
        assert_eq!(partners.get("src/a.rs"), Some(&2));
        assert_eq!(partners.get("src/c.rs"), Some(&2));
        assert_eq!(partners.get("src/d.rs"), Some(&0));
    }

    #[test]
    fn a_parent_and_its_child_module_can_form_a_cycle() {
        let partners = Modules::graph(&[
            ("src/shapes/mod.rs", "mod square; use square::Square;"),
            ("src/shapes/square.rs", "use super::Shape;"),
        ])
        .cycle_partners();
        assert_eq!(partners.get("src/shapes/mod.rs"), Some(&1));
    }

    #[test]
    fn a_self_path_reaches_only_a_child_module() {
        let partners = Modules::graph(&[
            ("src/a.rs", "use self::b::Thing;"),
            ("src/b.rs", "use crate::a::Other;"),
        ])
        .cycle_partners();
        assert_eq!(partners.get("src/a.rs"), Some(&0));
    }

    #[test]
    fn globs_renames_and_nested_type_paths_add_edges() {
        for using in [
            "use crate::b::*;",
            "use crate::b::Thing as Other;",
            "fn make() -> Option<crate::b::Thing> { None }",
        ] {
            let partners =
                Modules::graph(&[("src/a.rs", using), ("src/b.rs", "use crate::a::Other;")])
                    .cycle_partners();
            assert_eq!(partners.get("src/a.rs"), Some(&1), "{using}");
        }
    }

    #[test]
    fn a_bare_path_in_a_child_never_reaches_its_parent() {
        let partners = Modules::graph(&[
            ("src/shapes/mod.rs", "use square::Square;"),
            (
                "src/shapes/square.rs",
                "fn side() -> String { self::String::new() }",
            ),
        ])
        .cycle_partners();
        assert_eq!(partners.get("src/shapes/square.rs"), Some(&0));
    }

    #[test]
    fn paths_outside_the_crate_add_no_edge() {
        let partners = Modules::graph(&[
            (
                "src/a.rs",
                "use std::fmt; use other_crate::b; use self::inner::X;",
            ),
            ("src/b.rs", "use crate::a::X;"),
        ])
        .cycle_partners();
        assert_eq!(partners.get("src/a.rs"), Some(&0));
    }

    #[test]
    fn crates_keep_their_modules_apart() {
        let partners = Modules::graph(&[
            ("src/a.rs", "use crate::b::X;"),
            ("expand/src/b.rs", "use crate::a::X;"),
            ("expand/src/a.rs", ""),
        ])
        .cycle_partners();
        assert_eq!(partners.get("src/a.rs"), Some(&0));
    }

    #[test]
    fn an_earlier_stage_that_uses_a_later_one_counts() {
        let later = Modules::graph(&[
            (
                "src/reader/parse.rs",
                "use crate::emit::rust::Item; use crate::emit::Plan;",
            ),
            (
                "src/emit/rust.rs",
                "use crate::reader::parse::Form; use crate::emit::Plan;",
            ),
            ("src/emit/mod.rs", ""),
            ("src/support.rs", "use crate::emit::Plan;"),
        ])
        .later_stage_dependencies(&Modules::STAGES);
        assert_eq!(later.get("src/reader/parse.rs"), Some(&2));
        assert_eq!(later.get("src/emit/rust.rs"), Some(&0));
        assert_eq!(later.get("src/support.rs"), None);
    }
}
