use crate::block_writer::BlockWriter;
use crate::expressions::Expressions;
use crate::generator_state::GeneratorState;
use crate::production::Production;
use crate::shape::Shape;
use crate::value_kind::ValueKind;

pub struct FreshFigure;

impl FreshFigure {
    const VARIANTS: [Shape; 3] = [
        Shape::of("(figure/circle {source})"),
        Shape::of("(figure/square {source})"),
        Shape::of("figure/point"),
    ];
}

impl Production for FreshFigure {
    fn write(&self, _blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        let variant = state.pick(&Self::VARIANTS)?;
        let payload = Expressions::literal(state);
        Some(state.bind_local(ValueKind::FIGURE, variant.filled(&payload)))
    }
}
