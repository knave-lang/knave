use std::collections::{BTreeMap, BTreeSet};

use syn::visit::{self, Visit};
use syn::{ItemUse, Path, Visibility};

use super::module_address::{ModuleFiles, Stage};
use super::use_paths::UseTreeWalk;
use crate::parsed_source::ParsedSource;
use crate::ratchet::Counts;

pub struct ModuleGraph {
    modules: ModuleFiles,
    edges: BTreeMap<String, BTreeSet<String>>,
}

impl ModuleGraph {
    pub fn build(sources: &[ParsedSource]) -> Self {
        let modules = ModuleFiles::build(sources);
        let edges = sources
            .iter()
            .map(|source| (source.file().to_string(), Self::targets(source, &modules)))
            .collect();
        Self { modules, edges }
    }

    pub fn cycle_partners(&self) -> Counts {
        let reachable: BTreeMap<&String, BTreeSet<&String>> = self
            .modules
            .files()
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

    pub fn later_stage_dependencies(&self, stage_order: &[Stage]) -> Counts {
        self.modules
            .files()
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

    fn stage(&self, module: &str, stage_order: &[Stage]) -> Option<usize> {
        let address = self.modules.address(module);
        stage_order.iter().position(|stage| stage.contains(address))
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

    fn targets(source: &ParsedSource, modules: &ModuleFiles) -> BTreeSet<String> {
        let mut paths = PathCollector::default();
        paths.visit_file(source.syntax());
        paths
            .paths
            .iter()
            .filter_map(|path| modules.resolve(source.file(), path))
            .filter(|target| **target != source.file())
            .cloned()
            .collect()
    }
}

#[derive(Default)]
struct PathCollector {
    paths: Vec<Vec<String>>,
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
        self.paths.extend(
            UseTreeWalk::paths(&item.tree)
                .into_iter()
                .map(|found| found.path),
        );
    }

    fn visit_visibility(&mut self, _visibility: &'ast Visibility) {}
}

#[cfg(test)]
mod tests {
    use super::ModuleGraph;
    use crate::design::module_address::Stage;
    use crate::parsed_source::ParsedSourceFixture;

    struct Modules;

    impl Modules {
        const STAGES: [Stage; 2] = [Stage::new("src", "reader"), Stage::new("src", "emit")];

        fn graph(files: &[(&str, &str)]) -> ModuleGraph {
            let sources: Vec<_> = files
                .iter()
                .map(|(file, text)| ParsedSourceFixture::sample(file, text))
                .collect();
            ModuleGraph::build(&sources)
        }
    }

    #[test]
    fn a_stage_belongs_to_its_crate() {
        let later = Modules::graph(&[
            ("expand/src/reader/parse.rs", "use crate::emit::Plan;"),
            ("expand/src/emit/mod.rs", ""),
        ])
        .later_stage_dependencies(&Modules::STAGES);
        assert_eq!(later.get("expand/src/reader/parse.rs"), None);
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
    fn a_restricted_visibility_adds_no_edge() {
        for visibility in ["pub(super)", "pub(in crate::shapes)"] {
            let partners = Modules::graph(&[
                ("src/shapes/mod.rs", "mod child; pub use child::Probe;"),
                (
                    "src/shapes/child.rs",
                    &format!("pub struct Probe; impl Probe {{ {visibility} fn make() {{}} }}"),
                ),
            ])
            .cycle_partners();
            assert_eq!(partners.get("src/shapes/mod.rs"), Some(&0), "{visibility}");
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
