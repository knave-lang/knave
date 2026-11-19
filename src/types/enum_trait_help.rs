use std::rc::Rc;

use super::EnumDefinition;
use super::explanation_sources::ExplanationSources;
use super::problems::TypeProblems;
use super::trait_help::TraitHelp;
use super::ty::Bound;
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;

pub(super) struct EnumTraitHelp<'h, 'a> {
    binding: BindingId,
    definition: &'h EnumDefinition,
    bound: &'h Bound,
    sources: &'h ExplanationSources<'a>,
}

impl<'h, 'a> EnumTraitHelp<'h, 'a> {
    pub(super) fn of(
        binding: BindingId,
        bound: &'h Bound,
        sources: &'h ExplanationSources<'a>,
    ) -> Option<Self> {
        let definition = sources.declarations.types().enums.get(binding)?;
        Some(Self {
            binding,
            definition,
            bound,
            sources,
        })
    }

    fn with_derive_label(&self, diagnostic: Diagnostic) -> Diagnostic {
        let written = self.definition.written_derive;
        self.with_written_derive(diagnostic, written, TypeProblems::add_to_enum_derive_help)
    }

    fn order_help(&self, enum_name: &str) -> String {
        let derives = self.derives_with_needed(&self.definition.derived_traits);
        let variants: Vec<Rc<str>> = self
            .definition
            .variants
            .iter()
            .map(|variant| variant.name.clone())
            .collect();
        TypeProblems::enum_order_help(enum_name, &derives, &variants)
    }
}

impl TraitHelp for EnumTraitHelp<'_, '_> {
    fn needed(&self) -> &Bound {
        self.bound
    }

    fn sources(&self) -> &ExplanationSources<'_> {
        self.sources
    }

    fn with_order_help(&self, diagnostic: Diagnostic) -> Diagnostic {
        let mut diagnostic = self.with_derive_label(diagnostic);
        let enum_name = self.name(self.binding).to_string();
        let trait_name = self.needed_name();
        if self.definition.written_derive.is_none() {
            diagnostic =
                diagnostic.with_note(TypeProblems::order_not_inferred_enum_note(trait_name));
        }
        let blocking = self
            .sources
            .derive_rules
            .blocking_enum_payload(self.binding, self.bound.binding);
        match blocking {
            Some((variant, field_type)) => {
                diagnostic.with_note(TypeProblems::order_blocked_enum_note(
                    &enum_name,
                    &variant,
                    &self.sources.display.show(&field_type),
                    trait_name,
                ))
            }
            None => diagnostic.with_help(self.order_help(&enum_name)),
        }
    }

    fn with_other_help(&self, diagnostic: Diagnostic) -> Diagnostic {
        let diagnostic = self.with_derive_label(diagnostic);
        match self.block_on_needed(&self.definition.blocked) {
            Some((blocked, variant, payload)) => {
                diagnostic.with_note(TypeProblems::blocked_enum_derive_note(
                    self.name(self.binding),
                    self.name(*blocked),
                    variant,
                    &self.sources.display.show(payload),
                ))
            }
            None => diagnostic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EnumTraitHelp;
    use crate::diagnostic::Diagnostic;
    use crate::types::trait_help::TraitHelp;
    use crate::types::trait_help_fixture::TraitHelpFixture;

    const SOURCE: &str = "
        (enum rank low high)
        (enum value (number f64))
        (derive value debug)
        (defn main () (do))";

    struct EnumHelpFixture {
        fixture: TraitHelpFixture,
    }

    impl EnumHelpFixture {
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
                    EnumTraitHelp::of(binding, bound, sources)
                        .expect("the source declares the type")
                        .add_to(diagnostic)
                },
            )
        }
    }

    #[test]
    fn an_order_bound_on_an_underived_enum_lists_its_cases() {
        let diagnostic = EnumHelpFixture::new().help("rank", "ord");
        assert_eq!(
            diagnostic.notes(),
            ["`ord` is never inferred for an enum, so the enum needs a written `derive`"]
        );
        assert_eq!(
            TraitHelpFixture::added_helps(&diagnostic, "ord"),
            [
                "add `(derive rank clone copy partial-eq eq hash show ord)` to compare cases in declared order: `low`, then `high`"
            ]
        );
    }

    #[test]
    fn an_order_bound_names_the_case_that_blocks_it() {
        let diagnostic = EnumHelpFixture::new().help("value", "ord");
        assert_eq!(
            diagnostic.notes(),
            ["`value` cannot derive `ord`: case `number` holds `float`"]
        );
    }

    #[test]
    fn a_derivable_bound_on_a_written_derive_asks_to_add_it() {
        let diagnostic = EnumHelpFixture::new().help("value", "clone");
        assert_eq!(
            TraitHelpFixture::added_helps(&diagnostic, "clone"),
            ["add `clone` to the enum's `derive`"]
        );
        assert_eq!(diagnostic.labels().count(), 2);
    }
}
