use crate::block_writer::BlockWriter;
use crate::expressions::Expressions;
use crate::generator_state::GeneratorState;
use crate::production::Production;

pub struct Branch;

impl Production for Branch {
    fn write(&self, blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        let condition = Expressions::condition(state);
        let then = blocks.block(state, None)?;
        let otherwise = blocks.block(state, None)?;
        Some(format!(
            "(if {condition} (do {}) (do {}))",
            then.join(" "),
            otherwise.join(" ")
        ))
    }
}
