use super::callee_borrows::CalleeBorrows;
use super::place_types::PlaceTypes;
use super::reference_depth::ReferenceDepth;
use super::shared_group::SharedGroup;
use super::ty::Type;
use super::typed::{Borrow, TypedArgument, TypedExpr, TypedExprKind};
use super::unifier::{LiteralKind, Unifier};
use crate::core::ReferenceKind;

pub(super) struct SharedPassingOrder<'a> {
    places: PlaceTypes<'a>,
    unifier: &'a Unifier,
    callee: CalleeBorrows<'a>,
}

impl<'a> SharedPassingOrder<'a> {
    pub(super) fn new(
        places: PlaceTypes<'a>,
        unifier: &'a Unifier,
        callee: CalleeBorrows<'a>,
    ) -> Self {
        Self {
            places,
            unifier,
            callee,
        }
    }

    pub(super) fn order(
        &self,
        group: &SharedGroup,
        shared: &Type,
        arguments: &[TypedArgument],
    ) -> Vec<usize> {
        let mut positions: Vec<usize> = group.positions().collect();
        positions.sort_by_key(|position| {
            let value = &arguments[*position].value;
            (
                self.template_place(*position, shared, value).is_none(),
                self.takes_type_from_others(group, *position, arguments),
            )
        });
        positions
    }

    pub(super) fn template_place<'v>(
        &self,
        position: usize,
        shared: &Type,
        value: &'v TypedExpr,
    ) -> Option<&'v TypedExpr> {
        let TypedExprKind::Borrow(ReferenceKind::Shared, place) = &value.kind else {
            return None;
        };
        (self.callee.borrows_at(position) && !self.unifier.shallow(shared).is_reference())
            .then_some(place)
    }

    pub(super) fn dereference_where_rust_borrows(
        &self,
        position: usize,
        borrow: Borrow,
        argument: &TypedArgument,
    ) -> Borrow {
        if borrow != Borrow::Passthrough || !self.callee.borrows_at(position) {
            return borrow;
        }
        ReferenceDepth::VALUE.borrow_from(self.places.reference_depth_as_written(&argument.value))
    }

    fn takes_type_from_others(
        &self,
        group: &SharedGroup,
        position: usize,
        arguments: &[TypedArgument],
    ) -> bool {
        let Some(kind) = self.open_literal_kind(&arguments[position].value.ty) else {
            return false;
        };
        let others = group.positions().filter(|other| *other != position);
        !self.callee.borrows_at(position) && self.others_take_literal(kind, others, arguments)
    }

    fn open_literal_kind(&self, value: &Type) -> Option<LiteralKind> {
        match self.unifier.shallow(value) {
            Type::Var(var) => self.unifier.literal_kind(var),
            _ => None,
        }
    }

    fn others_take_literal(
        &self,
        kind: LiteralKind,
        mut others: impl Iterator<Item = usize>,
        arguments: &[TypedArgument],
    ) -> bool {
        others.all(|other| self.referent_takes_literal(kind, &arguments[other].value.ty))
    }

    fn referent_takes_literal(&self, kind: LiteralKind, value: &Type) -> bool {
        let resolved = self.unifier.resolve(value);
        match resolved.innermost_referent() {
            Type::Primitive(primitive) => kind.accepts(*primitive),
            Type::Var(var) => self.unifier.literal_kind(*var).is_some(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::passing_fixture::PassingFixture;
    use crate::types::{Borrow, Type};

    #[test]
    fn a_literal_that_the_others_take_passes_after_them() {
        let mut fixture = PassingFixture::new();
        let group = fixture.shared_group(&[0, 0]);
        let literal = fixture.literal();
        let arguments = [
            fixture.argument(fixture.local(literal)),
            fixture.argument(fixture.local(PassingFixture::int())),
        ];
        let shared = fixture.unifier().fresh();
        let order = fixture
            .shared_passing_order(None)
            .order(&group, &shared, &arguments);
        assert_eq!(order, vec![1, 0]);
    }

    #[test]
    fn a_literal_beside_a_type_that_refuses_it_keeps_its_place() {
        let mut fixture = PassingFixture::new();
        let group = fixture.shared_group(&[0, 0]);
        let literal = fixture.literal();
        let arguments = [
            fixture.argument(fixture.local(literal)),
            fixture.argument(fixture.local(PassingFixture::string())),
        ];
        let shared = fixture.unifier().fresh();
        let order = fixture
            .shared_passing_order(None)
            .order(&group, &shared, &arguments);
        assert_eq!(order, vec![0, 1]);
    }

    #[test]
    fn a_place_that_the_template_borrows_passes_first() {
        let mut fixture = PassingFixture::new();
        let callee = PassingFixture::template_call(PassingFixture::BORROWING_SECOND);
        let group = fixture.shared_group(&[0, 0]);
        let arguments = [
            fixture.argument(fixture.local(PassingFixture::string())),
            fixture.argument(fixture.written_borrow(PassingFixture::string())),
        ];
        let shared = fixture.unifier().fresh();
        let order =
            (fixture.shared_passing_order(Some(&callee))).order(&group, &shared, &arguments);
        assert_eq!(order, vec![1, 0]);
    }

    #[test]
    fn a_written_borrow_that_the_template_borrows_passes_its_place() {
        let mut fixture = PassingFixture::new();
        let callee = PassingFixture::template_call(PassingFixture::BORROWING_SECOND);
        let value = fixture.written_borrow(PassingFixture::string());
        let shared = fixture.unifier().fresh();
        let place =
            (fixture.shared_passing_order(Some(&callee))).template_place(1, &shared, &value);
        assert_eq!(
            place.map(|place| &place.ty),
            Some(&PassingFixture::string())
        );
    }

    #[test]
    fn a_written_borrow_that_the_template_does_not_borrow_passes_as_is() {
        let mut fixture = PassingFixture::new();
        let callee = PassingFixture::template_call(PassingFixture::BORROWING_SECOND);
        let value = fixture.written_borrow(PassingFixture::string());
        let shared = fixture.unifier().fresh();
        let place =
            (fixture.shared_passing_order(Some(&callee))).template_place(0, &shared, &value);
        assert!(place.is_none());
    }

    #[test]
    fn a_written_borrow_of_a_shared_reference_type_passes_as_is() {
        let fixture = PassingFixture::new();
        let callee = PassingFixture::template_call(PassingFixture::BORROWING_SECOND);
        let value = fixture.written_borrow(PassingFixture::string());
        let shared = Type::shared(PassingFixture::string());
        let place =
            (fixture.shared_passing_order(Some(&callee))).template_place(1, &shared, &value);
        assert!(place.is_none());
    }

    #[test]
    fn a_passthrough_that_the_template_borrows_is_dereferenced() {
        let fixture = PassingFixture::new();
        let callee = PassingFixture::template_call(PassingFixture::BORROWING_SECOND);
        let argument = fixture.argument(fixture.written_borrow(PassingFixture::string()));
        let borrow = (fixture.shared_passing_order(Some(&callee))).dereference_where_rust_borrows(
            1,
            Borrow::Passthrough,
            &argument,
        );
        assert_eq!(borrow, Borrow::Deref(1));
    }

    #[test]
    fn a_passthrough_that_the_template_does_not_borrow_stays() {
        let fixture = PassingFixture::new();
        let callee = PassingFixture::template_call(PassingFixture::BORROWING_SECOND);
        let argument = fixture.argument(fixture.written_borrow(PassingFixture::string()));
        let borrow = (fixture.shared_passing_order(Some(&callee))).dereference_where_rust_borrows(
            0,
            Borrow::Passthrough,
            &argument,
        );
        assert_eq!(borrow, Borrow::Passthrough);
    }

    #[test]
    fn another_borrow_that_the_template_borrows_stays() {
        let fixture = PassingFixture::new();
        let callee = PassingFixture::template_call(PassingFixture::BORROWING_SECOND);
        let argument = fixture.argument(fixture.written_borrow(PassingFixture::string()));
        let borrow = (fixture.shared_passing_order(Some(&callee))).dereference_where_rust_borrows(
            1,
            Borrow::Shared,
            &argument,
        );
        assert_eq!(borrow, Borrow::Shared);
    }
}
