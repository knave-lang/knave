use crate::core::{Literal, PrimitiveType};
use crate::types::{Type, TypedPattern, TypedPatternKind};

pub(super) struct PatternGuard<'p> {
    pattern: &'p TypedPattern,
}

impl<'p> PatternGuard<'p> {
    pub(super) fn of(pattern: &'p TypedPattern) -> Self {
        Self { pattern }
    }

    pub(super) fn is_needed(&self) -> bool {
        if self.pattern.boxed
            && matches!(
                &self.pattern.kind,
                TypedPatternKind::Variant(..) | TypedPatternKind::Literal(..)
            )
        {
            return true;
        }
        match &self.pattern.kind {
            TypedPatternKind::Wildcard | TypedPatternKind::Bind(_) => false,
            TypedPatternKind::Variant(_, arguments) | TypedPatternKind::Struct(_, arguments) => {
                arguments
                    .iter()
                    .any(|argument| PatternGuard::of(argument).is_needed())
            }
            TypedPatternKind::Literal(literal, ty) => {
                matches!(literal, Literal::String(_))
                    && matches!(ty, Type::Primitive(PrimitiveType::String))
            }
        }
    }
}
