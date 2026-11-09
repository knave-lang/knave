use rustc_hash::{FxHashMap, FxHashSet};

use super::functions::{FunctionInfo, ParamMode, ParamSlot};
use super::tail_calls::TailCalls;
use super::value_kinds::ValueKind;
use crate::core::BindingId;
use crate::types::{CopyRules, TypedArgument};

#[derive(Default)]
pub struct Recurrence {
    next_value_modes: Vec<ParamMode>,
    tail_calls: FxHashSet<*const TypedArgument>,
}

impl Recurrence {
    pub fn of(
        function: BindingId,
        info: &FunctionInfo,
        modes: &FxHashMap<BindingId, ParamMode>,
        copy: &CopyRules,
    ) -> Self {
        let next_value_modes = info
            .params
            .iter()
            .map(|slot| Self::next_value_mode(slot, modes[&slot.binding], copy))
            .collect();
        let tail_calls = TailCalls::of(Some(function), &info.lambda)
            .arguments()
            .iter()
            .map(|arguments| arguments.as_ptr())
            .collect();
        Self {
            next_value_modes,
            tail_calls,
        }
    }

    fn next_value_mode(slot: &ParamSlot, mode: ParamMode, copy: &CopyRules) -> ParamMode {
        let written_through = mode == ParamMode::Mut
            && slot.inferable
            && ValueKind::of(slot, copy) != ValueKind::Moving;
        if written_through {
            ParamMode::Copy
        } else {
            mode
        }
    }

    pub fn next_value_modes(&self) -> &[ParamMode] {
        &self.next_value_modes
    }

    pub fn is_tail_call(&self, arguments: &[TypedArgument]) -> bool {
        self.tail_calls.contains(&arguments.as_ptr())
    }
}
