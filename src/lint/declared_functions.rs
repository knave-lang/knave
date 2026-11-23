use super::declared_signature::DeclaredSignature;
use super::function_declaration::FunctionDeclaration;
use super::written_program::WrittenProgram;
use crate::core::{BindingId, CoreModule, ExprKind, Item, ItemKind};
use crate::expand::Program;

pub struct DeclaredFunctions<'p> {
    program: &'p Program,
    written: WrittenProgram<'p>,
}

impl<'p> DeclaredFunctions<'p> {
    pub fn new(program: &'p Program) -> Self {
        Self {
            program,
            written: WrittenProgram::new(program),
        }
    }

    pub fn candidates(&self) -> Vec<FunctionDeclaration> {
        self.program
            .project_modules()
            .flat_map(|module| self.declared_in(module))
            .collect()
    }

    pub fn declared_in(&self, module: &CoreModule) -> Vec<FunctionDeclaration> {
        module
            .items
            .iter()
            .filter(|item| self.written.is_written(item) && Self::is_type_form(module, item))
            .filter_map(|item| Self::written_declaration(module, item))
            .collect()
    }

    fn written_declaration(module: &CoreModule, item: &Item) -> Option<FunctionDeclaration> {
        let ItemKind::Signature(function, written) = &item.kind else {
            return None;
        };
        Some(FunctionDeclaration {
            function: *function,
            module: module.path.clone(),
            span: item.span,
            signature: DeclaredSignature::of(written)?,
            parameters: Self::lambda_parameters(module, *function)?,
        })
    }

    fn lambda_parameters(module: &CoreModule, function: BindingId) -> Option<Vec<BindingId>> {
        module
            .definitions()
            .find(|(binding, _)| *binding == function)
            .and_then(|(_, value)| match value.kind() {
                ExprKind::Fn(lambda) => {
                    Some(lambda.params.iter().map(|param| param.binding).collect())
                }
                _ => None,
            })
    }

    fn is_type_form(module: &CoreModule, item: &Item) -> bool {
        !item.hidden
            && !module
                .items
                .iter()
                .any(|other| matches!(other.kind, ItemKind::Def(..)) && other.span == item.span)
    }
}
