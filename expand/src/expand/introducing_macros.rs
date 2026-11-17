use std::cell::Cell;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::macro_origin::MacroOrigin;
use crate::core::BindingId;
use crate::reader::{Form, Symbol};
use crate::scope::Scope;
use crate::source::Span;

#[derive(Default)]
pub struct IntroducingMacros {
    expansions: FxHashMap<Scope, Expansion>,
    generators: FxHashMap<BindingId, Scope>,
}

pub struct MacroCall<'c> {
    pub binding: BindingId,
    pub form: &'c Form,
    pub origin: &'c MacroOrigin,
}

pub struct ProjectWriter {
    pub name: Rc<str>,
    pub generator_call: Option<Span>,
}

struct Expansion {
    origin: MacroOrigin,
    call_writer: Option<Scope>,
    generator: Option<Scope>,
}

impl IntroducingMacros {
    pub fn define(&mut self, binding: BindingId, definition: &Form) {
        let mut writer = self.latest_writer(definition);
        while let Some(expansion) = writer.and_then(|scope| self.expansions.get(&scope))
            && expansion.origin.through_prelude
        {
            writer = expansion.call_writer;
        }
        if let Some(generator) = writer {
            self.generators.insert(binding, generator);
        }
    }

    pub fn record(&mut self, introduction: Scope, call: &MacroCall<'_>) {
        let expansion = Expansion {
            origin: call.origin.clone(),
            call_writer: self.latest_writer(call.form),
            generator: self.generators.get(&call.binding).copied(),
        };
        self.expansions.insert(introduction, expansion);
    }

    pub fn project_writer_of(&self, binder: &Symbol) -> Option<ProjectWriter> {
        let writer = self.writer_of(binder)?;
        writer.origin.in_project.then(|| ProjectWriter {
            name: Rc::clone(&writer.origin.name),
            generator_call: writer
                .generator
                .and_then(|generator| self.expansions.get(&generator))
                .map(|generator| generator.origin.call_site),
        })
    }

    fn writer_of(&self, binder: &Symbol) -> Option<&Expansion> {
        self.expansions
            .iter()
            .filter(|(introduction, _)| binder.scopes().contains(**introduction))
            .min_by_key(|(introduction, _)| **introduction)
            .map(|(_, expansion)| expansion)
    }

    fn latest_writer(&self, form: &Form) -> Option<Scope> {
        let head = form.as_list()?.first()?.as_symbol()?;
        let latest = Cell::new(None);
        head.scopes().any(|scope| {
            if self.expansions.contains_key(&scope) {
                latest.set(latest.get().max(Some(scope)));
            }
            false
        });
        latest.get()
    }
}
