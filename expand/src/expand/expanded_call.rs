use super::macro_origin::MacroOrigin;
use crate::core::BindingId;
use crate::scope::Scope;

pub struct ExpandedCall<'c> {
    pub binding: BindingId,
    pub head_writer: Option<Scope>,
    pub origin: &'c MacroOrigin,
}
