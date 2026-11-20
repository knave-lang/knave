use knave_expand::mangle::NameMangler;

pub struct FieldGetterName;

impl FieldGetterName {
    pub fn of(field: &str) -> String {
        NameMangler::value(field)
    }
}
