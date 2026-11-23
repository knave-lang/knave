use super::field_getter_name::FieldGetterName;
use super::field_trait::FieldTrait;
use super::field_traits::FieldTraits;
use super::written_refusal::{ReadRefusal, UseRefusal, WrittenRefusal};
use crate::core::Keyword;

pub(super) struct RefusalWording {
    remedy: &'static str,
}

impl RefusalWording {
    pub(super) fn written() -> Self {
        Self {
            remedy: "name the struct in the signature, or write one function for each struct",
        }
    }

    pub(super) fn inferred() -> Self {
        Self {
            remedy: "write one function for each struct",
        }
    }

    pub(super) const ONE_READ_PER_FUNCTION: &'static str =
        "write one function for each struct, or declare the field with one type in every struct";

    pub(super) fn differing_reads(bare: &str, reads: &[(String, String)]) -> String {
        let listed: Vec<String> = (reads.iter())
            .map(|(owner, read)| format!("`{owner}` as `{read}`"))
            .collect();
        format!(
            "the structs that reach this function read `{}{bare}` as different types: {}",
            Keyword::MARKER,
            listed.join(", ")
        )
    }

    pub(super) fn agreed_read(bare: &str, read: &str) -> String {
        format!(
            "the structs that reach this function read `{}{bare}` as `{read}`",
            Keyword::MARKER
        )
    }

    pub(super) fn of(&self, refusal: WrittenRefusal, field: &str) -> (String, String) {
        match refusal {
            WrittenRefusal::Read(read) => self.read_wording(read, field),
            WrittenRefusal::Use(using) => self.use_wording(using, field),
        }
    }

    fn read_wording(&self, refusal: ReadRefusal, bare: &str) -> (String, String) {
        let remedy = self.remedy;
        let field = &format!("{}{bare}", Keyword::MARKER);
        match refusal {
            ReadRefusal::Differing => (
                format!(
                    "the structs that reach this `has` bound read `{field}` as different types"
                ),
                Self::ONE_READ_PER_FUNCTION.to_string(),
            ),
            ReadRefusal::Function => (
                format!("a `has` bound cannot read the function field `{field}`"),
                format!("{remedy} to read or call its function field"),
            ),
            ReadRefusal::Open => (
                format!("the read of `{field}` in this `has` bound names a type variable"),
                "write the read with no type variable".to_string(),
            ),
            ReadRefusal::OpenStruct => (
                format!(
                    "a struct with an open type argument reaches this `has` bound on `{field}`"
                ),
                format!("pass the struct with its type arguments fixed, or {remedy}"),
            ),
            ReadRefusal::Otherwise => (
                format!("another `has` bound reads `{field}` as a different type"),
                format!("write `{field}` with one read type in every `has` bound"),
            ),
        }
    }

    fn use_wording(&self, refusal: UseRefusal, bare: &str) -> (String, String) {
        let field = &format!("{}{bare}", Keyword::MARKER);
        let remedy = self.remedy;
        match refusal {
            UseRefusal::ChangesField => (
                format!("this function changes `{field}` through a `has` bound"),
                format!("{remedy} to change its fields"),
            ),
            UseRefusal::MovesOwner => (
                format!(
                    "this function moves a value that it reads `{field}` from through a `has` bound"
                ),
                format!("take the value as `(ref a)`, or {remedy}"),
            ),
            UseRefusal::BorrowLeaves => (
                format!("a borrow read from `{field}` through a `has` bound leaves this function"),
                format!("return or store an owned value, or {remedy}"),
            ),
            UseRefusal::ValueLeaves => (
                format!(
                    "a value read from `{field}` through a `has` bound holds a borrow that leaves this function"
                ),
                format!("return or store an owned value, or {remedy}"),
            ),
            UseRefusal::NameTaken => (
                format!(
                    "the Rust trait `{}`, its method `{}` or the module `{}` for `{field}` would take a name the program already uses",
                    FieldTrait::rust_name(bare),
                    FieldGetterName::of(bare),
                    FieldTraits::MODULE
                ),
                format!("rename what the program names that way, or {remedy}"),
            ),
        }
    }
}
