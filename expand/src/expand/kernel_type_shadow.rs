use super::namespace_problems::NamespaceProblems;
use crate::core::{BindingKind, KernelType, Namespace};
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub(super) struct KernelTypeShadow<'name> {
    written: &'name str,
    kernel_type: KernelType,
}

impl<'name> KernelTypeShadow<'name> {
    pub fn of(written: &'name str, namespace: Namespace, kind: BindingKind) -> Option<Self> {
        if namespace != Namespace::Type {
            return None;
        }
        let kernel_type = KernelType::named(written)?;
        let shadows = kind != BindingKind::KernelType(kernel_type);
        shadows.then_some(Self {
            written,
            kernel_type,
        })
    }

    pub fn at_definition(&self, span: Span) -> Diagnostic {
        NamespaceProblems::kernel_type_defined(self.written, self.kernel_type.name(), span)
    }

    pub fn at_import(&self, span: Span) -> Diagnostic {
        NamespaceProblems::kernel_type_imported(self.written, self.kernel_type.name(), span)
    }
}

#[cfg(test)]
mod tests {
    use super::KernelTypeShadow;
    use crate::core::{BindingKind, KernelType, ModuleId, Namespace, PrimitiveType};

    struct ShadowFixture;

    impl ShadowFixture {
        const INT: BindingKind = BindingKind::KernelType(KernelType::Primitive(PrimitiveType::I64));

        fn shadows(name: &str, namespace: Namespace, kind: BindingKind) -> bool {
            KernelTypeShadow::of(name, namespace, kind).is_some()
        }

        fn program_struct() -> BindingKind {
            BindingKind::Struct(ModuleId::new(0))
        }
    }

    #[test]
    fn a_struct_named_after_a_kernel_type_shadows_it() {
        let kind = ShadowFixture::program_struct();
        assert!(ShadowFixture::shadows("int", Namespace::Type, kind));
    }

    #[test]
    fn the_other_name_of_a_kernel_type_is_shadowed_too() {
        let kind = ShadowFixture::program_struct();
        assert!(ShadowFixture::shadows("i64", Namespace::Type, kind));
    }

    #[test]
    fn a_kernel_type_under_either_of_its_names_shadows_nothing() {
        assert!(!ShadowFixture::shadows(
            "int",
            Namespace::Type,
            ShadowFixture::INT
        ));
        assert!(!ShadowFixture::shadows(
            "i64",
            Namespace::Type,
            ShadowFixture::INT
        ));
    }

    #[test]
    fn a_value_named_after_a_kernel_type_shadows_nothing() {
        let kind = ShadowFixture::program_struct();
        assert!(!ShadowFixture::shadows("int", Namespace::Value, kind));
    }

    #[test]
    fn a_name_no_kernel_type_has_shadows_nothing() {
        let kind = ShadowFixture::program_struct();
        assert!(!ShadowFixture::shadows("point", Namespace::Type, kind));
    }
}
