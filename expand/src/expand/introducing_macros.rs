use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::binder_site::BinderSite;
use super::expanded_call::ExpandedCall;
use super::macro_origin::MacroOrigin;
use crate::core::BindingId;
use crate::reader::Symbol;
use crate::scope::Scope;
use crate::source::{SourceMap, Span};

#[derive(Default)]
pub struct IntroducingMacros {
    expansions: FxHashMap<Scope, Expansion>,
    generators: FxHashMap<BindingId, Scope>,
}

pub struct ProjectWriter {
    name: Rc<str>,
    generator_call: Option<Span>,
}

impl ProjectWriter {
    pub fn name(&self) -> Rc<str> {
        Rc::clone(&self.name)
    }

    pub fn passed_in(&self, site: &BinderSite<'_>, sources: &SourceMap, written: Span) -> bool {
        self.generator_call
            .is_some_and(|generator_call| generator_call.contains(written))
            && site.is_spelled_at(sources, written)
    }
}

struct Expansion {
    origin: MacroOrigin,
    call_writer: Option<Scope>,
    generator: Option<Scope>,
}

impl IntroducingMacros {
    pub fn define(&mut self, binding: BindingId, definition_writer: Option<Scope>) {
        let mut writer = definition_writer;
        while let Some(expansion) = writer.and_then(|scope| self.expansions.get(&scope))
            && expansion.origin.through_prelude
        {
            writer = expansion.call_writer;
        }
        if let Some(generator) = writer {
            self.generators.insert(binding, generator);
        }
    }

    pub fn record(&mut self, introduction: Scope, call: &ExpandedCall<'_>) {
        let expansion = Expansion {
            origin: call.origin.clone(),
            call_writer: call.head_writer.filter(|writer| *writer < introduction),
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
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use super::IntroducingMacros;
    use crate::core::BindingId;
    use crate::expand::expanded_call::ExpandedCall;
    use crate::expand::origins_fixture::OriginsFixture;
    use crate::scope::{Scope, ScopeKind, Scopes};

    struct WalkFixture {
        macros: IntroducingMacros,
        scopes: Scopes,
    }

    impl WalkFixture {
        const DEADLINE: Duration = Duration::from_secs(5);
        const CALLED: BindingId = BindingId::from_index(1);
        const DEFINED: BindingId = BindingId::from_index(2);

        fn ends_within_deadline(build: fn(&mut Self) -> Option<Scope>) -> bool {
            let (finished, ended) = mpsc::channel();
            thread::spawn(move || {
                let mut fixture = Self {
                    macros: IntroducingMacros::default(),
                    scopes: Scopes::default(),
                };
                let definition_writer = build(&mut fixture);
                fixture.macros.define(Self::DEFINED, definition_writer);
                finished.send(()).ok();
            });
            ended.recv_timeout(Self::DEADLINE).is_ok()
        }

        fn fresh(&mut self) -> Scope {
            self.scopes.fresh(ScopeKind::Introduction)
        }

        fn record_prelude_call(&mut self, introduction: Scope, head_writer: Scope) {
            let origin =
                OriginsFixture::joined_origin("prelude-macro", OriginsFixture::call_site());
            let call = ExpandedCall {
                binding: Self::CALLED,
                head_writer: Some(head_writer),
                origin: &origin,
            };
            self.macros.record(introduction, &call);
        }
    }

    #[test]
    fn a_call_written_by_its_own_expansion_ends_the_walk() {
        assert!(WalkFixture::ends_within_deadline(|fixture| {
            let expansion = fixture.fresh();
            fixture.record_prelude_call(expansion, expansion);
            Some(expansion)
        }));
    }

    #[test]
    fn a_call_written_by_a_newer_expansion_ends_the_walk() {
        assert!(WalkFixture::ends_within_deadline(|fixture| {
            let older = fixture.fresh();
            let newer = fixture.fresh();
            fixture.record_prelude_call(older, newer);
            fixture.record_prelude_call(newer, older);
            Some(newer)
        }));
    }
}
