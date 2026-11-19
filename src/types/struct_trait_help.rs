use std::rc::Rc;

use super::StructDefinition;
use super::explanation_sources::ExplanationSources;
use super::problems::TypeProblems;
use super::trait_help::TraitHelp;
use super::ty::{Bound, GenericId};
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;

pub(super) struct StructTraitHelp<'h, 'a> {
    binding: BindingId,
    definition: &'h StructDefinition,
    bound: &'h Bound,
    sources: &'h ExplanationSources<'a>,
}

impl<'h, 'a> StructTraitHelp<'h, 'a> {
    pub(super) fn of(
        binding: BindingId,
        bound: &'h Bound,
        sources: &'h ExplanationSources<'a>,
    ) -> Option<Self> {
        let definition = sources.declarations.types().structs.get(binding)?;
        Some(Self {
            binding,
            definition,
            bound,
            sources,
        })
    }

    fn order_derive_help(&self, struct_name: &str) -> String {
        let fields: Vec<Rc<str>> = self
            .definition
            .fields
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        let derives = self.derives_with_needed(&self.definition.derived_traits);
        TypeProblems::order_derive_help(struct_name, &derives, &fields)
    }

    fn first_ord_field(&self) -> Option<&'h str> {
        let params: Vec<GenericId> = self
            .definition
            .params
            .iter()
            .map(|param| param.id)
            .collect();
        let traits = &self.sources.declarations.traits().table;
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
}

impl TraitHelp for StructTraitHelp<'_, '_> {
    fn needed(&self) -> &Bound {
        self.bound
    }

    fn sources(&self) -> &ExplanationSources<'_> {
        self.sources
    }

    fn with_order_help(&self, diagnostic: Diagnostic) -> Diagnostic {
        let struct_name = self.name(self.binding);
        let trait_name = self.needed_name();
        let diagnostic = match self.definition.written_derive {
            Some(span) => diagnostic.with_label(TypeProblems::derive_leaves_out(span, trait_name)),
            None => diagnostic.with_note(TypeProblems::order_not_inferred_note(trait_name)),
        };
        let sortable = self.first_ord_field();
        match self
            .sources
            .derive_rules
            .blocking_field(self.binding, self.bound.binding)
        {
            Some((field, field_type)) => diagnostic
                .with_note(TypeProblems::order_blocked_note(
                    struct_name,
                    &field,
                    &self.sources.display.show(&field_type),
                    trait_name,
                ))
                .with_help(TypeProblems::sort_by_help(sortable)),
            None => diagnostic
                .with_help(self.order_derive_help(struct_name))
                .with_help(TypeProblems::sort_by_help(sortable)),
        }
    }

    fn with_other_help(&self, diagnostic: Diagnostic) -> Diagnostic {
        let written = self.definition.written_derive;
        let diagnostic =
            self.with_written_derive(diagnostic, written, TypeProblems::add_to_derive_help);
        match self.block_on_needed(&self.definition.blocked) {
            Some((_, field, field_type)) => {
                diagnostic.with_note(TypeProblems::blocked_derive_note(
                    self.name(self.binding),
                    self.needed_name(),
                    field,
                    &self.sources.display.show(field_type),
                ))
            }
            None => diagnostic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StructTraitHelp;
    use crate::diagnostic::Diagnostic;
    use crate::types::trait_help::TraitHelp;
    use crate::types::trait_help_fixture::TraitHelpFixture;

    const SOURCE: &str = "
        (defstruct score :points i64 :name string)
        (defstruct ticket :id u32)
        (derive ticket debug)
        (defstruct point :x f64)
        (trait shown (show (fn ((ref self)) usize)))
        (defn main () (do))";

    struct StructHelpFixture {
        fixture: TraitHelpFixture,
    }

    impl StructHelpFixture {
        fn new() -> Self {
            Self {
                fixture: TraitHelpFixture::new(SOURCE),
            }
        }

        fn help(&self, type_name: &str, trait_name: &str) -> Diagnostic {
            self.fixture.explain(
                type_name,
                trait_name,
                |binding, bound, sources, diagnostic| {
                    StructTraitHelp::of(binding, bound, sources)
                        .expect("the source declares the type")
                        .add_to(diagnostic)
                },
            )
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
            TraitHelpFixture::added_helps(&diagnostic, "ord"),
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
            TraitHelpFixture::added_helps(&diagnostic, "clone"),
            ["add `clone` to the struct's `derive`"]
        );
        assert_eq!(diagnostic.labels().count(), 2);
    }

    #[test]
    fn a_user_trait_on_a_written_derive_adds_nothing() {
        let diagnostic = StructHelpFixture::new().help("ticket", "shown");
        assert!(TraitHelpFixture::added_helps(&diagnostic, "shown").is_empty());
        assert_eq!(diagnostic.labels().count(), 1);
    }

    #[test]
    fn a_blocked_derive_names_the_field_that_blocks_it() {
        let diagnostic = StructHelpFixture::new().help("point", "eq");
        assert_eq!(
            diagnostic.notes(),
            ["`point` cannot derive `eq`: field `x` is `float`"]
        );
    }
}
