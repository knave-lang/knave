use crate::block_writer::BlockWriter;
use crate::generator_state::GeneratorState;
use crate::production::Production;
use crate::reading::Reading;
use crate::value_kind::ValueKind;

pub struct DerivedBinding {
    reading: Reading,
    result: ValueKind,
    percent: u64,
}

impl DerivedBinding {
    const ALWAYS: u64 = 100;
    const MOVE_PERCENT: u64 = 25;

    pub const BORROW_ELEMENT: Self = Self::of(Reading::ELEMENT, ValueKind::ELEMENT_BORROW);
    pub const BORROW_MAKE: Self = Self::of(Reading::MAKE, ValueKind::MAKE_BORROW);
    pub const MOVE_NUMBERS: Self = Self {
        percent: Self::MOVE_PERCENT,
        ..Self::of(Reading::VECTOR, ValueKind::NUMBERS)
    };
    pub const BIND_TAG: Self = Self::of(Reading::TAG_OF_MAKE, ValueKind::TAG);
    pub const HOLD_BORROWS: Self = Self::of(Reading::HOLDING_BORROW, ValueKind::HELD_BORROWS);
    pub const BIND_READER: Self = Self::of(Reading::LENGTH_READER, ValueKind::READER);
    pub const BORROW_ENTRY: Self = Self::of(Reading::MAP_ENTRY, ValueKind::MAP_ENTRY);
    pub const MOVE_MAP: Self = Self {
        percent: Self::MOVE_PERCENT,
        ..Self::of(Reading::MAP_GROWN, ValueKind::MAP)
    };
    pub const BIND_CHANGER: Self = Self::of(Reading::PUSHING_CLOSURE, ValueKind::CHANGER);

    const fn of(reading: Reading, result: ValueKind) -> Self {
        Self {
            reading,
            result,
            percent: Self::ALWAYS,
        }
    }
}

impl Production for DerivedBinding {
    fn write(&self, _blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        if !state.chance(self.percent) {
            return None;
        }
        let value = self.reading.read(state)?;
        Some(state.bind_local(self.result, value))
    }
}

#[cfg(test)]
mod tests {
    use super::DerivedBinding;
    use crate::block_writer::BlockWriter;
    use crate::generator_state::GeneratorState;
    use crate::production::Production;
    use crate::seed::Seed;
    use crate::value_kind::ValueKind;

    struct NoBlocks;

    impl BlockWriter for NoBlocks {
        fn block(
            &self,
            _state: &mut GeneratorState,
            _loop: Option<&'static str>,
        ) -> Option<Vec<String>> {
            None
        }
    }

    #[test]
    fn binds_a_borrow_only_once_a_vector_is_visible() {
        let mut state = GeneratorState::from(Seed::from(1));
        assert_eq!(
            DerivedBinding::BORROW_ELEMENT.write(&NoBlocks, &mut state),
            None
        );
        state.bind_local(ValueKind::NUMBERS, "(vec 1)".to_string());
        let vector = state.some(ValueKind::NUMBERS).expect("a vector is visible");
        let borrow = DerivedBinding::BORROW_ELEMENT
            .write(&NoBlocks, &mut state)
            .expect("a vector is visible");
        assert!(borrow.ends_with(&format!("(at {vector} 0))")));
        assert!(state.some(ValueKind::ELEMENT_BORROW).is_some());
    }
}
