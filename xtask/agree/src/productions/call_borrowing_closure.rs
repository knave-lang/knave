use crate::block_writer::BlockWriter;
use crate::generator_state::GeneratorState;
use crate::production::Production;
use crate::value_kind::ValueKind;

pub struct CallBorrowingClosure;

impl Production for CallBorrowingClosure {
    fn write(&self, _blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        let closure = state.some(ValueKind::BORROWING_CLOSURE)?;
        let vector = state.some(ValueKind::NUMBERS)?;
        Some(format!("(println ({closure} (ref {vector})))"))
    }
}
