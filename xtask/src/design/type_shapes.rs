use std::collections::{BTreeMap, BTreeSet};

use syn::visit::{self, Visit};
use syn::{ImplItem, ItemEnum, ItemImpl, ItemStruct, ItemUnion, Visibility};

use crate::functions::FunctionCollector;
use crate::parsed_source::ParsedSource;
use crate::ratchet::Counts;

#[derive(Default)]
pub struct TypeShapes {
    pub methods: Counts,
    pub non_private_methods: Counts,
    pub inherent_impl_files: Counts,
    pub fields: Counts,
}

impl TypeShapes {
    pub fn measure(sources: &[ParsedSource]) -> Self {
        let definitions = TypeDefinitions::collect(sources);
        let mut census = ImplCensus::default();
        for source in sources {
            ShapeVisitor {
                source,
                definitions: &definitions,
                census: &mut census,
            }
            .visit_file(source.syntax());
        }
        census.into_shapes()
    }
}

#[derive(Default)]
struct TypeDefinitions {
    files: BTreeMap<(String, String), BTreeSet<String>>,
    collecting_from: Option<(String, String)>,
}

impl TypeDefinitions {
    fn collect(sources: &[ParsedSource]) -> Self {
        let mut definitions = Self::default();
        for source in sources {
            definitions.collecting_from = Some((
                source.crate_directory().to_string(),
                source.file().to_string(),
            ));
            definitions.visit_file(source.syntax());
        }
        definitions
    }

    fn define(&mut self, name: &syn::Ident) {
        if let Some((crate_directory, file)) = &self.collecting_from {
            self.files
                .entry((crate_directory.clone(), name.to_string()))
                .or_default()
                .insert(file.clone());
        }
    }

    fn key(&self, source: &ParsedSource, name: &str) -> String {
        let defined_in = self
            .files
            .get(&(source.crate_directory().to_string(), name.to_string()));
        let file = match defined_in {
            Some(files) if files.len() == 1 && !files.contains(source.file()) => {
                files.first().map_or(source.file(), String::as_str)
            }
            _ => source.file(),
        };
        format!("{file}::{name}")
    }
}

impl<'ast> Visit<'ast> for TypeDefinitions {
    fn visit_item_struct(&mut self, definition: &'ast ItemStruct) {
        self.define(&definition.ident);
        visit::visit_item_struct(self, definition);
    }

    fn visit_item_enum(&mut self, definition: &'ast ItemEnum) {
        self.define(&definition.ident);
        visit::visit_item_enum(self, definition);
    }

    fn visit_item_union(&mut self, definition: &'ast ItemUnion) {
        self.define(&definition.ident);
        visit::visit_item_union(self, definition);
    }
}

#[derive(Default)]
struct ImplCensus {
    shapes: TypeShapes,
    impl_files: BTreeMap<String, BTreeSet<String>>,
}

impl ImplCensus {
    fn record_impl(&mut self, key: &str, file: &str, implementation: &ItemImpl) {
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
        *self.shapes.methods.entry(key.to_string()).or_default() += methods.len();
        *self
            .shapes
            .non_private_methods
            .entry(key.to_string())
            .or_default() += non_private;
        self.impl_files
            .entry(key.to_string())
            .or_default()
            .insert(file.to_string());
    }

    fn record_struct(&mut self, key: String, fields: usize) {
        let recorded = self.shapes.fields.entry(key).or_default();
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

struct ShapeVisitor<'a> {
    source: &'a ParsedSource,
    definitions: &'a TypeDefinitions,
    census: &'a mut ImplCensus,
}

impl<'ast> Visit<'ast> for ShapeVisitor<'_> {
    fn visit_item_impl(&mut self, implementation: &'ast ItemImpl) {
        let inherent_type = implementation
            .trait_
            .is_none()
            .then(|| FunctionCollector::type_name(implementation))
            .flatten();
        if let Some(name) = inherent_type {
            let key = self.definitions.key(self.source, &name);
            self.census
                .record_impl(&key, self.source.file(), implementation);
        }
        visit::visit_item_impl(self, implementation);
    }

    fn visit_item_struct(&mut self, definition: &'ast ItemStruct) {
        let key = format!("{}::{}", self.source.file(), definition.ident);
        self.census.record_struct(key, definition.fields.len());
        visit::visit_item_struct(self, definition);
    }
}

#[cfg(test)]
mod tests {
    use super::TypeShapes;
    use crate::parsed_source::ParsedSource;

    struct Sources;

    impl Sources {
        fn shapes(files: &[(&str, &str)]) -> TypeShapes {
            let sources: Vec<ParsedSource> = files
                .iter()
                .map(|(file, text)| ParsedSource::sample(file, text))
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
}
