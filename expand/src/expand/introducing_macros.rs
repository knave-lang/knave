use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::binder_reach::BinderReach;
use super::macro_origin::MacroOrigin;
use crate::reader::Symbol;
use crate::scope::Scope;
use crate::source::Span;

#[derive(Default)]
pub struct IntroducingMacros {
    expansions: FxHashMap<Scope, MacroOrigin>,
}

impl IntroducingMacros {
    pub fn record(&mut self, introduction: Scope, origin: &MacroOrigin) {
        self.expansions.insert(introduction, origin.clone());
    }

    pub fn reach_of(&self, binder: &Symbol, written: Span, call: Span) -> Option<BinderReach> {
        let (introduction, writer) = self.writer_of(binder)?;
        if !writer.in_project {
            return None;
        }
        if self.passed_in_before(introduction, written) {
            return Some(BinderReach::Written { binder: written });
        }
        Some(BinderReach::InMacro {
            binder: written,
            call,
            introducing_macro: Rc::clone(&writer.name),
        })
    }

    fn writer_of(&self, binder: &Symbol) -> Option<(Scope, &MacroOrigin)> {
        self.expansions
            .iter()
            .filter(|(introduction, _)| binder.scopes().contains(**introduction))
            .min_by_key(|(introduction, _)| **introduction)
            .map(|(introduction, origin)| (*introduction, origin))
    }

    fn passed_in_before(&self, writer: Scope, written: Span) -> bool {
        self.expansions.iter().any(|(introduction, origin)| {
            *introduction < writer && !origin.through_prelude && origin.call_site.contains(written)
        })
    }
}
