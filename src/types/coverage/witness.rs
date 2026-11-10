use super::constructor::{Constructor, Tag};
use crate::core::BindingId;

#[derive(Clone)]
pub(super) enum Witness {
    Wild,
    Variant {
        owner: BindingId,
        index: usize,
        arguments: Vec<Witness>,
    },
    Struct {
        owner: BindingId,
        arguments: Vec<Witness>,
    },
    Bool(bool),
}

impl Witness {
    pub(super) fn built_by(constructor: &Constructor, arguments: &[Witness]) -> Self {
        match constructor.tag() {
            Tag::Variant { owner, index } => Witness::Variant {
                owner,
                index,
                arguments: arguments.to_vec(),
            },
            Tag::Struct(owner) => Witness::Struct {
                owner,
                arguments: arguments.to_vec(),
            },
            Tag::Bool(value) => Witness::Bool(value),
        }
    }
}
