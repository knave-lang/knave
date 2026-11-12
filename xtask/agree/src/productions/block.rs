use crate::block_writer::BlockWriter;
use crate::generator_state::GeneratorState;
use crate::production::Production;

pub struct Block;

impl Production for Block {
    fn write(&self, blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        let body = blocks.block(state, None)?;
        Some(format!("(do {})", body.join(" ")))
    }
}
