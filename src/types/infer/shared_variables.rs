use rustc_hash::FxHashMap;

use crate::diagnostic::Diagnostic;
use crate::types::checker::TypeChecker;
use crate::types::expectation::Expectation;
use crate::types::group_depths::GroupDepths;
use crate::types::shared_group::SharedGroup;
use crate::types::trait_arguments::TraitArguments;
use crate::types::unifier::Unifier;
use crate::types::{Borrow, Type, TypeVar, TypedArgument, TypedExpr};

pub(super) struct SharedVariables {
    groups: Vec<SharedGroup>,
}

#[derive(Default)]
struct VariableLinks {
    parents: FxHashMap<TypeVar, TypeVar>,
}

impl VariableLinks {
    fn link(&mut self, left: TypeVar, right: TypeVar) {
        let left = self.find(left);
        let right = self.find(right);
        if left != right {
            self.parents.insert(left, right);
        }
    }

    fn find(&mut self, var: TypeVar) -> TypeVar {
        let parent = *self.parents.entry(var).or_insert(var);
        if parent == var {
            return var;
        }
        let root = self.find(parent);
        self.parents.insert(var, root);
        root
    }
}

impl SharedVariables {
    pub(super) fn of(
        parameters: &[Type],
        unifier: &Unifier,
        traits: &crate::types::TraitTable,
    ) -> Self {
        let whole: Vec<Option<TypeVar>> = parameters
            .iter()
            .map(|parameter| match unifier.shallow(parameter) {
                Type::Var(var) => Some(var),
                _ => None,
            })
            .collect();
        let mut links = Self::links(&whole, unifier, traits);
        let mut groups: Vec<SharedGroup> = Vec::new();
        for (position, whole) in whole.iter().enumerate() {
            let Some(var) = whole else {
                continue;
            };
            let root = links.find(*var);
            let group = Self::group_of(&mut groups, root);
            group.add_position(*var, position);
        }
        groups.retain(|group| group.positions().count() > 1);
        Self { groups }
    }

    fn links(
        whole: &[Option<TypeVar>],
        unifier: &Unifier,
        traits: &crate::types::TraitTable,
    ) -> VariableLinks {
        let mut links = VariableLinks::default();
        let arguments = TraitArguments::of(traits);
        for var in whole.iter().flatten() {
            for bound in unifier.bounds(*var) {
                let argument = bound
                    .argument
                    .as_ref()
                    .map(|argument| unifier.shallow(argument));
                if let Some(Type::Var(linked)) = argument
                    && arguments.links(bound.binding)
                {
                    links.link(*var, linked);
                }
            }
        }
        links
    }

    fn group_of(groups: &mut Vec<SharedGroup>, root: TypeVar) -> &mut SharedGroup {
        match groups.iter().position(|group| group.root() == root) {
            Some(position) => &mut groups[position],
            None => {
                groups.push(SharedGroup::new(root));
                groups.last_mut().expect("the group was just added")
            }
        }
    }

    pub(super) fn contains(&self, position: usize) -> bool {
        self.groups.iter().any(|group| group.contains(position))
    }

    fn groups(&self) -> &[SharedGroup] {
        &self.groups
    }
}

impl TypeChecker {
    pub(super) fn pass_shared_arguments(
        &mut self,
        callee: Option<&TypedExpr>,
        shared: &SharedVariables,
        arguments: &mut [TypedArgument],
    ) -> Result<(), Diagnostic> {
        for group in shared.groups() {
            if self
                .shared_variable_group(group, callee)
                .passes_as_written(arguments)
            {
                self.pass_each(callee, group, arguments)?;
                continue;
            }
            if group.is_linked() && self.try_as_written(callee, group, arguments)? {
                continue;
            }
            self.pass_group(callee, group, arguments)?;
        }
        Ok(())
    }

    fn try_as_written(
        &mut self,
        callee: Option<&TypedExpr>,
        group: &SharedGroup,
        arguments: &mut [TypedArgument],
    ) -> Result<bool, Diagnostic> {
        let Some(assignment) = self
            .shared_variable_group(group, callee)
            .written_assignment(arguments)
        else {
            return Ok(false);
        };
        for member in group.members() {
            let span = arguments[member.positions[0]].value.span;
            let found = assignment[&member.var].clone();
            self.unification()
                .expect(Expectation::plain(&Type::Var(member.var)), &found, span)?;
            for position in &member.positions {
                arguments[*position].borrow = Borrow::None;
            }
        }
        Ok(true)
    }

    fn pass_group(
        &mut self,
        callee: Option<&TypedExpr>,
        group: &SharedGroup,
        arguments: &mut [TypedArgument],
    ) -> Result<(), Diagnostic> {
        let rules = self.shared_variable_group(group, callee);
        let depths = rules.depths(arguments);
        if rules.leaves_referent_to_each(&depths) {
            return self.pass_each(callee, group, arguments);
        }
        let referent = self.unify_referents(&depths, arguments)?;
        let target = (self.shared_variable_group(group, callee)).target_depth(&referent, &depths);
        let span = arguments[depths.first().position].value.span;
        let chosen = target.wrap(referent);
        for member in group.members() {
            self.unification()
                .expect(Expectation::plain(&Type::Var(member.var)), &chosen, span)?;
        }
        let passing = self.shared_member_passing(callee);
        for argument in depths.arguments() {
            let at = argument.position;
            let borrow = target.borrow_from(argument.depth);
            arguments[at].borrow =
                passing.dereference_where_rust_borrows(at, borrow, &arguments[at]);
        }
        Ok(())
    }

    fn pass_each(
        &mut self,
        callee: Option<&TypedExpr>,
        group: &SharedGroup,
        arguments: &mut [TypedArgument],
    ) -> Result<(), Diagnostic> {
        let root = group.root();
        let span = arguments[group.positions().next().expect("a group has a position")]
            .value
            .span;
        for member in group.members() {
            self.unification().expect(
                Expectation::plain(&Type::Var(root)),
                &Type::Var(member.var),
                span,
            )?;
        }
        self.view_borrowed_strings_beside_views(group.positions(), arguments)?;
        let shared = Type::Var(root);
        let order = (self.shared_member_passing(callee)).order(group, &shared, arguments);
        for position in order {
            self.pass_member(callee, position, &shared, &mut arguments[position])?;
        }
        Ok(())
    }

    fn pass_member(
        &mut self,
        callee: Option<&TypedExpr>,
        position: usize,
        shared: &Type,
        argument: &mut TypedArgument,
    ) -> Result<(), Diagnostic> {
        let passing = self.shared_member_passing(callee);
        match passing.template_place(position, shared, &argument.value) {
            Some(place) => {
                let place = place.clone();
                self.pass_template_borrow(shared, place, argument)
            }
            None => {
                let borrow = self.pass_call_argument(callee, position, shared, &argument.value)?;
                argument.borrow = (self.shared_member_passing(callee))
                    .dereference_where_rust_borrows(position, borrow, argument);
                Ok(())
            }
        }
    }

    fn pass_template_borrow(
        &mut self,
        shared: &Type,
        place: TypedExpr,
        argument: &mut TypedArgument,
    ) -> Result<(), Diagnostic> {
        let written = self.place_types().read(&place);
        self.unification()
            .expect(Expectation::plain(shared), &written, place.span)?;
        *argument = TypedArgument {
            value: place,
            borrow: Borrow::Shared,
        };
        Ok(())
    }

    fn unify_referents(
        &mut self,
        depths: &GroupDepths,
        arguments: &[TypedArgument],
    ) -> Result<Type, Diagnostic> {
        let referent = depths.first().resolved.innermost_referent().clone();
        for argument in depths.arguments().iter().skip(1) {
            let span = arguments[argument.position].value.span;
            let found = argument.resolved.innermost_referent();
            self.unification()
                .expect(Expectation::plain(&referent), found, span)?;
        }
        Ok(self.unifier.resolve(&referent))
    }
}
