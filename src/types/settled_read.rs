use super::ty::Type;
use crate::core::BindingId;

#[derive(Clone, Debug)]
pub(super) enum SettledRead {
    Read(Type),
    Owner(BindingId),
}
