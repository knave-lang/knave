use rustc_hash::FxHashSet;

use crate::core::BindingId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemplateAuthor {
    Prelude,
    User,
}

impl TemplateAuthor {
    pub fn of(binding: BindingId, prelude: &FxHashSet<BindingId>) -> Self {
        if prelude.contains(&binding) {
            Self::Prelude
        } else {
            Self::User
        }
    }

    pub fn knows_what_an_iterator_captures(self) -> bool {
        self == Self::Prelude
    }
}
