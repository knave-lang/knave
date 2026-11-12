use crate::block_writer::BlockWriter;
use crate::expressions::Expressions;
use crate::generator_state::GeneratorState;
use crate::production::Production;
use crate::value_kind::ValueKind;

pub struct FreshVector;

impl Production for FreshVector {
    fn write(&self, _blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        let first = Expressions::literal(state);
        let second = Expressions::literal(state);
        Some(state.bind_local(ValueKind::Numbers, format!("(vec {first} {second})")))
    }
}
