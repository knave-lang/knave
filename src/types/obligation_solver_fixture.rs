use rustc_hash::FxHashMap;

use super::declarations_fixture::DeclarationsFixture;
use super::implied_bounds::ImpliedBounds;
use super::obligation_pass::ObligationPass;
use super::obligation_solver::ObligationSolver;
use super::solver_event::SolverEvent;
use super::solver_sources::SolverSources;
use super::solver_state::SolverState;
use super::ty::{Bound, GenericId, GenericParam, Type};
use super::unifier::{Obligation, ObligationSite, Unifier};
use crate::core::PrimitiveType;
use crate::source::{SourceId, Span};

pub(super) struct ObligationSolverFixture {
    declarations: DeclarationsFixture,
    unifier: Unifier,
    implied_bounds: ImpliedBounds,
    generics: FxHashMap<GenericId, GenericParam>,
}

impl ObligationSolverFixture {
    const SOURCE: &'static str = "
        (defstruct point :x int)
        (defn same? (a b) (= a b))
        (defn main ()
          (println (same? 1 2))
          (println (= (vec 1) (vec 2)))
          (println 3))";

    pub(super) fn new() -> Self {
        Self {
            declarations: DeclarationsFixture::new(Self::SOURCE),
            unifier: Unifier::default(),
            implied_bounds: ImpliedBounds::default(),
            generics: FxHashMap::default(),
        }
    }

    pub(super) fn discharge(
        &mut self,
        obligations: Vec<Obligation>,
        pass: ObligationPass,
    ) -> Vec<SolverEvent> {
        let sources = SolverSources {
            declarations: self.declarations.declarations(),
            generics: &self.generics,
        };
        let state = SolverState {
            unifier: &mut self.unifier,
            implied_bounds: &mut self.implied_bounds,
        };
        ObligationSolver::new(state, sources, pass).discharge_all(obligations)
    }

    pub(super) fn kinds(events: &[SolverEvent]) -> Vec<&'static str> {
        events
            .iter()
            .map(|event| match event {
                SolverEvent::MissingTrait { .. } => "missing trait",
                SolverEvent::StructuralUse { .. } => "structural use",
                SolverEvent::UnprovenBound { .. } => "unproven bound",
            })
            .collect()
    }

    pub(super) fn obligation(&self, bound: Bound, implementor: Type) -> Obligation {
        Obligation {
            bound,
            implementor,
            site: ObligationSite { span: Self::span() },
        }
    }

    pub(super) fn trait_bound(&self, name: &str) -> Bound {
        Bound::plain(self.declarations.trait_named(name))
    }

    pub(super) fn trait_bound_with(&self, name: &str, argument: Type) -> Bound {
        Bound::with_argument(self.declarations.trait_named(name), argument)
    }

    pub(super) fn point(&self) -> Type {
        Type::Named(self.declarations.type_named("point"), Vec::new())
    }

    pub(super) fn vec_of(&self, element: Type) -> Type {
        Type::Named(self.declarations.rust_type_named("vec"), vec![element])
    }

    pub(super) fn int() -> Type {
        Type::Primitive(PrimitiveType::I64)
    }

    pub(super) fn boolean() -> Type {
        Type::Primitive(PrimitiveType::Bool)
    }

    pub(super) fn var(&mut self) -> Type {
        self.unifier.fresh()
    }

    pub(super) fn bind(&mut self, var: &Type, value: Type) {
        let bound = self.unifier.unify(var, &value, Self::span());
        assert!(bound.is_ok(), "a fresh variable takes any type");
    }

    pub(super) fn bounds_of(&self, var: &Type) -> Vec<Bound> {
        let Type::Var(var) = var else {
            panic!("bounds_of takes a variable")
        };
        self.unifier.bounds(*var).to_vec()
    }

    pub(super) fn unbounded_generic(&mut self) -> Type {
        let generic = GenericId::new(0);
        self.unifier.register_generic(generic, Vec::new());
        self.generics.insert(
            generic,
            GenericParam {
                id: generic,
                name: "T".into(),
                bounds: Vec::new(),
            },
        );
        Type::Generic(generic)
    }

    pub(super) fn deferred_count(&mut self) -> usize {
        self.unifier.take_obligations().len()
    }

    pub(super) fn structural_argument(events: &[SolverEvent]) -> Option<Type> {
        events.iter().find_map(|event| match event {
            SolverEvent::StructuralUse { bound, .. } => bound.argument.clone(),
            _ => None,
        })
    }

    fn span() -> Span {
        Span::new(SourceId::new(0), 0..0)
    }
}
