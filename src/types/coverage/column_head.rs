use super::constructor_tag::ConstructorTag;
use crate::core::Literal;

pub(super) enum ColumnHead<'p> {
    Any,
    Constructor(ConstructorTag),
    Literal(&'p Literal),
}
