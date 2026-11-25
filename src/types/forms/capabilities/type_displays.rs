use crate::types::ty::Type;

pub(in crate::types) trait TypeDisplays {
    fn show(&self, ty: &Type) -> String;
    fn describe(&mut self, ty: &Type) -> String;
}
