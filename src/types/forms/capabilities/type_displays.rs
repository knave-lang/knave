use crate::types::ty::Type;

pub(in crate::types) trait TypeDisplays {
    fn show(&self, ty: &Type) -> String;
}
