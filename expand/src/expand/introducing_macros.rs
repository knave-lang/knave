use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::binder_reach::BinderReach;
use super::macro_origin::MacroOrigin;
use crate::core::BindingId;
use crate::reader::{Form, Symbol};
use crate::scope::Scope;
use crate::source::Span;

#[derive(Default)]
pub struct IntroducingMacros {
    expansions: FxHashMap<Scope, Expansion>,
    generated: FxHashSet<BindingId>,
}

struct Expansion {
    origin: MacroOrigin,
    of_generated_macro: bool,
}

impl IntroducingMacros {
    pub fn define(&mut self, binding: BindingId, definition: &Form) {
        if self.written_by_a_template(definition) {
            self.generated.insert(binding);
        }
    }

    pub fn record(&mut self, introduction: Scope, binding: BindingId, origin: &MacroOrigin) {
        let expansion = Expansion {
            origin: origin.clone(),
            of_generated_macro: self.generated.contains(&binding),
        };
        self.expansions.insert(introduction, expansion);
    }

    pub fn reach_of(&self, binder: &Symbol, written: Span, call: Span) -> Option<BinderReach> {
        let writer = self.writer_of(binder)?;
        if !writer.origin.in_project {
            return None;
        }
        if writer.of_generated_macro {
            return Some(BinderReach::Written { binder: written });
        }
        Some(BinderReach::InMacro {
            binder: written,
            call,
            introducing_macro: Rc::clone(&writer.origin.name),
        })
    }

    fn writer_of(&self, binder: &Symbol) -> Option<&Expansion> {
        self.expansions
            .iter()
            .filter(|(introduction, _)| binder.scopes().contains(**introduction))
            .min_by_key(|(introduction, _)| **introduction)
            .map(|(_, expansion)| expansion)
    }

    fn written_by_a_template(&self, form: &Form) -> bool {
        match (form.as_symbol(), form.as_list()) {
            (Some(symbol), _) => symbol.scopes().any(|scope| self.is_template_scope(scope)),
            (None, Some(items)) => items.iter().any(|item| self.written_by_a_template(item)),
            (None, None) => false,
        }
    }

    fn is_template_scope(&self, scope: Scope) -> bool {
        self.expansions
            .get(&scope)
            .is_some_and(|expansion| !expansion.origin.through_prelude)
    }
}
