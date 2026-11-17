use super::ty::Type;
use crate::core::{BindingArena, BindingId, FieldLabel, FieldSpelling};

pub struct FieldPlace<'a> {
    pub root: BindingId,
    pub root_type: &'a Type,
    pub path: Vec<&'a FieldLabel>,
}

impl FieldPlace<'_> {
    pub fn reads_field(&self) -> bool {
        !self.path.is_empty()
    }

    pub fn pair_part_type(&self) -> Option<&Type> {
        self.path.iter().try_fold(self.root_type, |owner, label| {
            match (label.spelling, owner) {
                (FieldSpelling::Pair(_), Type::Named(_, parts)) => {
                    parts.get(label.name.parse::<usize>().ok()?)
                }
                _ => None,
            }
        })
    }

    pub fn written(&self, bindings: &BindingArena) -> String {
        let root = bindings.get(self.root).name().to_string();
        self.path.iter().fold(root, Self::field_text)
    }

    pub fn field_text(owner: String, label: &&FieldLabel) -> String {
        let field = label.spelled(&label.name);
        match label.spelling {
            FieldSpelling::Keyword | FieldSpelling::Pair(_) => format!("({field} {owner})"),
            FieldSpelling::Bare => format!("(field {owner} {field})"),
        }
    }
}
