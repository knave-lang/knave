use std::rc::Rc;

use super::ty::{Bound, Scheme, Type};
use crate::core::{BindingId, RustTraitDeclaration};

pub struct Implementor {
    pub scheme: Scheme,
    pub argument: Option<Type>,
}

pub struct RustTrait {
    pub path: Rc<str>,
    pub help: Option<Rc<str>>,
    pub associated: Option<Rc<str>>,
    rows: TraitRowsLowering,
}

enum TraitRowsLowering {
    Unreached,
    Lowered(LoweredRows),
}

impl RustTrait {
    const LEFT_OUT: &'static str = "the trait reach left out a trait the program uses";

    pub fn unreached(declaration: &RustTraitDeclaration) -> Self {
        Self {
            path: Rc::clone(&declaration.path),
            help: declaration.help.clone(),
            associated: None,
            rows: TraitRowsLowering::Unreached,
        }
    }

    pub fn lowered(declaration: &RustTraitDeclaration, rows: LoweredRows) -> Self {
        Self {
            rows: TraitRowsLowering::Lowered(rows),
            ..Self::unreached(declaration)
        }
    }

    pub fn written(path: Rc<str>, associated: Option<Rc<str>>, rows: LoweredRows) -> Self {
        Self {
            path,
            help: None,
            associated,
            rows: TraitRowsLowering::Lowered(rows),
        }
    }

    pub fn rows(&self) -> &LoweredRows {
        self.lowered_rows().expect(Self::LEFT_OUT)
    }

    pub fn lowered_rows(&self) -> Option<&LoweredRows> {
        match &self.rows {
            TraitRowsLowering::Unreached => None,
            TraitRowsLowering::Lowered(rows) => Some(rows),
        }
    }

    pub fn rows_mut(&mut self) -> &mut LoweredRows {
        match &mut self.rows {
            TraitRowsLowering::Unreached => panic!("{}", Self::LEFT_OUT),
            TraitRowsLowering::Lowered(rows) => rows,
        }
    }
}

pub struct LoweredRows {
    implementors: Vec<Implementor>,
    supertraits: Vec<Bound>,
}

impl LoweredRows {
    pub fn new(implementors: Vec<Implementor>, supertraits: Vec<Bound>) -> Self {
        Self {
            implementors,
            supertraits,
        }
    }

    pub fn implementors(&self) -> &[Implementor] {
        &self.implementors
    }

    pub fn add(&mut self, implementor: Implementor) {
        self.implementors.push(implementor);
    }

    pub fn remove_struct(&mut self, struct_binding: BindingId) {
        self.implementors.retain(|implementor| {
            !matches!(implementor.scheme.body, Type::Named(named, _) if named == struct_binding)
        });
    }

    pub fn set_scheme(&mut self, body: &Type, scheme: Scheme) {
        if let Some(implementor) = self
            .implementors
            .iter_mut()
            .find(|implementor| implementor.scheme.body == *body)
        {
            implementor.scheme = scheme;
        }
    }

    pub fn supertraits(&self) -> &[Bound] {
        &self.supertraits
    }

    pub fn is_open(&self) -> bool {
        self.implementors
            .iter()
            .any(|implementor| !implementor.scheme.params.is_empty())
    }
}
