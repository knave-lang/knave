use std::rc::Rc;

use crate::core::BindingId;

pub(super) struct GivenHolders {
    pub(super) receiver: Option<BindingId>,
    pub(super) impl_variables: Vec<Rc<str>>,
}
