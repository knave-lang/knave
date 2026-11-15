use std::iter;

use crate::core::{BindingInfo, BindingKind, KernelType, Namespace};

pub(super) struct BuiltinExport {
    name: &'static str,
    namespace: Namespace,
    kind: BindingKind,
    differing_spelling: Option<&'static str>,
}

impl BuiltinExport {
    pub fn value(name: &'static str, kind: BindingKind) -> Self {
        Self {
            name,
            namespace: Namespace::Value,
            kind,
            differing_spelling: None,
        }
    }

    pub fn kernel_type(kernel_type: KernelType) -> Self {
        Self {
            name: kernel_type.name(),
            namespace: Namespace::Type,
            kind: BindingKind::KernelType(kernel_type),
            differing_spelling: kernel_type.differing_spelling(),
        }
    }

    pub fn binding(&self) -> BindingInfo {
        BindingInfo::new(self.name.into(), self.kind, None)
    }

    pub fn namespace(&self) -> Namespace {
        self.namespace
    }

    pub fn names(&self) -> impl Iterator<Item = &'static str> + use<> {
        iter::once(self.name).chain(self.differing_spelling)
    }
}

#[cfg(test)]
mod tests {
    use super::BuiltinExport;
    use crate::core::{BindingKind, KernelType, Namespace, PrimitiveType};

    #[test]
    fn a_type_with_a_differing_spelling_exports_both_names() {
        let export = BuiltinExport {
            name: "int",
            differing_spelling: Some("i64"),
            ..BuiltinExport::kernel_type(KernelType::Primitive(PrimitiveType::I64))
        };
        assert_eq!(export.names().collect::<Vec<_>>(), ["int", "i64"]);
    }

    #[test]
    fn a_kernel_type_exports_its_shown_name_in_the_type_namespace() {
        let export = BuiltinExport::kernel_type(KernelType::Primitive(PrimitiveType::I64));
        assert_eq!(export.names().collect::<Vec<_>>(), ["i64"]);
        assert_eq!(export.namespace(), Namespace::Type);
        assert_eq!(export.binding().name(), "i64");
    }

    #[test]
    fn a_value_exports_one_name_in_the_value_namespace() {
        let export = BuiltinExport::value("true", BindingKind::Boolean(true));
        assert_eq!(export.names().collect::<Vec<_>>(), ["true"]);
        assert_eq!(export.namespace(), Namespace::Value);
    }
}
