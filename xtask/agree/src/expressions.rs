use crate::generator_state::GeneratorState;
use crate::reading::Reading;
use crate::value_kind::ValueKind;

pub struct Expressions;

impl Expressions {
    const REUSE_PERCENT: u64 = 40;
    const LITERALS: [u8; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 9];
    const NOTHING_TO_PRINT: &'static str = "0";
    const NUMBER_READINGS: [Reading; 4] = [
        Reading::ELEMENT,
        Reading::SHAPE_MATCH,
        Reading::SHAPE_TWICE,
        Reading::CAR_SIZE,
    ];
    const NUMBER_KINDS: [ValueKind; 3] =
        [ValueKind::NUMBER, ValueKind::ELEMENT_BORROW, ValueKind::CAR];

    pub fn literal(state: &mut GeneratorState) -> String {
        state
            .pick(&Self::LITERALS)
            .expect("literals exist")
            .to_string()
    }

    pub fn number(state: &mut GeneratorState) -> String {
        let mut available = Self::printed_locals(state, &Self::NUMBER_KINDS);
        available.extend(
            Self::NUMBER_READINGS
                .iter()
                .filter_map(|reading| reading.read(state)),
        );
        let reused = state.chance(Self::REUSE_PERCENT);
        match Self::chosen(state, available) {
            Some(value) if reused => value,
            _ => Self::literal(state),
        }
    }

    pub fn printable(state: &mut GeneratorState) -> String {
        let mut available = Self::printed_locals(state, &ValueKind::ALL);
        available.push(Self::number(state));
        Self::chosen(state, available).unwrap_or_else(|| Self::NOTHING_TO_PRINT.to_string())
    }

    pub fn condition(state: &mut GeneratorState) -> String {
        let left = Self::number(state);
        let right = Self::literal(state);
        format!("(> {left} {right})")
    }

    fn chosen(state: &mut GeneratorState, mut available: Vec<String>) -> Option<String> {
        let positions: Vec<usize> = (0..available.len()).collect();
        state
            .pick(&positions)
            .map(|position| available.swap_remove(position))
    }

    fn printed_locals(state: &mut GeneratorState, kinds: &[ValueKind]) -> Vec<String> {
        kinds
            .iter()
            .filter_map(|kind| kind.printed(state.some(*kind)?))
            .collect()
    }
}
