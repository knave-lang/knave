use super::ty::Type;

pub(super) struct PassedTypes<'t> {
    pub(super) parameter: &'t Type,
    pub(super) argument: &'t Type,
}
