use rustc_hash::FxHashSet;

use super::impl_bound_seeds::ImplBoundSeeds;
use super::typed::{CheckWork, TypedProgram};
use crate::diagnostic::Diagnostics;
use crate::source::Span;

pub(super) enum CheckRound {
    Checked(Box<TypedProgram>),
    Failed(Diagnostics),
    Grown(RoundGrowth, CheckWork),
}

pub(super) enum RoundGrowth {
    ImplBounds(ImplBoundSeeds),
    VectorBases(FxHashSet<Span>),
}
