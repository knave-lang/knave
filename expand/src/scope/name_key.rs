use crate::core::Namespace;

#[derive(Clone, Copy)]
pub(crate) struct NameKey<'a> {
    pub(crate) name: &'a str,
    pub(crate) namespace: Namespace,
}

impl<'a> NameKey<'a> {
    pub(crate) fn new(name: &'a str, namespace: Namespace) -> Self {
        Self { name, namespace }
    }
}
