use super::bound_proof::BoundProof;
use super::generic_bound_check::GenericBoundCheck;
use super::generic_bound_tables::GenericBoundTables;
use super::implied_bounds::ImpliedBounds;
use super::literal_settlement::LiteralSettlement;
use super::obligation_pass::ObligationPass;
use super::problems::TypeProblems;
use super::selection::{Requirement, Selection};
use super::solver_event::SolverEvent;
use super::solver_sources::SolverSources;
use super::solver_state::SolverState;
use super::ty::{Bound, GenericId, Type};
use super::type_variables::TypeVariables;
use super::unifier::{Obligation, Unifier};
use crate::source::Span;

pub(super) struct ObligationSolver<'a> {
    unifier: &'a mut Unifier,
    implied_bounds: &'a mut ImpliedBounds,
    sources: SolverSources<'a>,
    pass: ObligationPass,
    events: Vec<SolverEvent>,
}

impl<'a> ObligationSolver<'a> {
    pub(super) fn new(
        state: SolverState<'a>,
        sources: SolverSources<'a>,
        pass: ObligationPass,
    ) -> Self {
        Self {
            unifier: state.unifier,
            implied_bounds: state.implied_bounds,
            sources,
            pass,
            events: Vec::new(),
        }
    }

    pub(super) fn discharge_all(mut self, obligations: Vec<Obligation>) -> Vec<SolverEvent> {
        for obligation in obligations {
            let outer = self.unifier.inside_operand();
            self.unifier.set_inside_operand(obligation.inside_operand);
            self.discharge(obligation);
            self.unifier.set_inside_operand(outer);
        }
        self.events
    }

    fn discharge(&mut self, obligation: Obligation) {
        let bound = self.resolve_bound(&obligation.bound);
        let implementor = self.unifier.resolve(&obligation.implementor);
        match implementor {
            Type::Var(var) => self.unifier.add_bound(var, bound),
            Type::Generic(generic) => self.settle_generic(generic, bound, obligation.site.span),
            _ => self.select_implementor(Obligation {
                bound,
                implementor,
                site: obligation.site,
                inside_operand: self.unifier.inside_operand(),
            }),
        }
    }

    fn resolve_bound(&self, bound: &Bound) -> Bound {
        Bound {
            binding: bound.binding,
            argument: bound
                .argument
                .as_ref()
                .map(|argument| self.unifier.resolve(argument)),
            origins: bound.origins.clone(),
        }
    }

    fn settle_generic(&mut self, generic: GenericId, bound: Bound, span: Span) {
        if let BoundProof::Unproven(bound) = self.generic_bound_check().settle(generic, bound, span)
        {
            self.events.push(SolverEvent::UnprovenBound {
                generic,
                bound,
                span,
            });
        }
    }

    fn select_implementor(&mut self, obligation: Obligation) {
        let declarations = self.sources.declarations;
        let boxed_object = declarations.types().boxed_object(&obligation.implementor);
        let inclusions = declarations.traits().table.inclusions();
        let binding = obligation.bound.binding;
        if boxed_object.is_some_and(|object| inclusions.is_met_by(binding, object)) {
            return;
        }
        match (self.literal_settlement()).select(&obligation.bound, &obligation.implementor) {
            Selection::Implemented {
                requirements,
                argument,
            } => self.adopt_selection(obligation, requirements, argument),
            Selection::Missing if !obligation.implementor.has_vars() => self.report_missing_trait(
                &obligation.bound,
                &obligation.implementor,
                obligation.site.span,
            ),
            Selection::Undecided | Selection::Missing => self.leave_unresolved(obligation),
        }
    }

    fn adopt_selection(
        &mut self,
        obligation: Obligation,
        requirements: Vec<Requirement>,
        argument: Option<Type>,
    ) {
        let Obligation {
            bound,
            implementor,
            site,
            ..
        } = obligation;
        if let (Some(argument), Some(expected)) = (argument, &bound.argument)
            && self.unifier.unify(expected, &argument, site.span).is_err()
        {
            self.report_missing_trait(&bound, &implementor, site.span);
            return;
        }
        self.events.push(SolverEvent::StructuralUse {
            bound: bound.clone(),
            implementor: implementor.clone(),
            span: site.span,
        });
        for requirement in requirements {
            let required = Obligation {
                bound: requirement.bound,
                implementor: requirement.implementor,
                site,
                inside_operand: self.unifier.inside_operand(),
            };
            self.discharge(required);
        }
    }

    fn report_missing_trait(&mut self, bound: &Bound, implementor: &Type, span: Span) {
        let shown_argument = self.literal_settlement().shown_argument(bound);
        self.events.push(SolverEvent::MissingTrait {
            bound: bound.clone(),
            implementor: implementor.clone(),
            span,
            shown_argument,
        });
    }

    fn leave_unresolved(&mut self, obligation: Obligation) {
        match self.pass {
            ObligationPass::Deferring => self.unifier.defer(obligation),
            ObligationPass::Final if obligation.inside_operand => {}
            ObligationPass::Final => self.report_open_obligation(&obligation),
        }
    }

    fn report_open_obligation(&mut self, obligation: &Obligation) {
        let variables = TypeVariables::new(self.unifier);
        let mut open_vars = variables.free_in(&obligation.implementor);
        if let Some(argument) = &obligation.bound.argument {
            open_vars.extend(variables.free_in(argument));
        }
        let site = obligation.site;
        let label_span = site.bound_site.unwrap_or(site.span);
        self.events.push(SolverEvent::OpenObligation {
            open_vars,
            problem: TypeProblems::cannot_infer_bound(label_span),
        });
    }

    fn generic_bound_check(&mut self) -> GenericBoundCheck<'_> {
        let tables = GenericBoundTables {
            traits: &self.sources.declarations.traits().table,
            generics: self.sources.generics,
        };
        GenericBoundCheck::new(self.unifier, self.implied_bounds, tables)
    }

    fn literal_settlement(&self) -> LiteralSettlement<'_> {
        LiteralSettlement::new(self.unifier, &self.sources.declarations.traits().table)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::obligation_pass::ObligationPass;
    use crate::types::obligation_solver_fixture::ObligationSolverFixture as Fixture;

    #[test]
    fn an_open_variable_takes_the_bound() {
        let mut fixture = Fixture::new();
        let var = fixture.var();
        let obligation = fixture.obligation(fixture.trait_bound("display"), var.clone());
        let events = fixture.discharge(vec![obligation], ObligationPass::Final);
        assert!(events.is_empty());
        assert_eq!(fixture.bounds_of(&var).len(), 1);
    }

    #[test]
    fn an_implemented_trait_records_a_structural_use() {
        let mut fixture = Fixture::new();
        let obligation = fixture.obligation(fixture.trait_bound("display"), Fixture::int());
        let events = fixture.discharge(vec![obligation], ObligationPass::Final);
        assert_eq!(Fixture::kinds(&events), ["structural use"]);
    }

    #[test]
    fn a_type_without_the_trait_is_missing_it() {
        let mut fixture = Fixture::new();
        let obligation = fixture.obligation(fixture.trait_bound("display"), fixture.point());
        let events = fixture.discharge(vec![obligation], ObligationPass::Final);
        assert_eq!(Fixture::kinds(&events), ["missing trait"]);
    }

    #[test]
    fn a_generic_without_the_bound_leaves_it_unproven() {
        let mut fixture = Fixture::new();
        let generic = fixture.unbounded_generic();
        let obligation = fixture.obligation(fixture.trait_bound("display"), generic);
        let events = fixture.discharge(vec![obligation], ObligationPass::Final);
        assert_eq!(Fixture::kinds(&events), ["unproven bound"]);
    }

    #[test]
    fn a_requirement_of_the_selected_implementor_is_discharged_too() {
        let mut fixture = Fixture::new();
        let points = fixture.vec_of(fixture.point());
        let bound = fixture.trait_bound_with("partial-eq", points.clone());
        let obligation = fixture.obligation(bound, points);
        let events = fixture.discharge(vec![obligation], ObligationPass::Final);
        assert_eq!(
            Fixture::kinds(&events),
            ["structural use", "structural use"]
        );
    }

    #[test]
    fn a_selected_argument_that_does_not_fit_misses_the_trait() {
        let mut fixture = Fixture::new();
        let bound = fixture.trait_bound_with("partial-eq", Fixture::boolean());
        let obligation = fixture.obligation(bound, Fixture::int());
        let events = fixture.discharge(vec![obligation], ObligationPass::Final);
        assert_eq!(Fixture::kinds(&events), ["missing trait"]);
    }

    #[test]
    fn the_bound_argument_is_resolved_before_selection() {
        let mut fixture = Fixture::new();
        let argument = fixture.var();
        fixture.bind(&argument, Fixture::int());
        let bound = fixture.trait_bound_with("partial-eq", argument);
        let obligation = fixture.obligation(bound, Fixture::int());
        let events = fixture.discharge(vec![obligation], ObligationPass::Final);
        assert_eq!(Fixture::structural_argument(&events), Some(Fixture::int()));
    }

    #[test]
    fn an_undecided_obligation_waits_in_the_deferring_pass() {
        let mut fixture = Fixture::new();
        let element = fixture.var();
        let open = fixture.vec_of(element);
        let obligation = fixture.obligation(fixture.trait_bound("display"), open);
        let events = fixture.discharge(vec![obligation], ObligationPass::Deferring);
        assert!(events.is_empty());
        assert_eq!(fixture.deferred_count(), 1);
    }

    #[test]
    fn an_undecided_obligation_reports_its_open_variables_in_the_final_pass() {
        let mut fixture = Fixture::new();
        let element = fixture.var();
        let open = fixture.vec_of(element);
        let obligation = fixture.obligation(fixture.trait_bound("display"), open);
        let events = fixture.discharge(vec![obligation], ObligationPass::Final);
        assert_eq!(Fixture::kinds(&events), ["open obligation"]);
        assert_eq!(Fixture::open_vars(&events).len(), 1);
    }

    #[test]
    fn an_undecided_obligation_inside_an_operand_stays_silent() {
        let mut fixture = Fixture::new();
        let element = fixture.var();
        let open = fixture.vec_of(element);
        let mut obligation = fixture.obligation(fixture.trait_bound("display"), open);
        obligation.inside_operand = true;
        let events = fixture.discharge(vec![obligation], ObligationPass::Final);
        assert!(events.is_empty());
    }

    #[test]
    fn the_operand_mark_is_restored_after_each_obligation() {
        let mut fixture = Fixture::new();
        fixture.set_inside_operand(false);
        let mut obligation = fixture.obligation(fixture.trait_bound("display"), Fixture::int());
        obligation.inside_operand = true;
        fixture.discharge(vec![obligation], ObligationPass::Final);
        assert!(!fixture.inside_operand());
    }

    #[test]
    fn an_open_bound_argument_counts_among_the_open_variables() {
        let mut fixture = Fixture::new();
        let element = fixture.var();
        let argument = fixture.var();
        let bound = fixture.trait_bound_with("partial-eq", argument);
        let obligation = fixture.obligation(bound, fixture.vec_of(element));
        let events = fixture.discharge(vec![obligation], ObligationPass::Final);
        assert_eq!(Fixture::open_vars(&events).len(), 2);
    }

    #[test]
    fn an_open_obligation_points_at_the_written_bound() {
        let mut fixture = Fixture::new();
        let element = fixture.var();
        let open = fixture.vec_of(element);
        let mut obligation = fixture.obligation(fixture.trait_bound("display"), open);
        obligation.site.bound_site = Some(Fixture::bound_site());
        let events = fixture.discharge(vec![obligation], ObligationPass::Final);
        assert_eq!(Fixture::problem_span(&events), Some(Fixture::bound_site()));
    }
}
