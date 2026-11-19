use crate::core::{BindingId, CoreModule, Provenance, WrittenUse};
use crate::diagnostic::Diagnostic;
use crate::expand::Expander;
use crate::reader::Form;
use crate::scope::Scope;
use crate::source::Span;

pub(in crate::expand) enum PendingItem {
    Def {
        binding: BindingId,
        annotation: Option<Form>,
        span: Span,
    },
    Signature(Form),
    Derive(Form),
    MismatchHelp(Form),
    TypeDefinition(PendingTypeDefinition),
    Impl(Scope, Form),
    Export(Form),
    Use(WrittenUse),
}

pub(in crate::expand) struct PendingTypeDefinition {
    kind: TypeDefinitionKind,
    binding: BindingId,
    form: Form,
}

#[derive(Clone, Copy)]
pub(in crate::expand) enum TypeDefinitionKind {
    Struct,
    Enum,
    Trait,
    RustTrait,
    RustType,
}

impl PendingItem {
    pub(in crate::expand) fn mark(self, provenance: Provenance) -> Self {
        match self {
            Self::Use(used) => Self::Use(WrittenUse { provenance, ..used }),
            item => item,
        }
    }
}

impl TypeDefinitionKind {
    pub(in crate::expand) fn pending(self, binding: BindingId, form: Form) -> PendingItem {
        PendingItem::TypeDefinition(PendingTypeDefinition {
            kind: self,
            binding,
            form,
        })
    }
}

impl PendingTypeDefinition {
    pub(in crate::expand) fn lower(
        self,
        expander: &mut Expander,
        core: &mut CoreModule,
    ) -> Result<(), Diagnostic> {
        let Self {
            kind,
            binding,
            form,
        } = self;
        match kind {
            TypeDefinitionKind::Struct => expander.lower_struct(binding, &form, core),
            TypeDefinitionKind::Enum => expander.lower_enum(binding, &form, core),
            TypeDefinitionKind::Trait => expander.lower_trait(binding, &form, core),
            TypeDefinitionKind::RustTrait => expander.lower_rust_trait(binding, &form, core),
            TypeDefinitionKind::RustType => expander.lower_rust_type(binding, &form, core),
        }
    }
}
