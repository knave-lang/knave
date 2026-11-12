use crate::block_writer::BlockWriter;
use crate::generator_state::GeneratorState;
use crate::production::Production;
use crate::value_kind::ValueKind;

pub struct FreshBorrowingClosure;

impl FreshBorrowingClosure {
    const CLOSURE: &'static str = "(fn ((xs (ref (vec i64)))) (at xs 0))";
    const PERCENT: u64 = 20;
}

impl Production for FreshBorrowingClosure {
    fn write(&self, _blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        if !state.chance(Self::PERCENT) {
            return None;
        }
        Some(state.bind_local(ValueKind::BORROWING_CLOSURE, Self::CLOSURE.to_string()))
    }
}
