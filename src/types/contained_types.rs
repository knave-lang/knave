use rustc_hash::FxHashMap;

use crate::core::{BindingId, ItemKind, NamedTypes};
use crate::expand::Program;

pub(super) struct ContainedTypes {
    contained: FxHashMap<BindingId, Vec<BindingId>>,
}

impl ContainedTypes {
    pub(super) fn of(program: &Program) -> Self {
        let definitions = program.modules.iter().flat_map(|module| &module.items);
        let contained = definitions
            .filter_map(|item| Self::contained_by(&item.kind))
            .collect();
        Self { contained }
    }

    pub(super) fn in_type(&self, binding: BindingId) -> &[BindingId] {
        self.contained
            .get(&binding)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    fn contained_by(kind: &ItemKind) -> Option<(BindingId, Vec<BindingId>)> {
        let (ItemKind::Struct(binding, ..) | ItemKind::Enum(binding, ..)) = kind else {
            return None;
        };
        let mut contained = Vec::new();
        NamedTypes::of(kind).visit(&mut |named| contained.push(named));
        Some((*binding, contained))
    }
}

#[cfg(test)]
mod tests {
    use super::ContainedTypes;
    use crate::core::{BindingId, ItemKind};
    use crate::types::typed_source::TypedSource;

    struct ContainedFixture {
        source: TypedSource,
    }

    impl ContainedFixture {
        const SOURCE: &str = "
            (defstruct (b0 a) :value a)
            (defenum (b1 a) (wrap (b0 a)))
            (defstruct (b2 a) :inner (b1 a))
            (defn main () (do))";

        fn new() -> Self {
            Self {
                source: TypedSource::expand(Self::SOURCE),
            }
        }

        fn contained_in(&self, name: &str) -> Vec<String> {
            let program = self.source.program();
            ContainedTypes::of(program)
                .in_type(self.type_named(name))
                .iter()
                .map(|binding| program.bindings.get(*binding).name().to_string())
                .collect()
        }

        fn type_named(&self, name: &str) -> BindingId {
            let program = self.source.program();
            program
                .modules
                .iter()
                .flat_map(|module| &module.items)
                .filter_map(|item| match &item.kind {
                    ItemKind::Struct(binding, ..) | ItemKind::Enum(binding, ..) => Some(*binding),
                    _ => None,
                })
                .find(|binding| program.bindings.get(*binding).name() == name)
                .expect("the source declares the type")
        }
    }

    #[test]
    fn an_enum_contains_the_type_its_variant_holds() {
        assert!(
            ContainedFixture::new()
                .contained_in("b1")
                .contains(&"b0".to_string())
        );
    }

    #[test]
    fn a_struct_contains_the_type_its_field_holds() {
        assert!(
            ContainedFixture::new()
                .contained_in("b2")
                .contains(&"b1".to_string())
        );
    }

    #[test]
    fn a_struct_that_holds_only_its_parameter_contains_no_declared_type() {
        let contained = ContainedFixture::new().contained_in("b0");
        assert!(!contained.iter().any(|name| name.starts_with('b')));
    }
}
