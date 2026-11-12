use crate::block_writer::BlockWriter;
use crate::generator_state::GeneratorState;

pub trait Production: Sync {
    fn write(&self, blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String>;
}
