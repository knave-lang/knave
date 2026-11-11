use crate::core::{BindingId, TraitMethod};

pub(super) struct DeclaredTrait<'d> {
    pub binding: BindingId,
    pub definer: &'d str,
    pub path: &'d str,
    pub path_owner: Option<BindingId>,
    pub self_variable: BindingId,
    pub methods: &'d [TraitMethod],
}
