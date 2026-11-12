use crate::generator_state::GeneratorState;
use crate::value_kind::ValueKind;

#[derive(Clone, Copy)]
pub struct Reading {
    kind: ValueKind,
    shape: &'static str,
}

impl Reading {
    const SOURCE: &'static str = "{source}";

    pub const ELEMENT: Self = Self::of(ValueKind::Numbers, "(at {source} 0)");
    pub const LENGTH: Self = Self::of(ValueKind::Numbers, "(len {source})");
    pub const MAKE: Self = Self::of(ValueKind::Car, "(:make {source})");
    pub const YEAR: Self = Self::of(ValueKind::Car, "(:year {source})");
    pub const NUMBER: Self = Self::of(ValueKind::Number, "{source}");
    pub const BORROWED_ELEMENT: Self = Self::of(ValueKind::ElementBorrow, "{source}");
    pub const BORROWED_MAKE: Self = Self::of(ValueKind::MakeBorrow, "{source}");
    pub const LABEL: Self = Self::of(ValueKind::Tag, "(:label {source})");
    pub const HELD_BORROW: Self = Self::of(ValueKind::HeldBorrows, "(at {source} 0)");
    pub const READER_CALL: Self = Self::of(ValueKind::Reader, "({source})");
    pub const CHANGER_CALL: Self = Self::of(ValueKind::Changer, "({source})");
    pub const VECTOR: Self = Self::of(ValueKind::Numbers, "{source}");
    pub const CAR: Self = Self::of(ValueKind::Car, "{source}");
    pub const TAG_OF_MAKE: Self =
        Self::of(ValueKind::Car, "(->tag :label (as-str (:make {source})))");
    pub const HOLDING_BORROW: Self = Self::of(ValueKind::ElementBorrow, "(vec {source})");
    pub const LENGTH_READER: Self = Self::of(ValueKind::Numbers, "(fn () (len {source}))");
    pub const PUSHING_CLOSURE: Self = Self::of(ValueKind::Numbers, "(fn () (push! {source} 7))");

    const fn of(kind: ValueKind, shape: &'static str) -> Self {
        Self { kind, shape }
    }

    pub fn read(&self, state: &mut GeneratorState) -> Option<String> {
        let source = state.some(self.kind)?;
        Some(self.shape.replace(Self::SOURCE, source))
    }
}
