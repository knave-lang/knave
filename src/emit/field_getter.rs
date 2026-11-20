use super::fragment::{Code, Precedence};
use crate::core::FieldLabel;
use crate::types::{Type, Unborrowed};

pub(super) struct FieldGetter;

impl FieldGetter {
    pub(super) fn reads_generic_owner(owner: &Type) -> bool {
        matches!(Unborrowed::of(owner), Type::Generic(_))
    }

    pub(super) fn call(owner: Code, label: &FieldLabel) -> Code {
        let owner = if owner.is_atomic() {
            owner
        } else {
            owner.parenthesized()
        };
        let getter = Code::atom(format!(".{}()", label.rust_name()));
        Code::joined_outside_delimiters(vec![owner, getter], Precedence::Atom)
    }
}
