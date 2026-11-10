use super::witness::Witness;
use crate::core::{BindingArena, BindingId};
use crate::types::enums::EnumTable;

pub(in crate::types) struct WitnessText<'a> {
    bindings: &'a BindingArena,
    enums: &'a EnumTable,
}

impl<'a> WitnessText<'a> {
    pub(in crate::types) fn new(bindings: &'a BindingArena, enums: &'a EnumTable) -> Self {
        Self { bindings, enums }
    }

    pub(super) fn show(&self, witness: &Witness) -> String {
        match witness {
            Witness::Wild => "_".to_string(),
            Witness::Bool(value) => value.to_string(),
            Witness::Struct { owner, arguments } => {
                self.applied(self.bindings.get(*owner).name(), arguments)
            }
            Witness::Variant {
                owner,
                index,
                arguments,
            } => {
                let name = self.variant_name(*owner, *index);
                if arguments.is_empty() {
                    return name.to_string();
                }
                self.applied(&name, arguments)
            }
        }
    }

    fn variant_name(&self, owner: BindingId, index: usize) -> std::rc::Rc<str> {
        self.enums
            .get(owner)
            .and_then(|definition| definition.variants.get(index))
            .map(|variant| variant.name.clone())
            .unwrap_or_default()
    }

    fn applied(&self, name: &str, arguments: &[Witness]) -> String {
        let parts: Vec<String> = arguments
            .iter()
            .map(|argument| self.show(argument))
            .collect();
        format!("({name} {})", parts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::WitnessText;
    use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind, ModuleId};
    use crate::source::{SourceId, Span};
    use crate::types::coverage::witness::Witness;
    use crate::types::enums::{EnumDefinition, EnumTable, VariantDefinition, VariantPayload};

    struct TextFixture {
        bindings: BindingArena,
        enums: EnumTable,
        shape: BindingId,
        point: BindingId,
    }

    impl TextFixture {
        fn new() -> Self {
            let mut bindings = BindingArena::default();
            let shape = bindings.add(BindingInfo::new(
                "shape".into(),
                BindingKind::Enum(ModuleId::new(0)),
                None,
            ));
            let point = bindings.add(BindingInfo::new(
                "point".into(),
                BindingKind::Struct(ModuleId::new(0)),
                None,
            ));
            let mut enums = EnumTable::default();
            enums.add(shape, Self::definition(&["circle", "dot"]));
            Self {
                bindings,
                enums,
                shape,
                point,
            }
        }

        fn definition(names: &[&str]) -> EnumDefinition {
            EnumDefinition {
                lifetimes: Vec::new(),
                params: Vec::new(),
                variants: names.iter().map(|name| Self::variant(name)).collect(),
                rust: None,
                definer: None,
                derives: Vec::new(),
                derived_traits: Vec::new(),
                blocked: Vec::new(),
                written_derive: None,
            }
        }

        fn variant(name: &str) -> VariantDefinition {
            VariantDefinition {
                name: name.into(),
                payload: VariantPayload::None,
                payload_spans: Vec::new(),
                function_spans: Vec::new(),
                span: Span::new(SourceId::new(0), 0..0),
            }
        }

        fn show(&self, witness: &Witness) -> String {
            WitnessText::new(&self.bindings, &self.enums).show(witness)
        }

        fn shape_variant(&self, index: usize, arguments: Vec<Witness>) -> Witness {
            Witness::Variant {
                owner: self.shape,
                index,
                arguments,
            }
        }
    }

    #[test]
    fn a_wildcard_shows_as_an_underscore() {
        assert_eq!(TextFixture::new().show(&Witness::Wild), "_");
    }

    #[test]
    fn a_bool_shows_its_value() {
        assert_eq!(TextFixture::new().show(&Witness::Bool(false)), "false");
    }

    #[test]
    fn a_variant_without_arguments_shows_its_bare_name() {
        let fixture = TextFixture::new();
        assert_eq!(fixture.show(&fixture.shape_variant(1, Vec::new())), "dot");
    }

    #[test]
    fn a_variant_with_arguments_shows_as_an_application() {
        let fixture = TextFixture::new();
        let witness = fixture.shape_variant(0, vec![Witness::Wild, Witness::Bool(true)]);
        assert_eq!(fixture.show(&witness), "(circle _ true)");
    }

    #[test]
    fn a_struct_shows_its_name_applied_to_its_fields() {
        let fixture = TextFixture::new();
        let witness = Witness::Struct {
            owner: fixture.point,
            arguments: vec![fixture.shape_variant(1, Vec::new()), Witness::Wild],
        };
        assert_eq!(fixture.show(&witness), "(point dot _)");
    }
}
