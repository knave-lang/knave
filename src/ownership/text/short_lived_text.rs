use super::reaching_text::ReachingText;
use super::source::Source;
use super::text_position::TextPositions;
use super::text_value::TextValue;
use crate::types::TypedExpr;

pub struct ShortLivedText;

impl ShortLivedText {
    pub fn of(expr: &TypedExpr) -> TextValue {
        if TextPositions::holds_top_text(&expr.ty) {
            TextValue::from(Source::Text(ReachingText::ASSUMED), expr.span)
        } else {
            TextValue::default()
        }
    }
}
