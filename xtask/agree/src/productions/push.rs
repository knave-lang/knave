use crate::block_writer::BlockWriter;
use crate::expressions::Expressions;
use crate::generator_state::GeneratorState;
use crate::production::Production;
use crate::reading::Reading;

pub struct Push;

impl Production for Push {
    fn write(&self, _blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        let numbers = Reading::VECTOR.read(state)?;
        let value = Expressions::number(state);
        Some(format!("(push! {numbers} {value})"))
    }
}
