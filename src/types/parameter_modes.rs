use rustc_hash::FxHashMap;

use crate::core::{BindingId, ReferenceKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParamMode {
    Copy,
    Ref,
    Mut,
    Own,
}

impl ParamMode {
    pub fn accepts(self, kind: ReferenceKind) -> bool {
        matches!(
            (kind, self),
            (ReferenceKind::Shared, ParamMode::Ref) | (ReferenceKind::Mutable, ParamMode::Mut)
        )
    }
}

#[derive(Default)]
pub struct ParameterModes {
    modes: FxHashMap<BindingId, ParamMode>,
}

impl ParameterModes {
    pub fn new(modes: FxHashMap<BindingId, ParamMode>) -> Self {
        Self { modes }
    }

    pub fn borrows(&self, param: BindingId) -> bool {
        matches!(
            self.modes.get(&param),
            Some(ParamMode::Ref | ParamMode::Mut)
        )
    }

    pub fn changes(&self, param: BindingId) -> bool {
        self.modes.get(&param) == Some(&ParamMode::Mut)
    }
}
