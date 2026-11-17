use super::ty::Type;
use crate::core::{BindingArena, BindingId, FieldLabel, FieldSpelling};

pub struct FieldPlace<'a> {
    pub root: BindingId,
    pub root_type: &'a Type,
    pub path: Vec<&'a FieldLabel>,
}

impl<'a> FieldPlace<'a> {
    pub fn whole(root: BindingId, root_type: &'a Type) -> Self {
        let path = Vec::new();
        Self {
            root,
            root_type,
            path,
        }
    }

    pub fn reads_field(&self) -> bool {
        !self.path.is_empty()
    }

    pub fn pair_part_type(&self) -> Option<&Type> {
        self.path.iter().try_fold(self.root_type, |owner, label| {
            match (label.spelling, owner) {
                (FieldSpelling::Pair(position), Type::Named(_, parts)) => {
                    parts.get(position.index())
                }
                _ => None,
            }
        })
    }

    pub fn written(&self, bindings: &BindingArena) -> String {
        self.written_from(bindings.get(self.root).name().to_string())
    }

    pub fn written_borrowed(&self, bindings: &BindingArena) -> String {
        self.written_from(Self::borrowed_root(bindings.get(self.root).name()))
    }

    pub fn borrowed_root(name: &str) -> String {
        format!("(ref {name})")
    }

    fn written_from(&self, root: String) -> String {
        self.path.iter().fold(root, Self::field_text)
    }

    fn field_text(owner: String, label: &&FieldLabel) -> String {
        let field = label.spelled(&label.name);
        match label.spelling {
            FieldSpelling::Keyword | FieldSpelling::Pair(_) => format!("({field} {owner})"),
            FieldSpelling::Bare => format!("(field {owner} {field})"),
        }
    }
}
