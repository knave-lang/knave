use super::by_value_signature::ByValueSignature;
use super::function_declaration::FunctionDeclaration;
use crate::core::{BindingId, ReferenceKind, TypeExpr, TypeExprKind};
use crate::types::{Type, TypedProgram};

pub struct ByValueSignatures<'t> {
    declared: &'t TypedProgram,
    inferred: &'t TypedProgram,
}

impl<'t> ByValueSignatures<'t> {
    pub fn new(declared: &'t TypedProgram, inferred: &'t TypedProgram) -> Self {
        Self { declared, inferred }
    }

    pub fn rewritten(&self, declaration: &FunctionDeclaration) -> Option<ByValueSignature> {
        let positions = self.references_inference_copies(declaration);
        (!positions.is_empty()).then(|| ByValueSignature {
            signature: declaration.signature.passing_by_value(&positions),
            positions,
        })
    }

    fn references_inference_copies(&self, declaration: &FunctionDeclaration) -> Vec<usize> {
        let declared = Self::parameters(self.declared, declaration.function);
        let inferred = Self::parameters(self.inferred, declaration.function);
        declaration
            .signature
            .parameters()
            .iter()
            .zip(declared.iter().zip(&inferred))
            .enumerate()
            .filter(|(_, (written, (declared, inferred)))| {
                Self::is_elided_shared_reference(written)
                    && Self::copies_referent(declared, inferred)
            })
            .map(|(position, _)| position)
            .collect()
    }

    fn parameters(typed: &TypedProgram, function: BindingId) -> Vec<Type> {
        typed
            .modules
            .iter()
            .flat_map(|module| &module.definitions)
            .find(|definition| definition.binding == function)
            .and_then(|definition| definition.scheme.body.parameters())
            .map(<[Type]>::to_vec)
            .unwrap_or_default()
    }

    fn is_elided_shared_reference(parameter: &TypeExpr) -> bool {
        matches!(
            parameter.kind(),
            TypeExprKind::Reference(ReferenceKind::Shared, None, _)
        )
    }

    fn copies_referent(declared: &Type, inferred: &Type) -> bool {
        matches!(declared, Type::Ref(_, referent) if referent.as_ref() == inferred)
    }
}
