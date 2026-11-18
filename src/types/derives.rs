use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::checker::TypeChecker;
use super::definition_kind::DefinitionKind;
use super::derive_candidate::DeriveCandidate;
use super::derive_catalog::DeriveCatalog;
use super::derive_source::DeriveSource;
use super::derive_target::DeriveTarget;
use super::parameter_names::ParameterNames;
use super::problems::TypeProblems;
use super::trait_rows_reach::Implementor;
use super::ty::{Bound, GenericParam, Scheme, Type};
use super::written_derive::WrittenDerive;
use crate::core::{BindingId, CoreModule, ItemKind};
use crate::source::Span;

impl TypeChecker {
    pub(super) fn register_derived_traits(&mut self, modules: &[CoreModule]) {
        let targets = DeriveTarget::all(modules);
        let catalog = DeriveCatalog::of(modules, &self.declarations.traits.structural);
        let written = self.written_derives(modules, &catalog);
        let mut candidates = self.add_derive_candidates(&targets, &catalog, &written);
        self.remove_blocked_derives(&mut candidates);
        self.record_derives(&candidates, &catalog);
    }

    fn written_derives(
        &mut self,
        modules: &[CoreModule],
        catalog: &DeriveCatalog,
    ) -> FxHashMap<BindingId, WrittenDerive> {
        let mut written: FxHashMap<BindingId, WrittenDerive> = FxHashMap::default();
        for item in modules.iter().flat_map(|module| &module.items) {
            let ItemKind::Derive(struct_binding, name_span, traits) = &item.kind else {
                continue;
            };
            if self.derive_rules().is_rust_enum(*struct_binding) {
                let enum_name = self.bindings.get(*struct_binding).name().to_string();
                self.diagnostics
                    .push(TypeProblems::derive_on_rust_type(&enum_name, *name_span));
                continue;
            }
            if let Some(first) = written.get(struct_binding) {
                let struct_name = self.bindings.get(*struct_binding).name().to_string();
                self.diagnostics.push(TypeProblems::duplicate_derive(
                    &struct_name,
                    item.span,
                    first.span,
                ));
                continue;
            }
            let listed = traits
                .iter()
                .copied()
                .filter(|(trait_binding, span)| {
                    self.accepts_derive(*trait_binding, *span, catalog)
                        && !self.report_derived_written_impl(*struct_binding, *trait_binding, *span)
                })
                .collect();
            if let Some(definition) = self.declarations.types.structs.get_mut(*struct_binding) {
                definition.written_derive = Some(item.span);
            }
            if let Some(definition) = self.declarations.types.enums.get_mut(*struct_binding) {
                definition.written_derive = Some(item.span);
            }
            written.insert(
                *struct_binding,
                WrittenDerive {
                    span: item.span,
                    traits: listed,
                },
            );
        }
        written
    }

    fn accepts_derive(
        &mut self,
        trait_binding: BindingId,
        span: Span,
        catalog: &DeriveCatalog,
    ) -> bool {
        if catalog.accepts(trait_binding) {
            return true;
        }
        let names = catalog.names(&self.bindings);
        let trait_name = self.bindings.get(trait_binding).name().to_string();
        self.diagnostics
            .push(TypeProblems::not_derivable(&trait_name, &names, span));
        false
    }

    fn add_derive_candidates(
        &mut self,
        targets: &[DeriveTarget],
        catalog: &DeriveCatalog,
        written: &FxHashMap<BindingId, WrittenDerive>,
    ) -> Vec<DeriveCandidate> {
        let mut candidates = Vec::new();
        for entry in catalog.entries() {
            for target in targets {
                let rules = self.derive_rules();
                if rules.is_rust_enum(target.binding) {
                    continue;
                }
                let source = match written.get(&target.binding) {
                    Some(derive) => catalog
                        .listing_span(derive, *entry)
                        .map(DeriveSource::Listed),
                    None => rules.unlisted_source(target.binding, *entry),
                };
                let Some(source) = source else {
                    continue;
                };
                let implementor =
                    self.derived_implementor(target.binding, target.arity, entry.binding);
                self.declarations
                    .traits
                    .table
                    .add_implementor(entry.binding, implementor);
                candidates.push(DeriveCandidate {
                    binding: target.binding,
                    kind: target.kind,
                    trait_binding: entry.binding,
                    source,
                });
            }
        }
        candidates
    }

    fn report_derived_written_impl(
        &mut self,
        head: BindingId,
        trait_binding: BindingId,
        listed: Span,
    ) -> bool {
        let Some(written) = self.declared_traits().written_impl(trait_binding, head) else {
            return false;
        };
        let trait_name = self.bindings.get(trait_binding).name().to_string();
        let implementor = self.display().show(&written.implementor);
        let diagnostic = TypeProblems::second_impl(
            &implementor,
            &trait_name,
            written.implementor_span,
            TypeProblems::derive_lists(listed, &trait_name),
        );
        self.diagnostics.push(diagnostic);
        true
    }

    fn remove_blocked_derives(&mut self, candidates: &mut Vec<DeriveCandidate>) {
        loop {
            let blocked = self.derive_rules().blocked(candidates);
            if blocked.is_empty() {
                break;
            }
            for (candidate, field, ty) in blocked {
                self.declarations
                    .traits
                    .table
                    .remove_struct_implementor(candidate.trait_binding, candidate.binding);
                candidates.retain(|kept| {
                    (kept.binding, kept.trait_binding)
                        != (candidate.binding, candidate.trait_binding)
                });
                self.report_blocked_derive(candidate, field, ty);
            }
        }
    }

    fn report_blocked_derive(&mut self, candidate: DeriveCandidate, field: Rc<str>, ty: Type) {
        match candidate.source {
            DeriveSource::Listed(span) => self.report_listed_block(candidate, &field, &ty, span),
            DeriveSource::Inferred => self.record_inferred_block(candidate, field, ty),
        }
    }

    fn report_listed_block(
        &mut self,
        candidate: DeriveCandidate,
        field: &Rc<str>,
        ty: &Type,
        span: Span,
    ) {
        let display = self.display();
        let name = self.bindings.get(candidate.binding).name();
        let trait_name = self.bindings.get(candidate.trait_binding).name();
        let diagnostic = match candidate.kind {
            DefinitionKind::Struct => {
                TypeProblems::blocked_derive(name, trait_name, field, &display.show(ty), span)
            }
            DefinitionKind::Enum => {
                TypeProblems::blocked_enum_derive(name, trait_name, field, &display.show(ty), span)
            }
        };
        self.diagnostics.push(diagnostic);
    }

    fn record_inferred_block(&mut self, candidate: DeriveCandidate, field: Rc<str>, ty: Type) {
        match candidate.kind {
            DefinitionKind::Struct => {
                if let Some(definition) = self.declarations.types.structs.get_mut(candidate.binding)
                {
                    definition
                        .blocked
                        .push((candidate.trait_binding, field, ty));
                }
            }
            DefinitionKind::Enum => {
                if let Some(definition) = self.declarations.types.enums.get_mut(candidate.binding) {
                    definition
                        .blocked
                        .push((candidate.trait_binding, field, ty));
                }
            }
        }
    }

    fn record_derives(&mut self, candidates: &[DeriveCandidate], catalog: &DeriveCatalog) {
        for candidate in candidates {
            let written_name = catalog.written_name(candidate.trait_binding);
            let (derives, derived_traits) = match candidate.kind {
                DefinitionKind::Struct => {
                    let Some(definition) =
                        self.declarations.types.structs.get_mut(candidate.binding)
                    else {
                        continue;
                    };
                    (&mut definition.derives, &mut definition.derived_traits)
                }
                DefinitionKind::Enum => {
                    let Some(definition) = self.declarations.types.enums.get_mut(candidate.binding)
                    else {
                        continue;
                    };
                    (&mut definition.derives, &mut definition.derived_traits)
                }
            };
            derives.extend(written_name);
            derived_traits.push(candidate.trait_binding);
        }
    }

    fn derived_implementor(
        &mut self,
        binding: BindingId,
        arity: usize,
        derived: BindingId,
    ) -> Implementor {
        let params: Vec<GenericParam> = (0..arity)
            .map(|position| {
                self.generic_declarations()
                    .declare(&ParameterNames::at(position), vec![Bound::plain(derived)])
            })
            .collect();
        let arguments = params.iter().map(|param| Type::Generic(param.id)).collect();
        Implementor {
            scheme: Scheme {
                lifetimes: Vec::new(),
                params,
                body: Type::Named(binding, arguments),
            },
            argument: None,
        }
    }
}
