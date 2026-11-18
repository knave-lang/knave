use std::rc::Rc;

use super::StructDefinition;
use super::derivable_traits::DerivableTraits;
use super::explanation_sources::ExplanationSources;
use super::problems::TypeProblems;
use super::trait_help::TraitHelp;
use super::ty::{Bound, GenericId};
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;

pub(super) struct StructTraitHelp<'h> {
    binding: BindingId,
    definition: &'h StructDefinition,
    bound: &'h Bound,
}

impl<'h> StructTraitHelp<'h> {
    pub(super) fn new(
        binding: BindingId,
        definition: &'h StructDefinition,
        bound: &'h Bound,
    ) -> Self {
        Self {
            binding,
            definition,
            bound,
        }
    }

    fn order_derive_help(&self, struct_name: &str, sources: &ExplanationSources<'_>) -> String {
        let fields: Vec<Rc<str>> = self
            .definition
            .fields
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        let mut derives: Vec<String> = self
            .definition
            .derived_traits
            .iter()
            .map(|binding| sources.bindings.get(*binding).name().to_string())
            .collect();
        derives.push(sources.bindings.get(self.bound.binding).name().to_string());
        TypeProblems::order_derive_help(struct_name, &derives, &fields)
    }

    fn first_ord_field(&self, sources: &ExplanationSources<'_>) -> Option<&'h str> {
        let params: Vec<GenericId> = self
            .definition
            .params
            .iter()
            .map(|param| param.id)
            .collect();
        let traits = &sources.declarations.traits().table;
        self.definition
            .fields
            .iter()
            .find(|(_, ty)| {
                traits.satisfies(&Bound::plain(self.bound.binding), ty, &|generic, _| {
                    params.contains(&generic)
                })
            })
            .map(|(name, _)| name.as_ref())
    }

    fn with_written_derive(
        &self,
        diagnostic: Diagnostic,
        sources: &ExplanationSources<'_>,
    ) -> Diagnostic {
        let Some(derive_span) = self.definition.written_derive else {
            return diagnostic;
        };
        if !DerivableTraits::new(sources.declarations.traits()).is_derivable(self.bound.binding) {
            return diagnostic;
        }
        let trait_name = sources.bindings.get(self.bound.binding).name();
        diagnostic
            .with_label(TypeProblems::derive_leaves_out(derive_span, trait_name))
            .with_help(TypeProblems::add_to_derive_help(trait_name))
    }
}

impl TraitHelp for StructTraitHelp<'_> {
    fn needed(&self) -> &Bound {
        self.bound
    }

    fn with_order_help(
        &self,
        diagnostic: Diagnostic,
        sources: &ExplanationSources<'_>,
    ) -> Diagnostic {
        let struct_name = sources.bindings.get(self.binding).name();
        let trait_name = sources.bindings.get(self.bound.binding).name();
        let diagnostic = match self.definition.written_derive {
            Some(span) => diagnostic.with_label(TypeProblems::derive_leaves_out(span, trait_name)),
            None => diagnostic.with_note(TypeProblems::order_not_inferred_note(trait_name)),
        };
        let sortable = self.first_ord_field(sources);
        match sources
            .derive_rules
            .blocking_field(self.binding, self.bound.binding)
        {
            Some((field, field_type)) => diagnostic
                .with_note(TypeProblems::order_blocked_note(
                    struct_name,
                    &field,
                    &sources.display.show(&field_type),
                    trait_name,
                ))
                .with_help(TypeProblems::sort_by_help(sortable)),
            None => diagnostic
                .with_help(self.order_derive_help(struct_name, sources))
                .with_help(TypeProblems::sort_by_help(sortable)),
        }
    }

    fn with_other_help(
        &self,
        diagnostic: Diagnostic,
        sources: &ExplanationSources<'_>,
    ) -> Diagnostic {
        let diagnostic = self.with_written_derive(diagnostic, sources);
        let blocked = self
            .definition
            .blocked
            .iter()
            .find(|(blocked, _, _)| *blocked == self.bound.binding);
        match blocked {
            Some((_, field, field_type)) => {
                diagnostic.with_note(TypeProblems::blocked_derive_note(
                    sources.bindings.get(self.binding).name(),
                    sources.bindings.get(self.bound.binding).name(),
                    field,
                    &sources.display.show(field_type),
                ))
            }
            None => diagnostic,
        }
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashSet;

    use super::StructTraitHelp;
    use crate::core::BindingId;
    use crate::diagnostic::Diagnostic;
    use crate::source::{SourceId, Span};
    use crate::types::declarations_fixture::DeclarationsFixture;
    use crate::types::problems::TypeProblems;
    use crate::types::trait_help::TraitHelp;
    use crate::types::ty::Bound;

    const SOURCE: &str = "
        (defstruct score :points i64 :name string)
        (defstruct ticket :id u32)
        (derive ticket debug)
        (trait shown (show (fn ((ref self)) usize)))
        (defn main () (do))";

    struct StructHelpFixture {
        declarations: DeclarationsFixture,
        changed_structs: FxHashSet<BindingId>,
    }

    impl StructHelpFixture {
        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(SOURCE),
                changed_structs: FxHashSet::default(),
            }
        }

        fn help(&self, struct_name: &str, trait_name: &str) -> Diagnostic {
            let binding = self.declarations.type_named(struct_name);
            let definition = self
                .declarations
                .types()
                .structs
                .get(binding)
                .expect("a struct");
            let bound = Bound::plain(self.declarations.trait_named(trait_name));
            let sources = self.declarations.explanation_sources(&self.changed_structs);
            let diagnostic = Self::unexplained(trait_name);
            StructTraitHelp::new(binding, definition, &bound).add_to(diagnostic, &sources)
        }

        fn unexplained(trait_name: &str) -> Diagnostic {
            TypeProblems::missing_bound(trait_name, "T", Span::new(SourceId::new(0), 0..0))
        }

        fn added_helps<'d>(diagnostic: &'d Diagnostic, trait_name: &str) -> Vec<&'d str> {
            let unexplained = Self::unexplained(trait_name).helps().len();
            (diagnostic.helps().iter().skip(unexplained))
                .map(|help| help.message())
                .collect()
        }
    }

    #[test]
    fn an_order_bound_on_an_underived_struct_asks_for_a_written_derive() {
        let diagnostic = StructHelpFixture::new().help("score", "ord");
        assert_eq!(
            diagnostic.notes(),
            ["`ord` is never inferred for a struct, so the struct needs a written `derive`"]
        );
        assert_eq!(
            StructHelpFixture::added_helps(&diagnostic, "ord"),
            [
                "add `(derive score clone partial-eq eq hash show ord)` to compare fields in this order: `points`, then `name`",
                "to sort by one field instead, use `sort-by!` with the field keyword `:points`",
            ]
        );
    }

    #[test]
    fn an_order_bound_on_a_written_derive_labels_the_derive() {
        let diagnostic = StructHelpFixture::new().help("ticket", "ord");
        assert!(diagnostic.notes().is_empty());
        assert_eq!(diagnostic.labels().count(), 2);
    }

    #[test]
    fn a_derivable_bound_on_a_written_derive_asks_to_add_it() {
        let diagnostic = StructHelpFixture::new().help("ticket", "clone");
        assert_eq!(
            StructHelpFixture::added_helps(&diagnostic, "clone"),
            ["add `clone` to the struct's `derive`"]
        );
        assert_eq!(diagnostic.labels().count(), 2);
    }

    #[test]
    fn a_user_trait_on_a_written_derive_adds_nothing() {
        let diagnostic = StructHelpFixture::new().help("ticket", "shown");
        assert!(StructHelpFixture::added_helps(&diagnostic, "shown").is_empty());
        assert_eq!(diagnostic.labels().count(), 1);
    }
}
