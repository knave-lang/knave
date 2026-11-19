use std::rc::Rc;

use super::RustTrait;
use super::derivable_traits::DerivableTraits;
use super::explanation_sources::ExplanationSources;
use super::problems::TypeProblems;
use super::ty::{Bound, Type};
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub(super) trait TraitHelp {
    fn needed(&self) -> &Bound;

    fn sources(&self) -> &ExplanationSources<'_>;

    fn with_order_help(&self, diagnostic: Diagnostic) -> Diagnostic;

    fn with_other_help(&self, diagnostic: Diagnostic) -> Diagnostic;

    fn add_to(&self, diagnostic: Diagnostic) -> Diagnostic {
        let traits = &self.sources().declarations.traits().table;
        if traits
            .get(self.needed().binding)
            .is_some_and(RustTrait::is_order)
        {
            self.with_order_help(diagnostic)
        } else {
            self.with_other_help(diagnostic)
        }
    }

    fn name(&self, binding: BindingId) -> &str {
        self.sources().bindings.get(binding).name()
    }

    fn needed_name(&self) -> &str {
        self.name(self.needed().binding)
    }

    fn derives_with_needed(&self, derived: &[BindingId]) -> Vec<String> {
        let mut names: Vec<String> = derived
            .iter()
            .map(|binding| self.name(*binding).to_string())
            .collect();
        names.push(self.needed_name().to_string());
        names
    }

    fn block_on_needed<'b>(
        &self,
        blocked: &'b [(BindingId, Rc<str>, Type)],
    ) -> Option<&'b (BindingId, Rc<str>, Type)> {
        blocked
            .iter()
            .find(|(trait_binding, _, _)| *trait_binding == self.needed().binding)
    }

    fn with_written_derive(
        &self,
        diagnostic: Diagnostic,
        written: Option<Span>,
        add_help: fn(&str) -> String,
    ) -> Diagnostic {
        let Some(span) = written else {
            return diagnostic;
        };
        let derivable = DerivableTraits::new(self.sources().declarations.traits());
        if !derivable.is_derivable(self.needed().binding) {
            return diagnostic;
        }
        let trait_name = self.needed_name();
        diagnostic
            .with_label(TypeProblems::derive_leaves_out(span, trait_name))
            .with_help(add_help(trait_name))
    }
}
