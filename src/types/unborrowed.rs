use super::ty::Type;

pub struct Unborrowed;

impl Unborrowed {
    pub fn of(value: &Type) -> &Type {
        match value {
            Type::Ref(_, inner) | Type::Mut(_, inner) => Self::of(inner),
            _ => value,
        }
    }
}
