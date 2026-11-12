use crate::block_writer::BlockWriter;
use crate::generator_state::GeneratorState;
use crate::production::Production;

pub struct Repeat;

impl Repeat {
    const COUNTER: &'static str = "i";
}

impl Production for Repeat {
    fn write(&self, blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        let body = blocks.block(state, Some(Self::COUNTER))?;
        Some(format!(
            "(loop ({counter} 0) (if (< {counter} 2) (do {} (recur (+ {counter} 1))) (do)))",
            body.join(" "),
            counter = Self::COUNTER
        ))
    }
}
