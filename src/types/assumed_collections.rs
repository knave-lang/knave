use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::typed::{TypedExpr, TypedExprKind, TypedLeaf};
use crate::core::{
    BindingId, CollectionForm, Expr, ExprKind, ExpressionForm, KernelForm, MapOperation,
    PairPosition,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollectionAssumption {
    PushTarget(Rc<str>),
    GetCollection,
    PairPartCollection(PairPosition),
}

impl CollectionAssumption {
    const PUSH_AT: KernelForm =
        KernelForm::Expression(ExpressionForm::Collection(CollectionForm::PushAt));

    pub fn push_target(written_by_macro: Option<Rc<str>>) -> Self {
        Self::PushTarget(written_by_macro.unwrap_or_else(|| Rc::from(Self::PUSH_AT.name())))
    }

    pub fn operation(&self) -> &str {
        match self {
            Self::PushTarget(written) => written,
            Self::GetCollection => "get",
            Self::PairPartCollection(position) => MapOperation::from(*position).keyword(),
        }
    }

    pub fn reading(&self) -> &'static str {
        match self {
            Self::PushTarget(_) => "a target of unknown type as a vector",
            Self::GetCollection => "a collection of unknown type as a vector or a slice",
            Self::PairPartCollection(_) => {
                "a value of unknown type as a vector or a slice, not a pair"
            }
        }
    }
}

#[derive(Clone, Copy)]
struct ParameterSlot {
    function: BindingId,
    position: usize,
}

#[derive(Default)]
pub struct AssumedCollections {
    parameters: FxHashMap<BindingId, ParameterSlot>,
    assumptions: FxHashMap<(BindingId, usize), CollectionAssumption>,
}

impl AssumedCollections {
    pub fn record_parameters(&mut self, function: BindingId, value: &Expr) {
        let ExprKind::Fn(lambda) = value.kind() else {
            return;
        };
        for (position, param) in lambda.params.iter().enumerate() {
            let slot = ParameterSlot { function, position };
            self.parameters.insert(param.binding, slot);
        }
    }

    pub fn assume(&mut self, target: &TypedExpr, assumption: CollectionAssumption) {
        let Some(slot) =
            Self::parameter_of(target).and_then(|binding| self.parameters.get(&binding))
        else {
            return;
        };
        self.assumptions
            .insert((slot.function, slot.position), assumption);
    }

    pub fn get(&self, function: BindingId, position: usize) -> Option<CollectionAssumption> {
        self.assumptions.get(&(function, position)).cloned()
    }

    fn parameter_of(target: &TypedExpr) -> Option<BindingId> {
        match &target.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => Some(*binding),
            TypedExprKind::Borrow(_, inner) => Self::parameter_of(inner),
            _ => None,
        }
    }
}
