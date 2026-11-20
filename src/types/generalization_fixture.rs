use std::rc::Rc;

use rustc_hash::FxHashSet;

use super::declarations_fixture::DeclarationsFixture;
use super::generalization::Generalization;
use super::generalization_rules::GeneralizationRules;
use super::generic_registry::GenericRegistry;
use super::literal_parameters::LiteralParameters;
use super::pinnable_variables::PinnableVariables;
use super::ty::{Bound, GenericParam, Type, TypeVar};
use super::type_variables::TypeVariables;
use super::typed::{TypedBinder, TypedBlock, TypedExpr, TypedExprKind, TypedLambda};
use super::unifier::{LiteralKind, Unifier};
use super::variable_linking::VariableLinking;
use super::{ClosureFacts, ClosureKind};
use crate::core::{BindingId, PrimitiveType};
use crate::source::{SourceId, Span};

pub(super) struct GeneralizationFixture {
    declarations: DeclarationsFixture,
    unifier: Unifier,
    generics: GenericRegistry,
}

impl GeneralizationFixture {
    const SOURCE: &'static str = "
        (defn main () (println (contains? \"Ana\" \"A\")))";
    pub(super) const UNLINKING_TRAIT: &'static str = "contains";

    pub(super) fn new() -> Self {
        Self {
            declarations: DeclarationsFixture::new(Self::SOURCE),
            unifier: Unifier::default(),
            generics: GenericRegistry::default(),
        }
    }

    pub(super) fn var(&mut self) -> TypeVar {
        Self::var_of(self.unifier.fresh())
    }

    pub(super) fn literal_var(&mut self) -> TypeVar {
        Self::var_of(self.unifier.fresh_literal(LiteralKind::Integer))
    }

    pub(super) fn bind(&mut self, var: TypeVar, value: Type) {
        self.unifier.bind(var, value);
    }

    pub(super) fn bound_by(&mut self, var: TypeVar, trait_name: &str, argument: Option<Type>) {
        let binding = self.declarations.trait_named(trait_name);
        let bound = match argument {
            Some(argument) => Bound::with_argument(binding, argument),
            None => Bound::plain(binding),
        };
        self.unifier.add_bound(var, bound);
    }

    pub(super) fn bound_of(&self, trait_name: &str, argument: Option<Type>) -> Bound {
        let binding = self.declarations.trait_named(trait_name);
        match argument {
            Some(argument) => Bound::with_argument(binding, argument),
            None => Bound::plain(binding),
        }
    }

    pub(super) fn resolve(&self, var: TypeVar) -> Type {
        self.unifier.resolve(&Type::Var(var))
    }

    pub(super) fn generic_of(&self, var: TypeVar) -> Option<&GenericParam> {
        match self.resolve(var) {
            Type::Generic(id) => self.generics.param(id),
            _ => None,
        }
    }

    pub(super) fn literal_of(&self, var: TypeVar) -> Option<LiteralKind> {
        self.generic_of(var)
            .and_then(|param| self.generics.literal(param.id))
    }

    pub(super) fn variables(&self) -> TypeVariables<'_> {
        TypeVariables::new(&self.unifier)
    }

    pub(super) fn variable_linking(&mut self) -> VariableLinking<'_> {
        VariableLinking::new(&mut self.unifier, &self.declarations.traits().table)
    }

    pub(super) fn pinnable_variables(&self) -> PinnableVariables<'_> {
        PinnableVariables::new(&self.unifier)
    }

    pub(super) fn generalization(&mut self) -> Generalization<'_> {
        Generalization::new(&mut self.unifier, &mut self.generics, Self::span())
    }

    pub(super) fn rules(
        pinnable: &[TypeVar],
        literal_parameters: LiteralParameters,
    ) -> GeneralizationRules {
        let pinnable: FxHashSet<TypeVar> = pinnable.iter().copied().collect();
        GeneralizationRules::new(pinnable, literal_parameters)
    }

    pub(super) fn function(parameters: Vec<Type>, result: Type) -> TypedExpr {
        let params = (parameters.iter())
            .map(|ty| TypedBinder {
                binding: BindingId::from_index(0),
                ty: ty.clone(),
                span: Self::span(),
                annotated: false,
            })
            .collect();
        let lambda = TypedLambda {
            params,
            body: TypedBlock { items: Vec::new() },
            wraps: None,
            kind: ClosureKind::Fn,
            facts: ClosureFacts::default(),
        };
        TypedExpr {
            kind: TypedExprKind::Fn(Rc::new(lambda)),
            ty: Type::Function(ClosureKind::Fn, parameters, Box::new(result)),
            span: Self::span(),
        }
    }

    pub(super) fn int() -> Type {
        Type::Primitive(PrimitiveType::I64)
    }

    pub(super) fn member(value: &TypedExpr) -> (BindingId, &TypedExpr) {
        (BindingId::from_index(0), value)
    }

    fn var_of(value: Type) -> TypeVar {
        let Type::Var(var) = value else {
            unreachable!("fresh makes a variable")
        };
        var
    }

    pub(super) fn named_binding() -> BindingId {
        BindingId::from_index(0)
    }

    pub(super) fn span() -> Span {
        Span::new(SourceId::new(0), 0..0)
    }
}
