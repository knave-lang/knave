use std::rc::Rc;

use super::definition_kind::DefinitionKind;
use super::derive_candidate::DeriveCandidate;
use super::trait_declarations::TraitDeclarations;
use super::trait_rows_reach::Implementor;
use super::ty::Type;
use super::type_declarations::TypeDeclarations;
use crate::core::BindingId;
use crate::source::Span;

pub(super) struct DerivesBuilder<'a> {
    types: &'a mut TypeDeclarations,
    traits: &'a mut TraitDeclarations,
}

impl<'a> DerivesBuilder<'a> {
    pub(super) fn new(types: &'a mut TypeDeclarations, traits: &'a mut TraitDeclarations) -> Self {
        Self { types, traits }
    }

    pub(super) fn mark_written_derive(self, binding: BindingId, span: Span) {
        if let Some(definition) = self.types.structs.get_mut(binding) {
            definition.written_derive = Some(span);
        }
        if let Some(definition) = self.types.enums.get_mut(binding) {
            definition.written_derive = Some(span);
        }
    }

    pub(super) fn add_implementor(self, trait_binding: BindingId, implementor: Implementor) {
        if let Some(rows) = self.traits.table.reached_rows_mut(trait_binding) {
            rows.add(implementor);
        }
    }

    pub(super) fn remove_implementor(self, trait_binding: BindingId, binding: BindingId) {
        if let Some(rows) = self.traits.table.reached_rows_mut(trait_binding) {
            rows.remove_struct(binding);
        }
    }

    pub(super) fn record_block(self, candidate: DeriveCandidate, field: Rc<str>, ty: Type) {
        let blocked = match candidate.kind {
            DefinitionKind::Struct => self
                .types
                .structs
                .get_mut(candidate.binding)
                .map(|definition| &mut definition.blocked),
            DefinitionKind::Enum => self
                .types
                .enums
                .get_mut(candidate.binding)
                .map(|definition| &mut definition.blocked),
        };
        if let Some(blocked) = blocked {
            blocked.push((candidate.trait_binding, field, ty));
        }
    }

    pub(super) fn record_derive(
        self,
        candidate: &DeriveCandidate,
        written_name: Option<&'static str>,
    ) {
        let recorded = match candidate.kind {
            DefinitionKind::Struct => self
                .types
                .structs
                .get_mut(candidate.binding)
                .map(|definition| (&mut definition.derives, &mut definition.derived_traits)),
            DefinitionKind::Enum => self
                .types
                .enums
                .get_mut(candidate.binding)
                .map(|definition| (&mut definition.derives, &mut definition.derived_traits)),
        };
        let Some((derives, derived_traits)) = recorded else {
            return;
        };
        derives.extend(written_name);
        derived_traits.push(candidate.trait_binding);
    }
}

#[cfg(test)]
mod tests {
    use crate::source::{SourceId, Span};
    use crate::types::Type;
    use crate::types::declarations_fixture::DeclarationsFixture;
    use crate::types::definition_kind::DefinitionKind;
    use crate::types::derive_candidate::DeriveCandidate;
    use crate::types::derive_source::DeriveSource;

    const SOURCE: &str = "(struct point)\n(enum mood calm angry)\n(defn main () (do))";
    const WRITTEN_AT: usize = 3;

    struct DerivesFixture {
        declarations: DeclarationsFixture,
    }

    impl DerivesFixture {
        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(SOURCE),
            }
        }

        fn candidate(&self, name: &str, kind: DefinitionKind) -> DeriveCandidate {
            DeriveCandidate {
                binding: self.declarations.type_named(name),
                kind,
                trait_binding: self.declarations.trait_named("debug"),
                source: DeriveSource::Inferred,
            }
        }

        fn span() -> Span {
            Span::new(SourceId::new(0), WRITTEN_AT..WRITTEN_AT + 1)
        }

        fn record_derive(&mut self, name: &str, kind: DefinitionKind) {
            let candidate = self.candidate(name, kind);
            let derives = self.declarations.builder().derives();
            derives.record_derive(&candidate, Some("Debug"));
        }

        fn record_block(&mut self, name: &str, kind: DefinitionKind) {
            let candidate = self.candidate(name, kind);
            let derives = self.declarations.builder().derives();
            derives.record_block(candidate, "field".into(), Type::unit());
        }
    }

    #[test]
    fn a_struct_derive_records_its_name_and_trait() {
        let mut fixture = DerivesFixture::new();
        fixture.record_derive("point", DefinitionKind::Struct);
        let point = fixture.declarations.type_named("point");
        let debug = fixture.declarations.trait_named("debug");
        let definition = fixture.declarations.types().structs.get(point).unwrap();
        assert!(definition.derives.contains(&"Debug"));
        assert!(definition.derived_traits.contains(&debug));
    }

    #[test]
    fn an_enum_derive_records_its_name_and_trait() {
        let mut fixture = DerivesFixture::new();
        fixture.record_derive("mood", DefinitionKind::Enum);
        let mood = fixture.declarations.type_named("mood");
        let debug = fixture.declarations.trait_named("debug");
        let definition = fixture.declarations.types().enums.get(mood).unwrap();
        assert!(definition.derives.contains(&"Debug"));
        assert!(definition.derived_traits.contains(&debug));
    }

    #[test]
    fn a_block_is_recorded_on_the_struct_or_the_enum() {
        let mut fixture = DerivesFixture::new();
        fixture.record_block("point", DefinitionKind::Struct);
        fixture.record_block("mood", DefinitionKind::Enum);
        let point = fixture.declarations.type_named("point");
        let mood = fixture.declarations.type_named("mood");
        let types = fixture.declarations.types();
        assert_eq!(types.structs.get(point).unwrap().blocked.len(), 1);
        assert_eq!(types.enums.get(mood).unwrap().blocked.len(), 1);
    }

    #[test]
    fn a_written_derive_is_marked_on_a_struct_and_an_enum() {
        let mut fixture = DerivesFixture::new();
        let point = fixture.declarations.type_named("point");
        let mood = fixture.declarations.type_named("mood");
        let span = DerivesFixture::span();
        fixture
            .declarations
            .builder()
            .derives()
            .mark_written_derive(point, span);
        fixture
            .declarations
            .builder()
            .derives()
            .mark_written_derive(mood, span);
        let types = fixture.declarations.types();
        assert_eq!(types.structs.get(point).unwrap().written_derive, Some(span));
        assert_eq!(types.enums.get(mood).unwrap().written_derive, Some(span));
    }
}
