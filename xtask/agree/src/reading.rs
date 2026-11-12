use crate::generator_state::GeneratorState;
use crate::shape::Shape;
use crate::value_kind::ValueKind;

#[derive(Clone, Copy)]
pub struct Reading {
    kind: ValueKind,
    shape: Shape,
}

impl Reading {
    pub const ELEMENT: Self = Self::of(ValueKind::NUMBERS, "(at {source} 0)");
    pub const SHAPE_MATCH: Self = Self::of(
        ValueKind::SHAPE,
        "(match {source} ((shape/circle r) r) ((shape/square w) w) (shape/point 0))",
    );
    pub const SHAPE_TWICE: Self = Self::of(ValueKind::SHAPE, "(twice {source})");
    pub const CAR_SIZE: Self = Self::of(ValueKind::CAR, "(size {source})");
    pub const MAP_ENTRY: Self = Self::of(ValueKind::MAP, "(get {source} 1)");
    pub const MAP_GROWN: Self = Self::of(ValueKind::MAP, "(assoc {source} 3 30)");
    pub const MAKE: Self = Self::of(ValueKind::CAR, "(:make {source})");
    pub const CHANGER_CALL: Self = Self::of(ValueKind::CHANGER, "({source})");
    pub const VECTOR: Self = Self::of(ValueKind::NUMBERS, "{source}");
    pub const CAR: Self = Self::of(ValueKind::CAR, "{source}");
    pub const TAG_OF_MAKE: Self =
        Self::of(ValueKind::CAR, "(->tag :label (as-str (:make {source})))");
    pub const HOLDING_BORROW: Self = Self::of(ValueKind::ELEMENT_BORROW, "(vec {source})");
    pub const LENGTH_READER: Self = Self::of(ValueKind::NUMBERS, "(fn () (len {source}))");
    pub const PUSHING_CLOSURE: Self = Self::of(ValueKind::NUMBERS, "(fn () (push! {source} 7))");

    const fn of(kind: ValueKind, shape: &'static str) -> Self {
        Self {
            kind,
            shape: Shape::of(shape),
        }
    }

    pub fn read(&self, state: &mut GeneratorState) -> Option<String> {
        let source = state.some(self.kind)?;
        Some(self.shape.filled(source))
    }
}
