use crate::block_writer::BlockWriter;
use crate::expressions::Expressions;
use crate::generator_state::GeneratorState;
use crate::production::Production;
use crate::shape::Shape;
use crate::value_kind::ValueKind;

pub struct FreshShape;

impl FreshShape {
    const VARIANTS: [Shape; 3] = [
        Shape::of("(shape/circle {source})"),
        Shape::of("(shape/square {source})"),
        Shape::of("shape/point"),
    ];
}

impl Production for FreshShape {
    fn write(&self, _blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        let variant = state.pick(&Self::VARIANTS)?;
        let payload = Expressions::literal(state);
        Some(state.bind_local(ValueKind::SHAPE, variant.filled(&payload)))
    }
}
