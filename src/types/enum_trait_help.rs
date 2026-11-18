use std::rc::Rc;

use super::EnumDefinition;
use super::derivable_traits::DerivableTraits;
use super::explanation_sources::ExplanationSources;
use super::problems::TypeProblems;
use super::trait_help::TraitHelp;
use super::ty::Bound;
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;

pub(super) struct EnumTraitHelp<'h> {
    binding: BindingId,
    definition: &'h EnumDefinition,
    bound: &'h Bound,
}

impl<'h> EnumTraitHelp<'h> {
    pub(super) fn new(
        binding: BindingId,
        definition: &'h EnumDefinition,
        bound: &'h Bound,
    ) -> Self {
        Self {
            binding,
            definition,
            bound,
        }
    }

    fn with_written_derive(
        &self,
        diagnostic: Diagnostic,
        sources: &ExplanationSources<'_>,
    ) -> Diagnostic {
        let trait_name = sources.bindings.get(self.bound.binding).name();
        let derivable = DerivableTraits::new(sources.declarations.traits());
        match self.definition.written_derive {
            Some(span) if derivable.is_derivable(self.bound.binding) => diagnostic
                .with_label(TypeProblems::derive_leaves_out(span, trait_name))
                .with_help(TypeProblems::add_to_enum_derive_help(trait_name)),
            _ => diagnostic,
        }
    }

    fn order_help(&self, enum_name: &str, sources: &ExplanationSources<'_>) -> String {
        let mut derives: Vec<String> = self
            .definition
            .derived_traits
            .iter()
            .map(|binding| sources.bindings.get(*binding).name().to_string())
            .collect();
        derives.push(sources.bindings.get(self.bound.binding).name().to_string());
        let variants: Vec<Rc<str>> = self
            .definition
            .variants
            .iter()
            .map(|variant| variant.name.clone())
            .collect();
        TypeProblems::enum_order_help(enum_name, &derives, &variants)
    }
}

impl TraitHelp for EnumTraitHelp<'_> {
    fn needed(&self) -> &Bound {
        self.bound
    }

    fn with_order_help(
        &self,
        diagnostic: Diagnostic,
        sources: &ExplanationSources<'_>,
    ) -> Diagnostic {
        let mut diagnostic = self.with_written_derive(diagnostic, sources);
        let enum_name = sources.bindings.get(self.binding).name().to_string();
        let trait_name = sources.bindings.get(self.bound.binding).name();
        if self.definition.written_derive.is_none() {
            diagnostic =
                diagnostic.with_note(TypeProblems::order_not_inferred_enum_note(trait_name));
        }
        let blocking = sources
            .derive_rules
            .blocking_enum_payload(self.binding, self.bound.binding);
        match blocking {
            Some((variant, field_type)) => {
                diagnostic.with_note(TypeProblems::order_blocked_enum_note(
                    &enum_name,
                    &variant,
                    &sources.display.show(&field_type),
                    trait_name,
                ))
            }
            None => diagnostic.with_help(self.order_help(&enum_name, sources)),
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
            Some((blocked, variant, payload)) => {
                diagnostic.with_note(TypeProblems::blocked_enum_derive_note(
                    sources.bindings.get(self.binding).name(),
                    sources.bindings.get(*blocked).name(),
                    variant,
                    &sources.display.show(payload),
                ))
            }
            None => diagnostic,
        }
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashSet;

    use super::EnumTraitHelp;
    use crate::core::BindingId;
    use crate::diagnostic::Diagnostic;
    use crate::source::{SourceId, Span};
    use crate::types::declarations_fixture::DeclarationsFixture;
    use crate::types::problems::TypeProblems;
    use crate::types::trait_help::TraitHelp;
    use crate::types::ty::Bound;

    const SOURCE: &str = "
        (enum rank low high)
        (enum value (number f64))
        (derive value debug)
        (defn main () (do))";

    struct EnumHelpFixture {
        declarations: DeclarationsFixture,
        changed_structs: FxHashSet<BindingId>,
    }

    impl EnumHelpFixture {
        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(SOURCE),
                changed_structs: FxHashSet::default(),
            }
        }

        fn help(&self, enum_name: &str, trait_name: &str) -> Diagnostic {
            let binding = self.declarations.type_named(enum_name);
            let definition = self
                .declarations
                .types()
                .enums
                .get(binding)
                .expect("an enum");
            let bound = Bound::plain(self.declarations.trait_named(trait_name));
            let sources = self.declarations.explanation_sources(&self.changed_structs);
            let diagnostic = Self::unexplained(trait_name);
            EnumTraitHelp::new(binding, definition, &bound).add_to(diagnostic, &sources)
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
    fn an_order_bound_on_an_underived_enum_lists_its_cases() {
        let diagnostic = EnumHelpFixture::new().help("rank", "ord");
        assert_eq!(
            diagnostic.notes(),
            ["`ord` is never inferred for an enum, so the enum needs a written `derive`"]
        );
        assert_eq!(
            EnumHelpFixture::added_helps(&diagnostic, "ord"),
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
            EnumHelpFixture::added_helps(&diagnostic, "clone"),
            ["add `clone` to the enum's `derive`"]
        );
        assert_eq!(diagnostic.labels().count(), 2);
    }
}
