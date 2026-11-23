use std::ops::ControlFlow;
use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::impl_bound_seeds::ImplBoundSeeds;
use super::round_failure::RoundFailure;
use super::settled_read::SettledRead;
use super::typed::{CheckWork, TypedProgram};
use crate::diagnostic::Diagnostics;
use crate::source::Span;

pub(super) enum CheckRound {
    Checked(Box<TypedProgram>),
    Failed(Diagnostics),
    FailedWithProgram(RoundFailure, Box<TypedProgram>),
    Grown(RoundGrowth, CheckWork),
}

impl CheckRound {
    pub(super) fn next(
        self,
    ) -> ControlFlow<Result<TypedProgram, Diagnostics>, (RoundGrowth, CheckWork)> {
        match self {
            Self::Checked(typed) => ControlFlow::Break(Ok(*typed)),
            Self::Failed(diagnostics) => ControlFlow::Break(Err(diagnostics)),
            Self::FailedWithProgram(failure, _) => ControlFlow::Break(Err(failure.alone())),
            Self::Grown(growth, checking) => ControlFlow::Continue((growth, checking)),
        }
    }
}

pub(super) enum RoundGrowth {
    ImplBounds(ImplBoundSeeds),
    VectorBases(FxHashSet<Span>),
    SharedFields(FxHashSet<Rc<str>>),
    SettledReads(FxHashMap<Span, SettledRead>),
}
