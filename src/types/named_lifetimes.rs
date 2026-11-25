use super::ty::Lifetime;
use crate::core::BindingId;

pub trait NamedLifetimes {
    fn of_named(&self, binding: BindingId) -> Option<Lifetime>;
}
