use super::field_getter_name::FieldGetterName;
use super::field_trait::FieldTrait;
use super::field_traits::FieldTraits;
use super::written_refusal::{ReadRefusal, UseRefusal, WrittenRefusal};
use crate::core::Keyword;

pub(super) struct RefusalWording;

impl RefusalWording {
    pub(super) fn of(refusal: WrittenRefusal, field: &str) -> (String, String) {
        match refusal {
            WrittenRefusal::Read(read) => Self::read_wording(read, field),
            WrittenRefusal::Use(using) => Self::use_wording(using, field),
        }
    }

    fn read_wording(refusal: ReadRefusal, bare: &str) -> (String, String) {
        let field = &format!("{}{bare}", Keyword::MARKER);
        match refusal {
            ReadRefusal::Function => (
                format!("a `has` bound cannot read the function field `{field}`"),
                "name the struct in the signature to call its function field".to_string(),
            ),
            ReadRefusal::Open => (
                format!("the read of `{field}` in this `has` bound names a type variable"),
                "write the read with no type variable, as the field declares it".to_string(),
            ),
            ReadRefusal::OpenStruct => (
                format!(
                    "a struct with an open type argument reaches this `has` bound on `{field}`"
                ),
                "pass the struct with its type arguments fixed, or name the struct".to_string(),
            ),
            ReadRefusal::Otherwise => (
                format!("another `has` bound reads `{field}` as a different type"),
                format!("write `{field}` with one read type in every `has` bound"),
            ),
        }
    }

    fn use_wording(refusal: UseRefusal, bare: &str) -> (String, String) {
        let field = &format!("{}{bare}", Keyword::MARKER);
        match refusal {
            UseRefusal::ChangesField => (
                format!("this function changes `{field}` through a `has` bound"),
                "name the struct in the signature to change its fields".to_string(),
            ),
            UseRefusal::MovesOwner => (
                format!(
                    "this function moves a value that it reads `{field}` from through a `has` bound"
                ),
                "take the value as `(ref a)`, or name the struct in the signature".to_string(),
            ),
            UseRefusal::BorrowLeaves => (
                format!("a borrow read from `{field}` through a `has` bound leaves this function"),
                "return or store an owned value, or name the struct in the signature".to_string(),
            ),
            UseRefusal::NameTaken => (
                format!(
                    "the Rust trait `{}`, its method `{}` or the module `{}` for `{field}` would take a name the program already uses",
                    FieldTrait::rust_name(bare),
                    FieldGetterName::of(bare),
                    FieldTraits::MODULE
                ),
                "rename what the program names that way, or name the struct".to_string(),
            ),
        }
    }
}
