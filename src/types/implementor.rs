use super::ty::{Scheme, Type};

pub struct Implementor {
    pub scheme: Scheme,
    pub argument: Option<Type>,
}
