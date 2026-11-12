use crate::block_writer::BlockWriter;
use crate::expressions::Expressions;
use crate::generator_state::GeneratorState;
use crate::production::Production;

pub struct Print;

impl Production for Print {
    fn write(&self, _blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        Some(format!("(println {})", Expressions::printable(state)))
    }
}
