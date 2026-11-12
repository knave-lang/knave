use crate::block_writer::BlockWriter;
use crate::generator_state::GeneratorState;
use crate::production::Production;
use crate::reading::Reading;

pub struct CallChanger;

impl Production for CallChanger {
    fn write(&self, _blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        Reading::CHANGER_CALL.read(state)
    }
}
