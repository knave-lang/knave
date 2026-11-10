use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use syn::visit::{self, Visit};
use syn::{Ident, ImplItem, ItemEnum, ItemImpl, ItemStruct, ItemUnion, Type, Visibility};

use super::module_address::ModuleFiles;
use super::use_bindings::UseBindings;
use crate::parsed_source::ParsedSource;
use crate::ratchet::Counts;

#[derive(Default)]
pub struct TypeShapes {
    methods: Counts,
    non_private_methods: Counts,
    inherent_impl_files: Counts,
    fields: Counts,
}

impl TypeShapes {
    pub fn measure(sources: &[ParsedSource]) -> Self {
        let definitions = TypeDefinitions::collect(sources);
        let mut census = ImplCensus::default();
        for source in sources {
            ShapeVisitor {
                source,
                definitions: &definitions,
                bindings: UseBindings::of(source.syntax()),
                census: &mut census,
            }
            .visit_file(source.syntax());
        }
        census.into_shapes()
    }

    pub fn into_counts(self) -> [Counts; 4] {
        [
            self.methods,
            self.non_private_methods,
            self.fields,
            self.inherent_impl_files,
        ]
    }
}

struct TypeKey<'a> {
    file: &'a str,
    name: &'a str,
}

impl fmt::Display for TypeKey<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}::{}", self.file, self.name)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct CrateType {
    crate_directory: String,
    name: String,
}

impl CrateType {
    fn of(source: &ParsedSource, name: &str) -> Self {
        Self {
            crate_directory: source.crate_directory().to_string(),
            name: name.to_string(),
        }
    }
}

struct TypeDefinitions {
    files: BTreeMap<CrateType, BTreeSet<String>>,
    modules: ModuleFiles,
}

impl TypeDefinitions {
    fn collect(sources: &[ParsedSource]) -> Self {
        let mut definitions = Self {
            files: BTreeMap::new(),
            modules: ModuleFiles::build(sources),
        };
        for source in sources {
            DefinitionVisitor {
                source,
                definitions: &mut definitions,
            }
            .visit_file(source.syntax());
        }
        definitions
    }

    fn define(&mut self, source: &ParsedSource, name: &Ident) {
        self.files
            .entry(CrateType::of(source, &name.to_string()))
            .or_default()
            .insert(source.file().to_string());
    }

    fn keys<'a>(&'a self, source: &'a ParsedSource, written: &'a ImplTarget) -> Vec<TypeKey<'a>> {
        let name = written.name();
        let Some(defined_in) = self.files.get(&CrateType::of(source, name)) else {
            return vec![TypeKey {
                file: source.file(),
                name,
            }];
        };
        let resolved = match written.module_path() {
            Some(path) => self.modules.resolve(source.file(), path),
            None => defined_in.get(source.file()),
        };
        let files: Vec<&String> = match resolved.filter(|file| defined_in.contains(*file)) {
            Some(file) => vec![file],
            None => defined_in.iter().collect(),
        };
        files
            .into_iter()
            .map(|file| TypeKey { file, name })
            .collect()
    }
}

struct DefinitionVisitor<'a> {
    source: &'a ParsedSource,
    definitions: &'a mut TypeDefinitions,
}

impl<'ast> Visit<'ast> for DefinitionVisitor<'_> {
    fn visit_item_struct(&mut self, definition: &'ast ItemStruct) {
        self.definitions.define(self.source, &definition.ident);
        visit::visit_item_struct(self, definition);
    }

    fn visit_item_enum(&mut self, definition: &'ast ItemEnum) {
        self.definitions.define(self.source, &definition.ident);
        visit::visit_item_enum(self, definition);
    }

    fn visit_item_union(&mut self, definition: &'ast ItemUnion) {
        self.definitions.define(self.source, &definition.ident);
        visit::visit_item_union(self, definition);
    }
}

#[derive(Default)]
struct ImplCensus {
    shapes: TypeShapes,
    impl_files: BTreeMap<String, BTreeSet<String>>,
}

impl ImplCensus {
    fn record_impl(&mut self, key: &TypeKey, file: &str, implementation: &ItemImpl) {
        let methods: Vec<&Visibility> = implementation
            .items
            .iter()
            .filter_map(|item| match item {
                ImplItem::Fn(method) => Some(&method.vis),
                _ => None,
            })
            .collect();
        let non_private = methods
            .iter()
            .filter(|visibility| !matches!(visibility, Visibility::Inherited))
            .count();
        let key = key.to_string();
        *self.shapes.methods.entry(key.clone()).or_default() += methods.len();
        *self
            .shapes
            .non_private_methods
            .entry(key.clone())
            .or_default() += non_private;
        self.impl_files
            .entry(key)
            .or_default()
            .insert(file.to_string());
    }

    fn record_struct(&mut self, key: &TypeKey, fields: usize) {
        let recorded = self.shapes.fields.entry(key.to_string()).or_default();
        *recorded = (*recorded).max(fields);
    }

    fn into_shapes(mut self) -> TypeShapes {
        self.shapes.inherent_impl_files = self
            .impl_files
            .into_iter()
            .map(|(key, files)| (key, files.len()))
            .collect();
        self.shapes
    }
}

enum ImplTarget {
    Local(String),
    Through { name: String, path: Vec<String> },
}

impl ImplTarget {
    fn of(implementation: &ItemImpl, bindings: &UseBindings) -> Option<Self> {
        let Type::Path(written) = implementation.self_ty.as_ref() else {
            return None;
        };
        let path: Vec<String> = written
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        match path.as_slice() {
            [name] => match bindings.path_of(name) {
                Some(imported) => Self::through(imported),
                None => Some(Self::Local(name.clone())),
            },
            _ => Self::through(&path),
        }
    }

    fn through(path: &[String]) -> Option<Self> {
        let name = path.last()?.clone();
        Some(Self::Through {
            name,
            path: path.to_vec(),
        })
    }

    fn name(&self) -> &str {
        match self {
            Self::Local(name) | Self::Through { name, .. } => name,
        }
    }

    fn module_path(&self) -> Option<&[String]> {
        match self {
            Self::Local(_) => None,
            Self::Through { path, .. } => Some(path),
        }
    }
}

struct ShapeVisitor<'a> {
    source: &'a ParsedSource,
    definitions: &'a TypeDefinitions,
    bindings: UseBindings,
    census: &'a mut ImplCensus,
}

impl<'ast> Visit<'ast> for ShapeVisitor<'_> {
    fn visit_item_impl(&mut self, implementation: &'ast ItemImpl) {
        let inherent_type = implementation
            .trait_
            .is_none()
            .then(|| ImplTarget::of(implementation, &self.bindings))
            .flatten();
        if let Some(target) = inherent_type {
            for key in self.definitions.keys(self.source, &target) {
                self.census
                    .record_impl(&key, self.source.file(), implementation);
            }
        }
        visit::visit_item_impl(self, implementation);
    }

    fn visit_item_struct(&mut self, definition: &'ast ItemStruct) {
        let name = definition.ident.to_string();
        let key = TypeKey {
            file: self.source.file(),
            name: &name,
        };
        self.census.record_struct(&key, definition.fields.len());
        visit::visit_item_struct(self, definition);
    }
}

#[cfg(test)]
mod tests {
    use super::TypeShapes;
    use crate::parsed_source::ParsedSourceFixture;

    struct Sources;

    impl Sources {
        fn shapes(files: &[(&str, &str)]) -> TypeShapes {
            let sources: Vec<_> = files
                .iter()
                .map(|(file, text)| ParsedSourceFixture::sample(file, text))
                .collect();
            TypeShapes::measure(&sources)
        }
    }

    #[test]
    fn counts_the_methods_of_every_inherent_impl_but_no_trait_impl() {
        let shapes = Sources::shapes(&[(
            "src/a.rs",
            "struct Square; impl Square { fn a() {} pub fn b() {} } impl Square { fn c() {} } impl Clone for Square { fn clone(&self) -> Self { Square } }",
        )]);
        assert_eq!(shapes.methods.get("src/a.rs::Square"), Some(&3));
    }

    #[test]
    fn counts_public_crate_and_super_methods_as_non_private() {
        let shapes = Sources::shapes(&[(
            "src/a.rs",
            "struct Square; impl Square { fn a() {} pub fn b() {} pub(crate) fn c() {} pub(super) fn d() {} #[cfg(test)] pub fn e() {} }",
        )]);
        assert_eq!(shapes.non_private_methods.get("src/a.rs::Square"), Some(&3));
    }

    #[test]
    fn counts_each_file_that_holds_an_inherent_impl_of_a_type() {
        let shapes = Sources::shapes(&[
            ("src/a.rs", "pub struct Square; impl Square { fn a() {} }"),
            (
                "src/b.rs",
                "use crate::a::Square; impl Square { fn b() {} }",
            ),
        ]);
        assert_eq!(shapes.inherent_impl_files.get("src/a.rs::Square"), Some(&2));
        assert_eq!(shapes.methods.get("src/a.rs::Square"), Some(&2));
    }

    #[test]
    fn a_type_defined_in_several_files_is_known_by_its_impl_file() {
        let shapes = Sources::shapes(&[
            ("src/a.rs", "struct Fixture; impl Fixture { fn a() {} }"),
            ("src/b.rs", "struct Fixture; impl Fixture { fn b() {} }"),
        ]);
        assert_eq!(
            shapes.inherent_impl_files.get("src/a.rs::Fixture"),
            Some(&1)
        );
        assert_eq!(
            shapes.inherent_impl_files.get("src/b.rs::Fixture"),
            Some(&1)
        );
    }

    #[test]
    fn counts_the_fields_of_each_struct() {
        let shapes = Sources::shapes(&[(
            "src/a.rs",
            "struct Point { x: i32, y: i32 } struct Pair(u8, u8, u8); struct Unit;",
        )]);
        assert_eq!(shapes.fields.get("src/a.rs::Point"), Some(&2));
        assert_eq!(shapes.fields.get("src/a.rs::Pair"), Some(&3));
        assert_eq!(shapes.fields.get("src/a.rs::Unit"), Some(&0));
    }

    #[test]
    fn a_struct_defined_twice_counts_its_larger_definition() {
        let shapes = Sources::shapes(&[(
            "src/a.rs",
            "#[cfg(unix)] struct Point { x: i32, y: i32 } #[cfg(not(unix))] struct Point { x: i32 }",
        )]);
        assert_eq!(shapes.fields.get("src/a.rs::Point"), Some(&2));
    }

    #[test]
    fn an_impl_counts_under_the_definition_its_use_names() {
        let shapes = Sources::shapes(&[
            (
                "expand/src/expand/shape.rs",
                "pub struct Shape; impl Shape { pub fn a(&self) {} }",
            ),
            (
                "expand/src/format/shape.rs",
                "pub struct Shape; impl Shape { fn b(&self) {} }",
            ),
            (
                "expand/src/expand/extra.rs",
                "use super::shape::Shape; impl Shape { pub fn one(&self) {} pub fn two(&self) {} }",
            ),
        ]);
        let expand = "expand/src/expand/shape.rs::Shape";
        let format = "expand/src/format/shape.rs::Shape";
        assert_eq!(shapes.methods.get(expand), Some(&3));
        assert_eq!(shapes.inherent_impl_files.get(expand), Some(&2));
        assert_eq!(shapes.methods.get(format), Some(&1));
        assert_eq!(shapes.inherent_impl_files.get(format), Some(&1));
    }

    #[test]
    fn an_impl_through_a_written_path_counts_under_that_definition() {
        let shapes = Sources::shapes(&[
            ("src/expand/shape.rs", "pub struct Shape;"),
            ("src/format/shape.rs", "pub struct Shape;"),
            (
                "src/expand/extra.rs",
                "impl super::shape::Shape { fn one() {} }",
            ),
        ]);
        assert_eq!(shapes.methods.get("src/expand/shape.rs::Shape"), Some(&1));
        assert_eq!(shapes.methods.get("src/format/shape.rs::Shape"), None);
    }

    #[test]
    fn an_impl_that_cannot_be_resolved_counts_under_every_definition() {
        let shapes = Sources::shapes(&[
            ("src/expand/shape.rs", "pub struct Shape;"),
            ("src/format/shape.rs", "pub struct Shape;"),
            (
                "src/other.rs",
                "use crate::expand::shape::*; impl Shape { fn one() {} }",
            ),
        ]);
        assert_eq!(shapes.methods.get("src/expand/shape.rs::Shape"), Some(&1));
        assert_eq!(shapes.methods.get("src/format/shape.rs::Shape"), Some(&1));
    }

    #[test]
    fn an_impl_through_a_re_export_counts_under_every_definition() {
        let shapes = Sources::shapes(&[
            ("src/expand/mod.rs", "mod shape; pub use shape::Shape;"),
            ("src/expand/shape.rs", "pub struct Shape;"),
            ("src/format/shape.rs", "pub struct Shape;"),
            (
                "src/other.rs",
                "use crate::expand::Shape; impl Shape { fn one() {} }",
            ),
        ]);
        assert_eq!(shapes.methods.get("src/expand/mod.rs::Shape"), None);
        assert_eq!(shapes.methods.get("src/expand/shape.rs::Shape"), Some(&1));
        assert_eq!(shapes.methods.get("src/format/shape.rs::Shape"), Some(&1));
    }
}
