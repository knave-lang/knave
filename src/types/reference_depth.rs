use super::ty::Type;
use super::typed::Borrow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ReferenceDepth {
    target: usize,
    through_borrow: bool,
}

impl ReferenceDepth {
    pub(super) const VALUE: Self = Self {
        target: 0,
        through_borrow: false,
    };
    pub(super) const VALUE_THROUGH_BORROW: Self = Self {
        target: 0,
        through_borrow: true,
    };

    pub(super) fn new(target: usize) -> Self {
        Self {
            target,
            through_borrow: false,
        }
    }

    pub(super) fn wrap(self, referent: Type) -> Type {
        (0..self.target).fold(referent, |inner, _| Type::shared(inner))
    }

    pub(super) fn borrow_from(self, depth: usize) -> Borrow {
        let target = self.target;
        if depth == target {
            Borrow::None
        } else if depth < target {
            Borrow::Shared
        } else if self.through_borrow {
            Borrow::Passthrough
        } else if target == 0 {
            Borrow::Deref(depth)
        } else {
            Borrow::Collapse(depth - target)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReferenceDepth;
    use crate::core::PrimitiveType;
    use crate::types::{Borrow, Type};

    #[test]
    fn a_value_at_the_target_depth_passes_as_is() {
        assert_eq!(ReferenceDepth::new(2).borrow_from(2), Borrow::None);
    }

    #[test]
    fn a_value_above_the_target_depth_is_borrowed() {
        assert_eq!(ReferenceDepth::new(1).borrow_from(0), Borrow::Shared);
    }

    #[test]
    fn a_reference_passed_by_value_is_dereferenced() {
        assert_eq!(ReferenceDepth::VALUE.borrow_from(2), Borrow::Deref(2));
    }

    #[test]
    fn a_reference_passed_through_to_a_value_stays_a_borrow() {
        assert_eq!(
            ReferenceDepth::VALUE_THROUGH_BORROW.borrow_from(1),
            Borrow::Passthrough
        );
    }

    #[test]
    fn a_value_passed_through_a_borrow_passes_a_value_as_is() {
        assert_eq!(
            ReferenceDepth::VALUE_THROUGH_BORROW.borrow_from(0),
            Borrow::None
        );
    }

    #[test]
    fn a_deeper_reference_collapses_to_the_target_depth() {
        assert_eq!(ReferenceDepth::new(1).borrow_from(3), Borrow::Collapse(2));
    }

    #[test]
    fn wrapping_adds_one_shared_layer_per_depth() {
        let int = Type::Primitive(PrimitiveType::I64);
        let wrapped = ReferenceDepth::new(2).wrap(int.clone());
        assert_eq!(wrapped, Type::shared(Type::shared(int)));
    }
}
