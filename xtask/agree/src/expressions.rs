use crate::generator_state::GeneratorState;
use crate::reading::Reading;

pub struct Expressions;

impl Expressions {
    const REUSE_PERCENT: u64 = 40;
    const LITERALS: [u8; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 9];
    const NOTHING_TO_PRINT: &'static str = "0";
    const NUMBER_READINGS: [Reading; 4] = [
        Reading::NUMBER,
        Reading::ELEMENT,
        Reading::BORROWED_ELEMENT,
        Reading::YEAR,
    ];
    const PRINTABLE_READINGS: [Reading; 6] = [
        Reading::LENGTH,
        Reading::BORROWED_ELEMENT,
        Reading::BORROWED_MAKE,
        Reading::LABEL,
        Reading::HELD_BORROW,
        Reading::READER_CALL,
    ];

    pub fn literal(state: &mut GeneratorState) -> String {
        state
            .pick(&Self::LITERALS)
            .expect("literals exist")
            .to_string()
    }

    pub fn number(state: &mut GeneratorState) -> String {
        let available = Self::available(state, &Self::NUMBER_READINGS);
        let reused = state.chance(Self::REUSE_PERCENT);
        match Self::chosen(state, available) {
            Some(value) if reused => value,
            _ => Self::literal(state),
        }
    }

    pub fn printable(state: &mut GeneratorState) -> String {
        let mut available = Self::available(state, &Self::PRINTABLE_READINGS);
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

    fn available(state: &mut GeneratorState, readings: &[Reading]) -> Vec<String> {
        readings
            .iter()
            .filter_map(|reading| reading.read(state))
            .collect()
    }
}
