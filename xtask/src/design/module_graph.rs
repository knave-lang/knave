use std::collections::{BTreeMap, BTreeSet};

use syn::visit::{self, Visit};
use syn::{ItemUse, Path, Visibility};

use super::defining_modules::DefiningModules;
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
        let edges = {
            let defining = DefiningModules::build(sources, &modules);
            sources
                .iter()
                .map(|source| (source.file().to_string(), Self::targets(source, &defining)))
                .collect()
        };
        Self { modules, edges }
    }

    pub fn cycle_links(&self) -> Counts {
        let reachable: BTreeMap<&String, BTreeSet<&String>> = self
            .modules
            .files()
            .map(|module| (module, self.reachable_from(module)))
            .collect();
        self.edges
            .iter()
            .map(|(module, targets)| {
                let links = targets
                    .iter()
                    .filter(|target| reachable[target].contains(module))
                    .count();
                (module.clone(), links)
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

    fn targets(source: &ParsedSource, defining: &DefiningModules) -> BTreeSet<String> {
        let mut paths = PathCollector::default();
        paths.visit_file(source.syntax());
        paths
            .paths
            .iter()
            .filter_map(|path| defining.target(source.file(), path))
            .filter(|target| *target != source.file())
            .map(String::from)
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
        if !matches!(item.vis, Visibility::Inherited) {
            return;
        }
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
    fn each_module_of_a_cycle_counts_its_own_links_within_it() {
        let partners = Modules::graph(&[
            ("src/a.rs", "use crate::b::Thing; use crate::c::Other;"),
            ("src/b.rs", "fn make() -> crate::c::Other { todo!() }"),
            ("src/c.rs", "use super::a::{First, Second};"),
            ("src/d.rs", "use crate::a::First;"),
        ])
        .cycle_links();
        assert_eq!(partners.get("src/a.rs"), Some(&2));
        assert_eq!(partners.get("src/b.rs"), Some(&1));
        assert_eq!(partners.get("src/c.rs"), Some(&1));
        assert_eq!(partners.get("src/d.rs"), Some(&0));
    }

    #[test]
    fn a_parent_that_only_re_exports_forms_no_cycle_with_its_children() {
        let partners = Modules::graph(&[
            (
                "src/shapes/mod.rs",
                "mod circle; mod square; pub use circle::Circle; pub(crate) use square::Square;",
            ),
            (
                "src/shapes/circle.rs",
                "use super::Square; pub struct Circle(Square);",
            ),
            ("src/shapes/square.rs", "pub struct Square;"),
        ])
        .cycle_links();
        assert!(partners.values().all(|links| *links == 0), "{partners:?}");
    }

    #[test]
    fn a_cycle_through_re_exports_is_found() {
        let partners = Modules::graph(&[
            (
                "src/types/mod.rs",
                "mod checker; mod scheme; pub use checker::TypeChecker; pub use scheme::Scheme;",
            ),
            (
                "src/types/checker.rs",
                "use super::Scheme; pub struct TypeChecker(Scheme);",
            ),
            (
                "src/types/scheme.rs",
                "use super::TypeChecker; pub struct Scheme(Box<TypeChecker>);",
            ),
        ])
        .cycle_links();
        assert_eq!(partners.get("src/types/checker.rs"), Some(&1));
        assert_eq!(partners.get("src/types/scheme.rs"), Some(&1));
        assert_eq!(partners.get("src/types/mod.rs"), Some(&0));
    }

    #[test]
    fn a_cycle_through_a_glob_re_export_and_a_chain_is_found() {
        let partners = Modules::graph(&[
            (
                "src/types/mod.rs",
                "mod checker; mod scheme; mod outer; pub use outer::*; pub use scheme::*;",
            ),
            ("src/types/outer.rs", "pub use super::checker::TypeChecker;"),
            (
                "src/types/checker.rs",
                "use super::Scheme; pub struct TypeChecker;",
            ),
            (
                "src/types/scheme.rs",
                "use super::TypeChecker; pub struct Scheme;",
            ),
        ])
        .cycle_links();
        assert_eq!(partners.get("src/types/checker.rs"), Some(&1));
        assert_eq!(partners.get("src/types/scheme.rs"), Some(&1));
        assert_eq!(partners.get("src/types/outer.rs"), Some(&0));
    }

    #[test]
    fn a_re_exported_module_leads_to_that_module() {
        let partners = Modules::graph(&[
            ("src/a.rs", "use crate::shapes::geometry; pub struct Thing;"),
            ("src/shapes/mod.rs", "pub use crate::geometry;"),
            ("src/geometry.rs", "use crate::a::Thing;"),
        ])
        .cycle_links();
        assert_eq!(partners.get("src/a.rs"), Some(&1));
        assert_eq!(partners.get("src/geometry.rs"), Some(&1));
    }

    #[test]
    fn a_glob_of_enum_variants_offers_no_item_of_its_module() {
        let partners = Modules::graph(&[
            (
                "src/shapes/mod.rs",
                "mod child; mod shape; use child::Probe; pub use shape::Kind::*;",
            ),
            (
                "src/shapes/shape.rs",
                "pub enum Kind { Circle } pub struct Circle;",
            ),
            (
                "src/shapes/child.rs",
                "use super::Circle; pub struct Probe;",
            ),
        ])
        .cycle_links();
        assert_eq!(partners.get("src/shapes/child.rs"), Some(&1));
    }

    #[test]
    fn a_new_link_inside_a_cycle_raises_one_entry() {
        let ring = [
            ("src/a.rs", "use crate::b::B; pub struct A;"),
            ("src/b.rs", "use crate::c::C; pub struct B;"),
            ("src/c.rs", "use crate::a::A; pub struct C;"),
        ];
        let before = Modules::graph(&ring).cycle_links();
        let mut linked = ring;
        linked[0] = (
            "src/a.rs",
            "use crate::b::B; use crate::c::C; pub struct A;",
        );
        let after = Modules::graph(&linked).cycle_links();
        let raised: Vec<&String> = after
            .iter()
            .filter(|(module, links)| before.get(*module) != Some(links))
            .map(|(module, _)| module)
            .collect();
        assert_eq!(raised, ["src/a.rs"]);
        assert_eq!(after.get("src/a.rs"), Some(&2));
    }

    #[test]
    fn a_new_file_joining_a_cycle_raises_only_the_modules_it_links() {
        let pair = [
            ("src/a.rs", "use crate::b::B; pub struct A;"),
            ("src/b.rs", "use crate::a::A; pub struct B;"),
        ];
        let before = Modules::graph(&pair).cycle_links();
        let after = Modules::graph(&[
            pair[0],
            (
                "src/b.rs",
                "use crate::a::A; use crate::c::C; pub struct B;",
            ),
            ("src/c.rs", "use crate::a::A; pub struct C;"),
        ])
        .cycle_links();
        assert_eq!(after.get("src/a.rs"), before.get("src/a.rs"));
        assert_eq!(after.get("src/b.rs"), Some(&2));
        assert_eq!(after.get("src/c.rs"), Some(&1));
    }

    #[test]
    fn a_name_that_cannot_be_resolved_keeps_its_edge() {
        let partners = Modules::graph(&[
            ("src/shapes/mod.rs", "mod child; use child::Probe;"),
            (
                "src/shapes/child.rs",
                "use super::Unknown; pub struct Probe;",
            ),
        ])
        .cycle_links();
        assert_eq!(partners.get("src/shapes/child.rs"), Some(&1));
    }

    #[test]
    fn a_parent_and_its_child_module_can_form_a_cycle() {
        let partners = Modules::graph(&[
            (
                "src/shapes/mod.rs",
                "mod square; use square::Square; pub trait Shape {}",
            ),
            (
                "src/shapes/square.rs",
                "use super::Shape; pub struct Square;",
            ),
        ])
        .cycle_links();
        assert_eq!(partners.get("src/shapes/mod.rs"), Some(&1));
    }

    #[test]
    fn a_self_path_reaches_only_a_child_module() {
        let partners = Modules::graph(&[
            ("src/a.rs", "use self::b::Thing;"),
            ("src/b.rs", "use crate::a::Other;"),
        ])
        .cycle_links();
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
                    .cycle_links();
            assert_eq!(partners.get("src/a.rs"), Some(&1), "{using}");
        }
    }

    #[test]
    fn a_restricted_visibility_adds_no_edge() {
        for visibility in ["pub(super)", "pub(in crate::shapes)"] {
            let partners = Modules::graph(&[
                ("src/shapes/mod.rs", "mod child; use child::Probe;"),
                (
                    "src/shapes/child.rs",
                    &format!("pub struct Probe; impl Probe {{ {visibility} fn make() {{}} }}"),
                ),
            ])
            .cycle_links();
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
        .cycle_links();
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
        .cycle_links();
        assert_eq!(partners.get("src/a.rs"), Some(&0));
    }

    #[test]
    fn crates_keep_their_modules_apart() {
        let partners = Modules::graph(&[
            ("src/a.rs", "use crate::b::X;"),
            ("expand/src/b.rs", "use crate::a::X;"),
            ("expand/src/a.rs", ""),
        ])
        .cycle_links();
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
