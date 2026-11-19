use crate::types::{Borrow, CopyRules, RustTypeTable, Type, ViewCoercion};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Adjustment {
    None,
    Shared,
    Mutable,
    Deref(usize),
}

impl Adjustment {
    pub(super) fn borrow(self) -> Borrow {
        match self {
            Self::None => Borrow::None,
            Self::Shared => Borrow::Shared,
            Self::Mutable => Borrow::Mutable,
            Self::Deref(levels) => Borrow::Deref(levels),
        }
    }
}

pub(super) struct ParameterAdjustments<'a> {
    copy: CopyRules<'a>,
    rust_types: &'a RustTypeTable,
}

impl<'a> ParameterAdjustments<'a> {
    pub(super) fn new(copy: CopyRules<'a>, rust_types: &'a RustTypeTable) -> Self {
        Self { copy, rust_types }
    }

    pub(super) fn of_each(
        &self,
        expected: &[Type],
        actual: &[Type],
    ) -> Result<Vec<Adjustment>, usize> {
        if expected.len() != actual.len() {
            return Err(1);
        }
        expected.iter().zip(actual).enumerate().try_fold(
            Vec::new(),
            |mut found, (position, (expected, actual))| match self.adjustment(expected, actual) {
                Some(adjustment) => {
                    found.push(adjustment);
                    Ok(found)
                }
                None => Err(position + 1),
            },
        )
    }

    fn adjustment(&self, expected: &Type, actual: &Type) -> Option<Adjustment> {
        if expected == actual {
            return Some(Adjustment::None);
        }
        self.added_borrow(expected, actual)
            .or_else(|| self.copied_value(expected, actual))
            .or_else(|| self.reborrowed(expected, actual))
            .or_else(|| self.removed_borrows(expected, actual))
            .or_else(|| self.coerced_view(expected, actual))
    }

    fn coerced_view(&self, expected: &Type, actual: &Type) -> Option<Adjustment> {
        let (Type::Ref(_, owner), Type::Ref(_, view)) = (expected, actual) else {
            return None;
        };
        let owner = Self::without_references(owner, Self::reference_depth(owner));
        let coercion = ViewCoercion::new(view, &owner);
        let views = coercion.views_string() || coercion.viewed_items(self.rust_types).is_some();
        views.then_some(Adjustment::None)
    }

    fn added_borrow(&self, expected: &Type, actual: &Type) -> Option<Adjustment> {
        if expected.is_reference() {
            return None;
        }
        match actual {
            Type::Ref(_, _) => Some(Adjustment::Shared),
            Type::Mut(_, _) => Some(Adjustment::Mutable),
            _ => None,
        }
    }

    fn copied_value(&self, expected: &Type, actual: &Type) -> Option<Adjustment> {
        if !expected.is_reference() || actual.is_reference() {
            return None;
        }
        self.copy
            .is_small_copy(actual)
            .then(|| Adjustment::Deref(Self::borrow_depth(expected)))
    }

    fn borrow_depth(value: &Type) -> usize {
        if value.is_reference() {
            1 + Self::borrow_depth(value.referent())
        } else {
            0
        }
    }

    fn reborrowed(&self, expected: &Type, actual: &Type) -> Option<Adjustment> {
        matches!((expected, actual), (Type::Mut(_, _), Type::Ref(_, _))).then_some(Adjustment::None)
    }

    fn removed_borrows(&self, expected: &Type, actual: &Type) -> Option<Adjustment> {
        if !actual.is_reference() {
            return None;
        }
        let expected_depth = Self::reference_depth(expected);
        let actual_depth = Self::reference_depth(actual);
        (expected_depth > actual_depth
            && Self::without_references(expected, expected_depth - actual_depth) == *actual)
            .then_some(Adjustment::None)
    }

    fn reference_depth(value: &Type) -> usize {
        match value {
            Type::Ref(_, inner) => 1 + Self::reference_depth(inner),
            _ => 0,
        }
    }

    fn without_references(value: &Type, levels: usize) -> Type {
        match value {
            Type::Ref(_, inner) if levels > 0 => Self::without_references(inner, levels - 1),
            _ => value.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_hash::FxHashMap;

    use crate::core::{BindingArena, BindingInfo, BindingKind, ModuleId, PrimitiveType};
    use crate::types::{
        GenericId, GenericParam, Implementor, LoweredRows, RustTrait, Scheme, TraitTable,
    };

    struct Adjustments {
        generics: FxHashMap<GenericId, GenericParam>,
        rust_types: RustTypeTable,
        declarations: crate::types::Declarations,
    }

    impl Adjustments {
        fn new() -> Self {
            let mut bindings = BindingArena::default();
            let mut traits = TraitTable::default();
            let copy = bindings.add(BindingInfo::new(
                "copy".into(),
                BindingKind::Trait(ModuleId::new(0)),
                None,
            ));
            traits.add(
                copy,
                RustTrait::written(
                    std::rc::Rc::from("Copy"),
                    None,
                    LoweredRows::new(
                        vec![Implementor {
                            scheme: Scheme::monomorphic(Type::Primitive(PrimitiveType::I64)),
                            argument: None,
                        }],
                        Vec::new(),
                    ),
                ),
            );
            Self {
                generics: FxHashMap::default(),
                rust_types: RustTypeTable::default(),
                declarations: crate::types::Declarations::with_traits(traits),
            }
        }

        fn number(&self) -> Type {
            Type::Primitive(PrimitiveType::I64)
        }

        fn text(&self) -> Type {
            Type::Primitive(PrimitiveType::String)
        }

        fn borrowed(&self, value: Type) -> Type {
            Type::shared(value)
        }

        fn changed(&self, value: Type) -> Type {
            Type::mutable(value)
        }

        fn decide(&self, expected: &Type, actual: &Type) -> Option<Adjustment> {
            self.adjustments().adjustment(expected, actual)
        }

        fn decide_many(
            &self,
            expected: &[Type],
            actual: &[Type],
        ) -> Result<Vec<Adjustment>, usize> {
            self.adjustments().of_each(expected, actual)
        }

        fn adjustments(&self) -> ParameterAdjustments<'_> {
            ParameterAdjustments::new(
                CopyRules::new(&self.declarations, &self.generics),
                &self.rust_types,
            )
        }
    }

    #[test]
    fn every_adjustment_names_its_borrow() {
        assert_eq!(Adjustment::None.borrow(), Borrow::None);
        assert_eq!(Adjustment::Shared.borrow(), Borrow::Shared);
        assert_eq!(Adjustment::Mutable.borrow(), Borrow::Mutable);
        assert_eq!(Adjustment::Deref(2).borrow(), Borrow::Deref(2));
    }

    #[test]
    fn different_owned_types_are_unfixable() {
        let fixture = Adjustments::new();
        let number = fixture.number();
        let unit = Type::Primitive(crate::core::PrimitiveType::Unit);
        assert_eq!(fixture.decide(&number, &unit), None);
    }

    #[test]
    fn matching_parameters_need_no_adjustment() {
        let fixture = Adjustments::new();
        let number = fixture.number();
        let borrowed = fixture.borrowed(number.clone());
        let changed = fixture.changed(number.clone());
        assert_eq!(fixture.decide(&number, &number), Some(Adjustment::None));
        assert_eq!(fixture.decide(&borrowed, &borrowed), Some(Adjustment::None));
        assert_eq!(fixture.decide(&changed, &changed), Some(Adjustment::None));
    }

    #[test]
    fn an_owned_parameter_takes_a_borrow() {
        let fixture = Adjustments::new();
        let number = fixture.number();
        assert_eq!(
            fixture.decide(&number, &fixture.borrowed(number.clone())),
            Some(Adjustment::Shared)
        );
        assert_eq!(
            fixture.decide(&number, &fixture.changed(number.clone())),
            Some(Adjustment::Mutable)
        );
    }

    #[test]
    fn a_borrowed_parameter_copies_a_small_value() {
        let fixture = Adjustments::new();
        let number = fixture.number();
        assert_eq!(
            fixture.decide(&fixture.borrowed(number.clone()), &number),
            Some(Adjustment::Deref(1))
        );
        assert_eq!(
            fixture.decide(&fixture.changed(number.clone()), &number),
            Some(Adjustment::Deref(1))
        );
    }

    #[test]
    fn a_doubly_borrowed_parameter_copies_through_each_borrow() {
        let fixture = Adjustments::new();
        let number = fixture.number();
        let twice = fixture.borrowed(fixture.borrowed(number.clone()));
        let changed_borrow = fixture.changed(fixture.borrowed(number.clone()));
        assert_eq!(fixture.decide(&twice, &number), Some(Adjustment::Deref(2)));
        assert_eq!(
            fixture.decide(&changed_borrow, &number),
            Some(Adjustment::Deref(2))
        );
    }

    #[test]
    fn a_borrowed_parameter_cannot_take_a_large_value() {
        let fixture = Adjustments::new();
        let text = fixture.text();
        assert_eq!(fixture.decide(&fixture.borrowed(text.clone()), &text), None);
        assert_eq!(fixture.decide(&fixture.changed(text.clone()), &text), None);
    }

    #[test]
    fn a_mutable_parameter_reborrows_a_shared_one() {
        let fixture = Adjustments::new();
        let number = fixture.number();
        assert_eq!(
            fixture.decide(&fixture.changed(number.clone()), &fixture.borrowed(number)),
            Some(Adjustment::None)
        );
    }

    #[test]
    fn a_shared_parameter_cannot_take_a_mutable_borrow() {
        let fixture = Adjustments::new();
        let number = fixture.number();
        assert_eq!(
            fixture.decide(&fixture.borrowed(number.clone()), &fixture.changed(number)),
            None
        );
    }

    #[test]
    fn extra_borrows_are_removed() {
        let fixture = Adjustments::new();
        let number = fixture.number();
        let borrowed = fixture.borrowed(number.clone());
        assert_eq!(
            fixture.decide(&fixture.borrowed(borrowed.clone()), &borrowed),
            Some(Adjustment::None)
        );
    }

    #[test]
    fn a_different_number_of_parameters_is_unfixable() {
        let fixture = Adjustments::new();
        let number = fixture.number();
        assert_eq!(fixture.decide_many(&[number], &[]), Err(1));
    }

    #[test]
    fn missing_borrows_are_not_added_to_a_reference() {
        let fixture = Adjustments::new();
        let number = fixture.number();
        let borrowed = fixture.borrowed(number);
        assert_eq!(
            fixture.decide(&borrowed, &fixture.borrowed(borrowed.clone())),
            None
        );
    }
}
