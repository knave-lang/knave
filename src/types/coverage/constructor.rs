use super::cover_pattern::CoverPattern;
use crate::core::{BindingId, Literal};
use crate::types::Type;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Tag {
    Variant { owner: BindingId, index: usize },
    Struct(BindingId),
    Bool(bool),
}

pub(super) struct Constructor {
    tag: Tag,
    arguments: Vec<Type>,
}

impl Constructor {
    pub(super) fn new(tag: Tag, arguments: Vec<Type>) -> Self {
        Self { tag, arguments }
    }

    pub(super) fn tag(&self) -> Tag {
        self.tag
    }

    pub(super) fn arity(&self) -> usize {
        self.arguments.len()
    }

    pub(super) fn wild_arguments(&self) -> Vec<CoverPattern> {
        vec![CoverPattern::Wild; self.arity()]
    }

    pub(super) fn columns_with(&self, rest: &[Type]) -> Vec<Type> {
        [&self.arguments[..], rest].concat()
    }

    pub(super) fn arguments_of(&self, pattern: &CoverPattern) -> Option<Vec<CoverPattern>> {
        match pattern {
            CoverPattern::Wild => Some(self.wild_arguments()),
            CoverPattern::Variant {
                owner,
                index,
                arguments,
            } if self.tag
                == (Tag::Variant {
                    owner: *owner,
                    index: *index,
                }) =>
            {
                Some(arguments.clone())
            }
            CoverPattern::Struct { owner, arguments } if self.tag == Tag::Struct(*owner) => {
                Some(arguments.clone())
            }
            CoverPattern::Literal(Literal::Bool(value)) if self.tag == Tag::Bool(*value) => {
                Some(Vec::new())
            }
            _ => None,
        }
    }
}
