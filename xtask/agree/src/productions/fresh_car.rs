use crate::block_writer::BlockWriter;
use crate::generator_state::GeneratorState;
use crate::production::Production;
use crate::value_kind::ValueKind;

pub struct FreshCar;

impl FreshCar {
    const CAR: &'static str = "(->car :make (str \"Leaf\") :year 2020)";
}

impl Production for FreshCar {
    fn write(&self, _blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        Some(state.bind_local(ValueKind::Car, Self::CAR.to_string()))
    }
}
