use std::rc::Rc;

use crate::core::BindingId;

#[derive(Clone)]
pub struct TypedAssociatedType {
    pub binding: BindingId,
    pub name: Rc<str>,
}
