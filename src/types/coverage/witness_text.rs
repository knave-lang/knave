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
