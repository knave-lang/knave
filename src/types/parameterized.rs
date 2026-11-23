use crate::core::BindingId;

pub trait Parameterized {
    fn parameter_bindings(&self) -> impl Iterator<Item = BindingId> + Clone + '_;
}
