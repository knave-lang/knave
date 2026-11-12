use crate::block_writer::BlockWriter;
use crate::generator_state::GeneratorState;
use crate::production::Production;
use crate::shape::Shape;
use crate::value_kind::ValueKind;

pub struct CallBorrowingClosure;

impl Production for CallBorrowingClosure {
    fn write(&self, _blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        let closure = state.some(ValueKind::BORROWING_CLOSURE)?;
        let vector = state.some(ValueKind::NUMBERS)?;
        let call = format!("({closure} {})", Shape::BORROWED.filled(vector));
        Some(Shape::PRINTED.filled(&call))
    }
}
