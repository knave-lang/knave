use std::path::PathBuf;

use super::declaration_key::DeclarationKey;
use super::declared_signature::DeclaredSignature;
use super::function_declaration::FunctionDeclaration;
use super::library_declarations::LibraryDeclarations;
use crate::core::{BindingId, CoreModule, ExprKind, Item, ItemKind, ModulePath};
use crate::expand::Program;

pub struct DeclaredFunctions<'p> {
    program: &'p Program,
}

impl<'p> DeclaredFunctions<'p> {
    pub fn new(program: &'p Program) -> Self {
        Self { program }
    }

    pub fn candidates(&self) -> Vec<FunctionDeclaration> {
        self.program
            .project_modules()
            .flat_map(|module| self.module_declarations(module))
            .collect()
    }

    pub fn library_declarations(&self, removable: &[BindingId]) -> Vec<LibraryDeclarations> {
        self.library_modules()
            .map(|module| LibraryDeclarations {
                module: module.path.clone(),
                names: self
                    .module_declarations(module)
                    .into_iter()
                    .filter(|declaration| removable.contains(&declaration.function))
                    .map(|declaration| self.name(declaration.function))
                    .collect(),
            })
            .filter(|library| !library.names.is_empty())
            .collect()
    }

    pub fn named_in(&self, path: &ModulePath, names: &[String]) -> Vec<FunctionDeclaration> {
        self.program
            .modules
            .iter()
            .filter(|module| &module.path == path)
            .flat_map(|module| self.module_declarations(module))
            .filter(|declaration| names.contains(&self.name(declaration.function)))
            .collect()
    }

    pub fn key(&self, declaration: &FunctionDeclaration, file: Option<PathBuf>) -> DeclarationKey {
        let function = self.name(declaration.function);
        match file {
            Some(file) => DeclarationKey::InFile(file, function),
            None => DeclarationKey::InModule(declaration.module.clone(), function),
        }
    }

    pub fn name(&self, function: BindingId) -> String {
        self.program.bindings.get(function).name().to_string()
    }

    fn library_modules(&self) -> impl Iterator<Item = &'p CoreModule> {
        let entry = self.program.entry.module;
        self.program
            .project_modules()
            .filter(move |module| module.id != entry)
    }

    fn module_declarations(&self, module: &CoreModule) -> Vec<FunctionDeclaration> {
        module
            .items
            .iter()
            .filter(|item| self.is_written(item) && Self::is_type_form(module, item))
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

    fn is_written(&self, item: &Item) -> bool {
        self.program
            .call_origins
            .introducing_macro(item.span)
            .is_none()
    }

    fn is_type_form(module: &CoreModule, item: &Item) -> bool {
        !item.hidden
            && !module
                .items
                .iter()
                .any(|other| matches!(other.kind, ItemKind::Def(..)) && other.span == item.span)
    }
}
