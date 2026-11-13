use std::rc::Rc;

use super::trait_rows_reach::{LoweredRows, TraitRowsReach};
use crate::core::RustTraitDeclaration;

pub struct RustTrait {
    pub path: Rc<str>,
    pub help: Option<Rc<str>>,
    pub associated: Option<Rc<str>>,
    rows: TraitRowsReach,
}

impl RustTrait {
    pub fn unreached(declaration: &RustTraitDeclaration) -> Self {
        Self {
            path: Rc::clone(&declaration.path),
            help: declaration.help.clone(),
            associated: None,
            rows: TraitRowsReach::unreached(),
        }
    }

    pub fn lowered(declaration: &RustTraitDeclaration, rows: LoweredRows) -> Self {
        Self {
            rows: TraitRowsReach::lowered(rows),
            ..Self::unreached(declaration)
        }
    }

    pub fn written(path: Rc<str>, associated: Option<Rc<str>>, rows: LoweredRows) -> Self {
        Self {
            path,
            help: None,
            associated,
            rows: TraitRowsReach::lowered(rows),
        }
    }

    pub fn rows(&self) -> &TraitRowsReach {
        &self.rows
    }

    pub fn rows_mut(&mut self) -> &mut TraitRowsReach {
        &mut self.rows
    }
}
