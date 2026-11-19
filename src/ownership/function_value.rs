use super::parameter_adjustments::Adjustment;
use crate::core::BindingId;
use crate::types::Type;

pub(super) enum FunctionValue {
    Named { binding: BindingId, ty: Type },
    Template { binding: BindingId, ty: Type },
}

impl FunctionValue {
    pub(super) fn binding(&self) -> BindingId {
        match self {
            Self::Named { binding, .. } | Self::Template { binding, .. } => *binding,
        }
    }

    pub(super) fn ty(&self) -> &Type {
        match self {
            Self::Named { ty, .. } | Self::Template { ty, .. } => ty,
        }
    }

    pub(super) fn needs_wrapper(&self, expected: &Type, adjustments: &[Adjustment]) -> bool {
        match self {
            Self::Named { ty, .. } => expected != ty,
            Self::Template { .. } => adjustments.iter().any(|adjustment| adjustment.changes()),
        }
    }

    pub(super) fn reports_mismatch(&self) -> bool {
        matches!(self, Self::Named { .. })
    }
}
