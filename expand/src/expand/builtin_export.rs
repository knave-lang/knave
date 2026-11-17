use crate::core::{BindingInfo, BindingKind, KernelType, Namespace};

pub(super) struct BuiltinExport {
    names: Vec<&'static str>,
    namespace: Namespace,
    kind: BindingKind,
}

impl BuiltinExport {
    pub fn value(name: &'static str, kind: BindingKind) -> Self {
        Self {
            names: vec![name],
            namespace: Namespace::Value,
            kind,
        }
    }

    pub fn kernel_type(kernel_type: KernelType) -> Self {
        Self {
            names: kernel_type.names().collect(),
            namespace: Namespace::Type,
            kind: BindingKind::KernelType(kernel_type),
        }
    }

    pub fn binding(&self) -> BindingInfo {
        BindingInfo::new(self.names[0].into(), self.kind, None)
    }

    pub fn namespace(&self) -> Namespace {
        self.namespace
    }

    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.names.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::BuiltinExport;
    use crate::core::{BindingKind, KernelType, Namespace, PrimitiveType};

    #[test]
    fn the_default_integer_is_one_binding_named_int_and_exported_as_both_names() {
        let export = BuiltinExport::kernel_type(KernelType::Primitive(PrimitiveType::I64));
        assert_eq!(export.names().collect::<Vec<_>>(), ["int", "i64"]);
        assert_eq!(export.binding().name(), "int");
        assert_eq!(export.namespace(), Namespace::Type);
    }

    #[test]
    fn the_default_float_is_one_binding_named_float_and_exported_as_both_names() {
        let export = BuiltinExport::kernel_type(KernelType::Primitive(PrimitiveType::F64));
        assert_eq!(export.names().collect::<Vec<_>>(), ["float", "f64"]);
        assert_eq!(export.binding().name(), "float");
    }

    #[test]
    fn a_kernel_type_shown_by_its_spelling_exports_one_name() {
        let export = BuiltinExport::kernel_type(KernelType::Primitive(PrimitiveType::I32));
        assert_eq!(export.names().collect::<Vec<_>>(), ["i32"]);
    }

    #[test]
    fn a_value_exports_one_name_in_the_value_namespace() {
        let export = BuiltinExport::value("true", BindingKind::Boolean(true));
        assert_eq!(export.names().collect::<Vec<_>>(), ["true"]);
        assert_eq!(export.namespace(), Namespace::Value);
    }
}
