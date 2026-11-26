use std::rc::Rc;

use super::bound_only_problems::{BoundOnlyProblems, NamedUnheld};
use super::given_holders::GivenHolders;
use super::held_binders::HeldBinders;
use super::implementor::Implementor;
use super::lowered_associated_value::LoweredAssociatedValue;
use super::lowered_impl::LoweredImpl;
use super::result_borrow_source::ResultBorrowSource;
use super::scheme_lowering::SchemeLowering;
use super::signature_owner::SignatureOwner;
use super::signature_scheme::SignatureScheme;
use super::text_place::TextPlace;
use super::trait_arguments::TraitArguments;
use super::trait_declarations::TraitDeclarations;
use super::ty::Scheme;
use super::unheld_at::UnheldAt;
use super::unheld_binder::UnheldBinder;
use super::written_type_facts::WrittenTypeFacts;
use super::written_type_lowering::WrittenTypeLowering;
use super::written_type_variables::WrittenTypeVariables;
use crate::core::{
    BindingArena, BindingId, ImplAssociatedType, ParsedTemplate, RustTemplate,
    RustTraitImplementor, TypeExpr, TypeExprKind,
};
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub(super) struct DefinitionSignatures<'l> {
    schemes: SchemeLowering<'l>,
    arguments: TraitArguments<'l>,
    bindings: &'l BindingArena,
    traits: &'l TraitDeclarations,
}

struct Signature<'s> {
    type_expr: &'s TypeExpr,
    name: Span,
    given: GivenHolders,
    owner: SignatureOwner,
    template_names: Vec<String>,
    clause_names: Vec<String>,
}

impl<'l> DefinitionSignatures<'l> {
    pub(super) fn new((types, facts): (WrittenTypeLowering<'l>, WrittenTypeFacts<'l>)) -> Self {
        let traits = facts.declarations.traits();
        Self {
            schemes: SchemeLowering::new(types, WrittenTypeVariables::new(facts.bindings)),
            arguments: TraitArguments::of(&traits.table),
            bindings: facts.bindings,
            traits,
        }
    }

    pub(super) fn lower_signature(
        &mut self,
        type_expr: &TypeExpr,
        binding: BindingId,
    ) -> Result<SignatureScheme, Diagnostic> {
        let scheme = self.schemes.lower_scheme(type_expr)?;
        self.check_held_binders(scheme, |signatures| {
            signatures.signature_of(type_expr, binding)
        })
    }

    pub(super) fn lower_declared_template(
        &mut self,
        template: &RustTemplate,
        binding: BindingId,
    ) -> Result<SignatureScheme, Diagnostic> {
        let scheme = self.schemes.lower_scheme(&template.type_expr)?;
        ResultBorrowSource::declared(&template.type_expr, binding, self.traits)?;
        self.check_held_binders(scheme, |signatures| Signature {
            owner: SignatureOwner::Template,
            template_names: Self::type_placeholders(std::iter::once(&template.template)),
            clause_names: Self::type_placeholders(
                template.clauses.iter().map(|clause| &clause.template),
            ),
            ..signatures.signature_of(&template.type_expr, binding)
        })
    }

    pub(super) fn lower_inline_template(
        &mut self,
        template: &RustTemplate,
    ) -> Result<SignatureScheme, Diagnostic> {
        let type_expr = &template.type_expr;
        let scheme = self.schemes.lower_template(type_expr)?;
        self.check_held_binders(scheme, |_| Signature {
            type_expr,
            name: type_expr.span(),
            given: GivenHolders {
                receiver: None,
                impl_variables: Vec::new(),
            },
            owner: SignatureOwner::Template,
            template_names: Self::type_placeholders(std::iter::once(&template.template)),
            clause_names: Vec::new(),
        })
    }

    pub(super) fn lower_implementor(
        &mut self,
        entry: &RustTraitImplementor,
        trait_name: &str,
    ) -> Result<(Implementor, Vec<LoweredAssociatedValue>), Diagnostic> {
        self.schemes.lower_implementor(entry, trait_name)
    }

    pub(super) fn lower_impl_implementor(
        &mut self,
        implementor: &TypeExpr,
        associated: &[ImplAssociatedType],
    ) -> Result<LoweredImpl, Diagnostic> {
        self.schemes.lower_impl_implementor(implementor, associated)
    }

    fn signature_of<'s>(&self, type_expr: &'s TypeExpr, binding: BindingId) -> Signature<'s> {
        let name = self.bindings.get(binding).span();
        Signature {
            type_expr,
            name: name.unwrap_or(type_expr.span()),
            given: GivenHolders {
                receiver: self.traits.methods().trait_defining(binding),
                impl_variables: self.impl_variables(binding),
            },
            owner: SignatureOwner::Function,
            template_names: Vec::new(),
            clause_names: Vec::new(),
        }
    }

    fn impl_variables(&self, binding: BindingId) -> Vec<Rc<str>> {
        let Some(position) = self.traits.impl_methods.get(&binding) else {
            return Vec::new();
        };
        let params = &self.traits.impls()[*position].params;
        params.iter().map(|param| Rc::clone(&param.name)).collect()
    }

    fn type_placeholders<'t>(texts: impl Iterator<Item = &'t Rc<str>>) -> Vec<String> {
        texts
            .filter_map(|text| ParsedTemplate::parse(text).ok())
            .flat_map(|parsed| {
                parsed
                    .type_placeholders()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn check_held_binders<'s>(
        &self,
        scheme: Scheme,
        build: impl FnOnce(&Self) -> Signature<'s>,
    ) -> Result<SignatureScheme, Diagnostic> {
        if HeldBinders::body_holds_every_binder(&scheme) {
            return Ok(SignatureScheme::held(scheme));
        }
        let signature = &build(self);
        let binders = HeldBinders::of(&scheme, &self.arguments, &signature.given);
        let unheld: Vec<UnheldAt> = (binders.unheld().into_iter())
            .filter(|unheld| signature.reports(unheld))
            .collect();
        if unheld.is_empty() {
            return Ok(SignatureScheme::held(scheme));
        }
        let named: Vec<NamedUnheld<'_>> = (unheld.iter())
            .map(|unheld| self.named_unheld(unheld, signature))
            .collect();
        let problem = BoundOnlyProblems::unheld_binders(&named, signature.name, &signature.owner);
        Ok(SignatureScheme::with_unheld(
            binders.held_scheme_without(&unheld),
            problem,
        ))
    }

    fn named_unheld<'u>(
        &'u self,
        unheld: &'u UnheldAt,
        signature: &Signature<'_>,
    ) -> NamedUnheld<'u> {
        let by_bound = signature.names_by_bound(unheld);
        let (text, position) = if by_bound {
            (None, unheld.holder_position)
        } else {
            (
                signature.text_naming(&unheld.binder),
                unheld.binder_position,
            )
        };
        NamedUnheld {
            unheld: &unheld.binder,
            trait_name: (unheld.binder.bound()).map(|bound| self.bindings.get(bound).name()),
            form: Self::binder_form(signature.type_expr, position),
            text,
        }
    }

    fn binder_form(type_expr: &TypeExpr, holder: usize) -> Span {
        let TypeExprKind::Where(binders, _) = type_expr.kind() else {
            return type_expr.span();
        };
        let mut type_binders = binders.iter().filter(|binder| !binder.lifetime);
        type_binders
            .nth(holder)
            .map_or(type_expr.span(), |binder| binder.span)
    }
}

impl Signature<'_> {
    fn reports(&self, unheld: &UnheldAt) -> bool {
        match self.owner {
            SignatureOwner::Function => true,
            SignatureOwner::Template => {
                self.names_by_bound(unheld) || self.text_naming(&unheld.binder).is_some()
            }
        }
    }

    fn names_by_bound(&self, unheld: &UnheldAt) -> bool {
        let in_argument = matches!(unheld.binder, UnheldBinder::InArgument { .. });
        match self.owner {
            SignatureOwner::Function => in_argument,
            SignatureOwner::Template => in_argument && unheld.holder_held,
        }
    }

    fn text_naming(&self, unheld: &UnheldBinder) -> Option<TextPlace> {
        let names = |names: &[String]| names.iter().any(|name| name.as_str() == &**unheld.binder());
        if names(&self.template_names) {
            return Some(TextPlace::Template);
        }
        names(&self.clause_names).then_some(TextPlace::Clause)
    }
}
