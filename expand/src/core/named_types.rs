use super::{BindingId, ItemKind, TypeBound, TypeExpr, VariantPayload};

pub struct NamedTypes<'i> {
    kind: &'i ItemKind,
}

impl<'i> NamedTypes<'i> {
    pub fn of(kind: &'i ItemKind) -> Self {
        Self { kind }
    }

    pub fn visit(&self, visit: &mut dyn FnMut(BindingId)) {
        match self.kind {
            ItemKind::Signature(_, signature) => Self::types([signature].into_iter(), visit),
            ItemKind::Struct(_, _, fields) => {
                Self::types(fields.iter().map(|(_, field)| field), visit);
            }
            ItemKind::Enum(_, _, variants, ..) => {
                let payloads = variants
                    .iter()
                    .flat_map(|variant| Self::payload(&variant.payload));
                Self::types(payloads, visit);
            }
            ItemKind::Derive(_, _, derived) => {
                derived.iter().for_each(|(binding, _)| visit(*binding));
            }
            _ => self.visit_declaration(visit),
        }
    }

    fn visit_declaration(&self, visit: &mut dyn FnMut(BindingId)) {
        match self.kind {
            ItemKind::Trait(_, _, _, supertraits, _, methods) => {
                Self::bounds(supertraits.iter(), visit);
                Self::types(methods.iter().map(|method| &method.signature), visit);
            }
            ItemKind::Impl(trait_binding, implementor, associated, _) => {
                visit(*trait_binding);
                Self::types([implementor].into_iter(), visit);
                Self::types(associated.iter().map(|value| &value.value), visit);
            }
            ItemKind::RustType(_, _, _, _, clauses) => {
                Self::types(clauses.iter().map(|(target, _)| target), visit);
            }
            _ => {}
        }
    }

    fn payload(payload: &VariantPayload) -> Vec<&TypeExpr> {
        match payload {
            VariantPayload::None => Vec::new(),
            VariantPayload::Positional(fields) => fields.iter().collect(),
            VariantPayload::Named(fields) => fields.iter().map(|(_, field)| field).collect(),
        }
    }

    fn types<'t>(written: impl Iterator<Item = &'t TypeExpr>, visit: &mut dyn FnMut(BindingId)) {
        for type_expr in written {
            type_expr.named_bindings().into_iter().for_each(&mut *visit);
        }
    }

    fn bounds<'b>(bounds: impl Iterator<Item = &'b TypeBound>, visit: &mut dyn FnMut(BindingId)) {
        for bound in bounds {
            bound.named_bindings().into_iter().for_each(&mut *visit);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::NamedTypes;
    use crate::core::{ItemKind, ModulePath};
    use crate::expand::Expander;
    use crate::modules::ModuleSources;
    use crate::source::SourceMap;

    struct NamedTypesFixture;

    impl NamedTypesFixture {
        const MODULE: &str = "shapes";
        const POINT: &str = "(defstruct point :x i64)\n";

        fn named_by(text: &str, kind: fn(&ItemKind) -> bool) -> Vec<String> {
            let mut sources = SourceMap::default();
            let entry = sources.add(Self::MODULE, format!("{}{text}", Self::POINT));
            let module_sources = ModuleSources::new(PathBuf::new(), PathBuf::new());
            let program = Expander::new(&mut sources, module_sources)
                .expand(ModulePath::new(Self::MODULE), entry)
                .unwrap_or_else(|_| panic!("the program expands"));
            let mut named = Vec::new();
            for item in &program.entry_module().items {
                if kind(&item.kind) {
                    NamedTypes::of(&item.kind).visit(&mut |binding| {
                        named.push(program.bindings.get(binding).name().to_string())
                    });
                }
            }
            named
        }
    }

    #[test]
    fn a_signature_names_its_types() {
        let named = NamedTypesFixture::named_by(
            "(type size (fn (point) i64))\n(defn size (p) 1)",
            |kind| matches!(kind, ItemKind::Signature(..)),
        );
        assert!(named.contains(&"point".to_string()), "{named:?}");
    }

    #[test]
    fn a_struct_names_its_field_types() {
        let named = NamedTypesFixture::named_by("(defstruct segment :start point)", |kind| {
            matches!(kind, ItemKind::Struct(..))
        });
        assert!(named.contains(&"point".to_string()), "{named:?}");
    }

    #[test]
    fn an_enum_names_its_payload_types() {
        let named = NamedTypesFixture::named_by("(defenum place (at point) nowhere)", |kind| {
            matches!(kind, ItemKind::Enum(..))
        });
        assert!(named.contains(&"point".to_string()), "{named:?}");
    }

    #[test]
    fn a_derive_names_its_traits() {
        let named = NamedTypesFixture::named_by("(derive point debug)", |kind| {
            matches!(kind, ItemKind::Derive(..))
        });
        assert!(named.contains(&"debug".to_string()), "{named:?}");
    }

    #[test]
    fn a_trait_names_its_method_types() {
        let named =
            NamedTypesFixture::named_by("(trait placed (place (fn ((ref self)) point)))", |kind| {
                matches!(kind, ItemKind::Trait(..))
            });
        assert!(named.contains(&"point".to_string()), "{named:?}");
    }

    #[test]
    fn a_trait_names_its_supertraits() {
        let named = NamedTypesFixture::named_by(
            "(trait (ranked (ord))\n  (rank (fn ((ref self)) i64)))",
            |kind| matches!(kind, ItemKind::Trait(..)),
        );
        assert!(named.contains(&"ord".to_string()), "{named:?}");
    }

    #[test]
    fn an_impl_names_its_trait_and_implementor() {
        let named = NamedTypesFixture::named_by(
            "(trait sized (size (fn ((ref self)) i64)))\n(impl sized point (size (p) 1))",
            |kind| matches!(kind, ItemKind::Impl(..)),
        );
        assert!(named.contains(&"sized".to_string()), "{named:?}");
        assert!(named.contains(&"point".to_string()), "{named:?}");
    }

    #[test]
    fn a_rust_type_names_its_clause_targets() {
        let named = NamedTypesFixture::named_by(
            "(rust-type (wrapped item) \"Vec<{0}>\" (ord \"Vec<{0}>\"))",
            |kind| matches!(kind, ItemKind::RustType(..)),
        );
        assert!(named.contains(&"ord".to_string()), "{named:?}");
    }
}
