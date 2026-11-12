use crate::block_writer::BlockWriter;
use crate::expressions::Expressions;
use crate::generator_state::GeneratorState;
use crate::production::Production;
use crate::value_kind::ValueKind;

pub struct FreshNumber;

impl Production for FreshNumber {
    fn write(&self, _blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        let value = Expressions::number(state);
        Some(state.bind_local(ValueKind::Number, value))
    }
}
