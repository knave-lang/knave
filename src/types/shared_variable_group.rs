use rustc_hash::FxHashMap;

use super::group_depths::GroupDepths;
use super::passing_facts::PassingFacts;
use super::passing_tables::PassingTables;
use super::place_types::PlaceTypes;
use super::reference_depth::ReferenceDepth;
use super::shared_group::SharedGroup;
use super::shared_passing_order::SharedPassingOrder;
use super::ty::{Bound, Type, TypeVar};
use super::typed::TypedArgument;
use super::unifier::Unifier;

pub(super) struct SharedVariableGroup<'a> {
    group: &'a SharedGroup,
    tables: PassingTables<'a>,
    unifier: &'a Unifier,
}

impl<'a> SharedVariableGroup<'a> {
    pub(super) fn new(
        group: &'a SharedGroup,
        tables: PassingTables<'a>,
        unifier: &'a Unifier,
    ) -> Self {
        Self {
            group,
            tables,
            unifier,
        }
    }

    pub(super) fn passes_as_written(&self, arguments: &[TypedArgument]) -> bool {
        let facts = self.facts();
        (self.group.positions()).any(|at| facts.passes_as_written(&arguments[at].value))
    }

    pub(super) fn written_assignment(
        &self,
        arguments: &[TypedArgument],
    ) -> Option<FxHashMap<TypeVar, Type>> {
        let assignment = self.assignment_as_written(arguments)?;
        if !self.written_depths_agree(arguments) || !self.written_types_hold(&assignment) {
            return None;
        }
        Some(assignment)
    }

    pub(super) fn depths(&self, arguments: &[TypedArgument]) -> GroupDepths {
        let places = self.places();
        let positions: Vec<usize> = self.group.positions().collect();
        let resolved = positions
            .iter()
            .map(|position| self.unifier.resolve(&arguments[*position].value.ty))
            .collect();
        let depths = positions
            .iter()
            .map(|position| places.reference_depth_as_written(&arguments[*position].value))
            .collect();
        GroupDepths {
            positions,
            resolved,
            depths,
        }
    }

    pub(super) fn leaves_referent_to_each(&self, depths: &GroupDepths) -> bool {
        (depths.resolved.iter()).any(|argument| self.facts().referent_unknown(argument))
            && !self.copies_known_referent(depths)
    }

    pub(super) fn target_depth(
        &self,
        referent: &Type,
        depths: &GroupDepths,
        order: &SharedPassingOrder,
    ) -> ReferenceDepth {
        let depths = &depths.depths;
        if self.copies_out(referent) {
            ReferenceDepth::VALUE
        } else if depths.windows(2).all(|pair| pair[0] == pair[1]) {
            let depth = depths[0];
            if self.reference_blocks_bounds(referent, depth) {
                ReferenceDepth::VALUE
            } else {
                ReferenceDepth::new(depth)
            }
        } else if order.borrows_every(self.group) {
            ReferenceDepth::VALUE
        } else {
            ReferenceDepth::new(1)
        }
    }

    fn assignment_as_written(
        &self,
        arguments: &[TypedArgument],
    ) -> Option<FxHashMap<TypeVar, Type>> {
        let mut assignment = FxHashMap::default();
        for member in self.group.members() {
            let first = self
                .unifier
                .resolve(&arguments[member.positions[0]].value.ty);
            let agrees = member
                .positions
                .iter()
                .all(|position| self.unifier.resolve(&arguments[*position].value.ty) == first);
            if !agrees {
                return None;
            }
            assignment.insert(member.var, first);
        }
        Some(assignment)
    }

    fn written_depths_agree(&self, arguments: &[TypedArgument]) -> bool {
        let resolved: Vec<Type> = (self.group.positions())
            .map(|position| self.unifier.resolve(&arguments[position].value.ty))
            .collect();
        let referent = resolved[0].innermost_referent().clone();
        if !resolved
            .iter()
            .all(|argument| argument.innermost_referent() == &referent)
        {
            return true;
        }
        let places = self.places();
        let depths: Vec<usize> = (self.group.positions())
            .map(|position| places.reference_depth_as_written(&arguments[position].value))
            .collect();
        depths.windows(2).all(|pair| pair[0] == pair[1])
    }

    fn written_types_hold(&self, assignment: &FxHashMap<TypeVar, Type>) -> bool {
        !assignment
            .values()
            .any(|argument| self.facts().referent_unknown(argument))
            && self.group_bounds_hold(assignment)
    }

    fn group_bounds_hold(&self, assignment: &FxHashMap<TypeVar, Type>) -> bool {
        self.group.members().iter().all(|member| {
            let implementor = &assignment[&member.var];
            self.unifier
                .bounds(member.var)
                .iter()
                .all(|bound| self.bound_holds(bound, implementor, assignment))
        })
    }

    fn copies_known_referent(&self, depths: &GroupDepths) -> bool {
        let facts = self.facts();
        depths.depths.windows(2).any(|pair| pair[0] != pair[1])
            && (depths.resolved.iter())
                .any(|argument| facts.holds_copy_value(argument.innermost_referent()))
    }

    fn reference_blocks_bounds(&self, referent: &Type, depth: usize) -> bool {
        depth > 0
            && !self.group_fails_bounds(referent)
            && self.group_fails_bounds(&ReferenceDepth::new(depth).wrap(referent.clone()))
    }

    fn copies_out(&self, referent: &Type) -> bool {
        match referent {
            Type::Var(literal) => self.unifier.literal_kind(*literal).is_some(),
            _ => {
                self.tables.copy_rules().is_small_copy(referent)
                    && !self.group_fails_bounds(referent)
            }
        }
    }

    fn group_fails_bounds(&self, referent: &Type) -> bool {
        let assignment: FxHashMap<TypeVar, Type> = (self.group.members().iter())
            .map(|member| (member.var, referent.clone()))
            .collect();
        self.group.members().iter().any(|member| {
            self.unifier
                .bounds(member.var)
                .iter()
                .any(|bound| !self.bound_holds(bound, referent, &assignment))
        })
    }

    fn bound_holds(
        &self,
        bound: &Bound,
        implementor: &Type,
        assignment: &FxHashMap<TypeVar, Type>,
    ) -> bool {
        let bound = bound.substituted_vars(assignment);
        self.tables.declarations.traits().table.satisfies(
            &bound,
            implementor,
            &|generic, required| self.unifier.generic_bounds(generic).contains(required),
        )
    }

    fn facts(&self) -> PassingFacts<'a> {
        PassingFacts::new(self.tables, self.unifier)
    }

    fn places(&self) -> PlaceTypes<'a> {
        PlaceTypes::new(self.unifier, self.tables.declared_types())
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Type;
    use crate::types::passing_fixture::PassingFixture;
    use crate::types::reference_depth::ReferenceDepth;

    #[test]
    fn a_written_borrow_passes_the_group_as_written() {
        let mut fixture = PassingFixture::new();
        let group = fixture.shared_group(&[0, 0]);
        let arguments = [
            fixture.argument(fixture.written_borrow(PassingFixture::int())),
            fixture.argument(fixture.local(PassingFixture::int())),
        ];
        assert!(
            fixture
                .shared_variable_group(&group)
                .passes_as_written(&arguments)
        );
    }

    #[test]
    fn locals_do_not_pass_the_group_as_written() {
        let mut fixture = PassingFixture::new();
        let group = fixture.shared_group(&[0, 0]);
        let arguments = [
            fixture.argument(fixture.local(PassingFixture::int())),
            fixture.argument(fixture.local(PassingFixture::int())),
        ];
        assert!(
            !fixture
                .shared_variable_group(&group)
                .passes_as_written(&arguments)
        );
    }

    #[test]
    fn members_of_different_types_are_assigned_as_written() {
        let mut fixture = PassingFixture::new();
        let group = fixture.shared_group(&[0, 1]);
        let arguments = [
            fixture.argument(fixture.local(PassingFixture::int())),
            fixture.argument(fixture.local(PassingFixture::string())),
        ];
        let assignment = (fixture.shared_variable_group(&group))
            .written_assignment(&arguments)
            .expect("each member takes its own type");
        let members = group.members();
        assert_eq!(assignment[&members[0].var], PassingFixture::int());
        assert_eq!(assignment[&members[1].var], PassingFixture::string());
    }

    #[test]
    fn a_member_whose_positions_disagree_is_not_assigned_as_written() {
        let mut fixture = PassingFixture::new();
        let group = fixture.shared_group(&[0, 0]);
        let arguments = [
            fixture.argument(fixture.local(PassingFixture::int())),
            fixture.argument(fixture.local(PassingFixture::string())),
        ];
        let assignment = fixture
            .shared_variable_group(&group)
            .written_assignment(&arguments);
        assert!(assignment.is_none());
    }

    #[test]
    fn one_referent_at_two_depths_is_not_assigned_as_written() {
        let mut fixture = PassingFixture::new();
        let group = fixture.shared_group(&[0, 1]);
        let arguments = [
            fixture.argument(fixture.written_borrow(PassingFixture::int())),
            fixture.argument(fixture.local(PassingFixture::int())),
        ];
        let assignment = fixture
            .shared_variable_group(&group)
            .written_assignment(&arguments);
        assert!(assignment.is_none());
    }

    #[test]
    fn an_unknown_type_is_not_assigned_as_written() {
        let mut fixture = PassingFixture::new();
        let group = fixture.shared_group(&[0, 1]);
        let (first, second) = (fixture.unifier().fresh(), fixture.unifier().fresh());
        let arguments = [
            fixture.argument(fixture.local(first)),
            fixture.argument(fixture.local(second)),
        ];
        let assignment = fixture
            .shared_variable_group(&group)
            .written_assignment(&arguments);
        assert!(assignment.is_none());
    }

    #[test]
    fn the_depths_are_read_as_written() {
        let mut fixture = PassingFixture::new();
        let group = fixture.shared_group(&[0, 0]);
        let arguments = [
            fixture.argument(fixture.written_borrow(PassingFixture::int())),
            fixture.argument(fixture.local(PassingFixture::int())),
        ];
        let depths = fixture.shared_variable_group(&group).depths(&arguments);
        assert_eq!(depths.positions, vec![0, 1]);
        assert_eq!(depths.depths, vec![1, 0]);
        assert_eq!(
            depths.resolved,
            vec![Type::shared(PassingFixture::int()), PassingFixture::int()]
        );
    }

    #[test]
    fn an_unknown_referent_leaves_the_group_to_each_member() {
        let mut fixture = PassingFixture::new();
        let group = fixture.shared_group(&[0, 0]);
        let unknown = fixture.unifier().fresh();
        let arguments = [
            fixture.argument(fixture.local(unknown.clone())),
            fixture.argument(fixture.local(unknown)),
        ];
        let rules = fixture.shared_variable_group(&group);
        assert!(rules.leaves_referent_to_each(&rules.depths(&arguments)));
    }

    #[test]
    fn a_known_referent_keeps_the_group_together() {
        let mut fixture = PassingFixture::new();
        let group = fixture.shared_group(&[0, 0]);
        let arguments = [
            fixture.argument(fixture.local(PassingFixture::string())),
            fixture.argument(fixture.local(PassingFixture::string())),
        ];
        let rules = fixture.shared_variable_group(&group);
        assert!(!rules.leaves_referent_to_each(&rules.depths(&arguments)));
    }

    #[test]
    fn a_copy_value_at_another_depth_keeps_an_unknown_beside_it_together() {
        let mut fixture = PassingFixture::new();
        let group = fixture.shared_group(&[0, 0]);
        let unknown = fixture.unifier().fresh();
        let arguments = [
            fixture.argument(fixture.local(unknown)),
            fixture.argument(fixture.written_borrow(PassingFixture::int())),
        ];
        let rules = fixture.shared_variable_group(&group);
        assert!(!rules.leaves_referent_to_each(&rules.depths(&arguments)));
    }

    #[test]
    fn a_small_copy_referent_passes_by_value() {
        let target = PassingFixture::new().shared_target(PassingFixture::int(), &[1, 1]);
        assert_eq!(target, ReferenceDepth::VALUE);
    }

    #[test]
    fn a_literal_referent_passes_by_value() {
        let mut fixture = PassingFixture::new();
        let literal = fixture.literal();
        assert_eq!(
            fixture.shared_target(literal, &[1, 1]),
            ReferenceDepth::VALUE
        );
    }

    #[test]
    fn an_owned_referent_at_one_depth_keeps_that_depth() {
        let target = PassingFixture::new().shared_target(PassingFixture::string(), &[2, 2]);
        assert_eq!(target, ReferenceDepth::new(2));
    }

    #[test]
    fn an_owned_referent_at_mixed_depths_is_borrowed_once() {
        let target = PassingFixture::new().shared_target(PassingFixture::string(), &[0, 2]);
        assert_eq!(target, ReferenceDepth::new(1));
    }

    #[test]
    fn mixed_depths_that_rust_borrows_pass_by_value() {
        let mut fixture = PassingFixture::new();
        let callee = PassingFixture::template_call(PassingFixture::BORROWING_BOTH);
        let target = fixture.shared_target_for(&callee, PassingFixture::string(), &[0, 2]);
        assert_eq!(target, ReferenceDepth::VALUE);
    }
}
