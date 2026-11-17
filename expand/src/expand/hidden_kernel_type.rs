use super::namespace_problems::NamespaceProblems;
use crate::core::{BindingKind, KernelType, Namespace};
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub(super) struct HiddenKernelType<'name> {
    written: &'name str,
    kernel_type: KernelType,
}

impl<'name> HiddenKernelType<'name> {
    pub fn of(written: &'name str, namespace: Namespace, kind: BindingKind) -> Option<Self> {
        if namespace != Namespace::Type {
            return None;
        }
        let kernel_type = KernelType::named(written)?;
        let hides = kind != BindingKind::KernelType(kernel_type);
        hides.then_some(Self {
            written,
            kernel_type,
        })
    }

    pub fn at_definition(&self, span: Span) -> Diagnostic {
        NamespaceProblems::kernel_type_defined(self.written, self.kernel_type, span)
    }

    pub fn at_import(&self, span: Span) -> Diagnostic {
        NamespaceProblems::kernel_type_imported(self.written, self.kernel_type, span)
    }
}

#[cfg(test)]
mod tests {
    use super::HiddenKernelType;
    use crate::core::{BindingKind, KernelType, ModuleId, Namespace, PrimitiveType};

    struct HiddenFixture;

    impl HiddenFixture {
        const INT: BindingKind = BindingKind::KernelType(KernelType::Primitive(PrimitiveType::I64));

        fn hides(name: &str, namespace: Namespace, kind: BindingKind) -> bool {
            HiddenKernelType::of(name, namespace, kind).is_some()
        }

        fn program_struct() -> BindingKind {
            BindingKind::Struct(ModuleId::new(0))
        }
    }

    #[test]
    fn a_struct_named_after_a_kernel_type_hides_it() {
        let kind = HiddenFixture::program_struct();
        assert!(HiddenFixture::hides("int", Namespace::Type, kind));
    }

    #[test]
    fn the_other_name_of_a_kernel_type_is_hidden_too() {
        let kind = HiddenFixture::program_struct();
        assert!(HiddenFixture::hides("i64", Namespace::Type, kind));
    }

    #[test]
    fn a_kernel_type_under_either_of_its_names_hides_nothing() {
        assert!(!HiddenFixture::hides(
            "int",
            Namespace::Type,
            HiddenFixture::INT
        ));
        assert!(!HiddenFixture::hides(
            "i64",
            Namespace::Type,
            HiddenFixture::INT
        ));
    }

    #[test]
    fn a_value_named_after_a_kernel_type_hides_nothing() {
        let kind = HiddenFixture::program_struct();
        assert!(!HiddenFixture::hides("int", Namespace::Value, kind));
    }

    #[test]
    fn a_name_no_kernel_type_has_hides_nothing() {
        let kind = HiddenFixture::program_struct();
        assert!(!HiddenFixture::hides("point", Namespace::Type, kind));
    }
}
