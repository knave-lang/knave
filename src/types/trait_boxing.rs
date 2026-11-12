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

#[cfg(test)]
mod tests {
    use crate::types::declarations_fixture::DeclarationsFixture;

    struct BoxingFixture {
        declarations: DeclarationsFixture,
    }

    impl BoxingFixture {
        const SOURCE: &str = "
            (trait drawn (draw (fn ((ref self)) i64)))
            (trait counted (count (fn ((mut self)) i64)))
            (trait made (make (fn (i64) f64)))
            (trait consumed (area (fn (self) f64)))
            (trait copied (copy (fn ((ref self)) self)))
            (trait listed
              (type item)
              (first (fn ((ref self)) item)))
            (defn main () (do))";

        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(Self::SOURCE),
            }
        }

        fn blocker(&self, trait_name: &str) -> Option<String> {
            let trait_binding = self.declarations.trait_named(trait_name);
            self.declarations
                .trait_boxing()
                .blocker(trait_binding)
                .map(|(blocker, _)| blocker)
        }
    }

    #[test]
    fn a_trait_whose_methods_borrow_self_can_be_boxed() {
        let fixture = BoxingFixture::new();
        assert_eq!(fixture.blocker("drawn"), None);
        assert_eq!(fixture.blocker("counted"), None);
    }

    #[test]
    fn a_method_without_a_receiver_blocks_a_box() {
        let fixture = BoxingFixture::new();
        assert_eq!(
            fixture.blocker("made"),
            Some("`make` has no receiver".to_string())
        );
    }

    #[test]
    fn a_method_taking_self_by_value_blocks_a_box() {
        let fixture = BoxingFixture::new();
        assert_eq!(
            fixture.blocker("consumed"),
            Some("`area` takes `self` by value".to_string())
        );
    }

    #[test]
    fn a_method_returning_self_blocks_a_box() {
        let fixture = BoxingFixture::new();
        assert_eq!(
            fixture.blocker("copied"),
            Some("`copy` returns `self` by value".to_string())
        );
    }

    #[test]
    fn an_associated_type_blocks_a_box() {
        let fixture = BoxingFixture::new();
        assert_eq!(
            fixture.blocker("listed"),
            Some("it declares the associated type `item`".to_string())
        );
    }

    #[test]
    fn a_trait_the_program_does_not_define_has_no_blocker() {
        let fixture = BoxingFixture::new();
        assert_eq!(fixture.blocker("debug"), None);
    }
}
