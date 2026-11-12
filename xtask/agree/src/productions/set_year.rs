use crate::block_writer::BlockWriter;
use crate::expressions::Expressions;
use crate::generator_state::GeneratorState;
use crate::production::Production;
use crate::reading::Reading;

pub struct SetYear;

impl Production for SetYear {
    fn write(&self, _blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        let car = Reading::CAR.read(state)?;
        let value = Expressions::number(state);
        Some(format!("(set! (:year {car}) {value})"))
    }
}
