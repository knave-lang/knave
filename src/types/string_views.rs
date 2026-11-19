use super::place_types::PlaceTypes;
use super::ty::Type;
use super::typed::{TypedArgument, TypedExpr, TypedExprKind};
use super::unifier::Unifier;
use super::view_coercion::ViewCoercion;
use super::viewed_arguments::ViewedArguments;
use crate::core::{BindingId, PrimitiveType, ReferenceKind};

pub(super) struct StringViews<'a> {
    unifier: &'a Unifier,
    places: PlaceTypes<'a>,
    view: Option<BindingId>,
}

impl<'a> StringViews<'a> {
    pub(super) fn new(
        unifier: &'a Unifier,
        places: PlaceTypes<'a>,
        view: Option<BindingId>,
    ) -> Self {
        Self {
            unifier,
            places,
            view,
        }
    }

    pub(super) fn view_for_closure_result(&self, result: &Type, found: &Type) -> Option<BindingId> {
        if !self.needs_string_view(result, found) {
            return None;
        }
        self.view
    }

    pub(super) fn strings_to_view_beside_views(
        &self,
        positions: impl Iterator<Item = usize> + Clone,
        arguments: &[TypedArgument],
    ) -> Option<ViewedArguments> {
        let view = self.view?;
        if !positions
            .clone()
            .any(|position| self.is_string_view(&arguments[position].value.ty))
        {
            return None;
        }
        let positions = positions
            .filter(|&position| self.borrows_string_as_written(&arguments[position].value))
            .collect();
        Some(ViewedArguments { view, positions })
    }

    fn is_string_view(&self, value: &Type) -> bool {
        match self.unifier.resolve(value) {
            Type::Ref(_, view) => ViewCoercion::new(&view, &Self::owned_string()).views_string(),
            _ => false,
        }
    }

    fn borrows_string_as_written(&self, value: &TypedExpr) -> bool {
        let TypedExprKind::Borrow(ReferenceKind::Shared, place) = &value.kind else {
            return false;
        };
        let borrowed = self.places.read(place);
        self.unifier.resolve(&borrowed) == Self::owned_string()
    }

    fn owned_string() -> Type {
        Type::Primitive(PrimitiveType::String)
    }

    fn needs_string_view(&self, result: &Type, found: &Type) -> bool {
        match (self.unifier.resolve(result), self.unifier.resolve(found)) {
            (Type::Ref(_, view), Type::Ref(_, owned)) => {
                ViewCoercion::new(&view, &owned).views_string()
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Type;
    use crate::types::passing_fixture::PassingFixture;

    #[test]
    fn a_closure_returning_a_borrowed_string_as_a_view_is_viewed() {
        let mut fixture = PassingFixture::new();
        let view = fixture.view_binding();
        let result = Type::shared(PassingFixture::str());
        let found = Type::shared(PassingFixture::string());
        let viewed = (fixture.string_views(Some(view))).view_for_closure_result(&result, &found);
        assert_eq!(viewed, Some(view));
    }

    #[test]
    fn a_closure_returning_a_view_is_not_viewed_again() {
        let mut fixture = PassingFixture::new();
        let view = fixture.view_binding();
        let result = Type::shared(PassingFixture::str());
        let viewed = (fixture.string_views(Some(view))).view_for_closure_result(&result, &result);
        assert_eq!(viewed, None);
    }

    #[test]
    fn a_closure_returning_a_value_is_not_viewed() {
        let mut fixture = PassingFixture::new();
        let view = fixture.view_binding();
        let (result, found) = (PassingFixture::str(), PassingFixture::string());
        let viewed = (fixture.string_views(Some(view))).view_for_closure_result(&result, &found);
        assert_eq!(viewed, None);
    }

    #[test]
    fn without_a_view_function_nothing_is_viewed() {
        let mut fixture = PassingFixture::new();
        let result = Type::shared(PassingFixture::str());
        let found = Type::shared(PassingFixture::string());
        let viewed = (fixture.string_views(None)).view_for_closure_result(&result, &found);
        assert_eq!(viewed, None);
    }

    #[test]
    fn a_written_string_borrow_beside_a_view_is_viewed() {
        let mut fixture = PassingFixture::new();
        let view = fixture.view_binding();
        let arguments = [
            fixture.argument(fixture.written_borrow(PassingFixture::string())),
            fixture.argument(fixture.local(Type::shared(PassingFixture::str()))),
        ];
        let viewed = (fixture.string_views(Some(view)))
            .strings_to_view_beside_views(0..2, &arguments)
            .expect("a view sits among the positions");
        assert_eq!((viewed.view, viewed.positions), (view, vec![0]));
    }

    #[test]
    fn string_borrows_with_no_view_beside_them_are_not_viewed() {
        let mut fixture = PassingFixture::new();
        let view = fixture.view_binding();
        let arguments = [
            fixture.argument(fixture.written_borrow(PassingFixture::string())),
            fixture.argument(fixture.written_borrow(PassingFixture::string())),
        ];
        let viewed =
            (fixture.string_views(Some(view))).strings_to_view_beside_views(0..2, &arguments);
        assert!(viewed.is_none());
    }

    #[test]
    fn only_shared_borrows_of_strings_beside_a_view_are_viewed() {
        let mut fixture = PassingFixture::new();
        let view = fixture.view_binding();
        let arguments = [
            fixture.argument(fixture.local(Type::shared(PassingFixture::str()))),
            fixture.argument(fixture.written_mut_borrow(PassingFixture::string())),
            fixture.argument(fixture.written_borrow(PassingFixture::int())),
        ];
        let viewed = (fixture.string_views(Some(view)))
            .strings_to_view_beside_views(0..3, &arguments)
            .expect("a view sits among the positions");
        assert!(viewed.positions.is_empty());
    }
}
