use super::derive_declaration::DeriveDeclaration;
use super::written_program::WrittenProgram;
use crate::core::{CoreModule, ItemKind};
use crate::expand::Program;
use crate::types::InferableDerives;

pub struct DeclaredDerives<'p> {
    program: &'p Program,
    written: WrittenProgram<'p>,
}

impl<'p> DeclaredDerives<'p> {
    pub fn new(program: &'p Program) -> Self {
        Self {
            program,
            written: WrittenProgram::new(program),
        }
    }

    pub fn written(&self) -> Vec<DeriveDeclaration> {
        self.program
            .project_modules()
            .flat_map(|module| self.declared_in(module))
            .collect()
    }

    pub fn inferred_among(
        written: Vec<DeriveDeclaration>,
        inferable: &InferableDerives,
    ) -> Vec<DeriveDeclaration> {
        written
            .into_iter()
            .filter(|derive| derive.listed.iter().all(|listed| inferable.gives(*listed)))
            .collect()
    }

    pub fn declared_in(&self, module: &CoreModule) -> Vec<DeriveDeclaration> {
        module
            .items
            .iter()
            .filter(|item| self.written.is_written(item))
            .filter_map(|item| match &item.kind {
                ItemKind::Derive(target, _, listed) => Some(DeriveDeclaration {
                    target: *target,
                    module: module.path.clone(),
                    span: item.span,
                    listed: listed.iter().map(|(listed, _)| *listed).collect(),
                }),
                _ => None,
            })
            .collect()
    }
}
