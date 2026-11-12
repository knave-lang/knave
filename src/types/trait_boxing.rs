use super::receiver_kind::ReceiverKind;
use super::trait_declarations::TraitDeclarations;
use super::typed_trait::TypedTrait;
use crate::core::{BindingArena, BindingId};

pub(super) struct TraitBoxing<'a> {
    traits: &'a TraitDeclarations,
    bindings: &'a BindingArena,
}

impl<'a> TraitBoxing<'a> {
    pub(super) fn new(traits: &'a TraitDeclarations, bindings: &'a BindingArena) -> Self {
        Self { traits, bindings }
    }

    pub(super) fn blocker(&self, trait_binding: BindingId) -> Option<(String, &'static str)> {
        let definition = self
            .traits
            .definitions
            .iter()
            .find(|definition| definition.binding == trait_binding)?;
        if let Some(associated) = self
            .traits
            .table
            .get(trait_binding)
            .and_then(|rust_trait| rust_trait.associated.clone())
        {
            return Some((
                format!("it declares the associated type `{associated}`"),
                "a trait with an associated type cannot be a trait object",
            ));
        }
        self.method_blocker(definition)
    }

    fn method_blocker(&self, definition: &TypedTrait) -> Option<(String, &'static str)> {
        for method in &definition.methods {
            let name = self.bindings.get(method.binding).name().to_string();
            if method.returns_self {
                return Some((
                    format!("`{name}` returns `self` by value"),
                    "return a reference or another type, not the implementor's own type",
                ));
            }
            match method.receiver {
                Some(ReceiverKind::Shared | ReceiverKind::Mutable) => continue,
                Some(ReceiverKind::Owned) => {
                    return Some((
                        format!("`{name}` takes `self` by value"),
                        "take the receiver by reference, as `(ref self)` or `(mut self)`",
                    ));
                }
                None => {
                    return Some((
                        format!("`{name}` has no receiver"),
                        "give the method a `(ref self)` or `(mut self)` receiver",
                    ));
                }
            }
        }
        None
    }
}
