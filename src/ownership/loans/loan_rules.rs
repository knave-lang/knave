use super::call_site::CallSite;
use super::holdings::Holdings;
use crate::types::{CopyRules, TemplateBorrows, TypedExpr, TypedProgram};

pub struct LoanRules<'p> {
    templates: &'p TemplateBorrows,
    copy: CopyRules<'p>,
}

impl<'p> LoanRules<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        Self {
            templates: &program.template_borrows,
            copy: CopyRules::new(&program.declarations, &program.generics),
        }
    }

    pub fn moves(&self, value: &TypedExpr) -> bool {
        !self.copy.is_copy(&value.ty)
    }

    pub fn result_of(&self, site: &CallSite<'_>, passed: Vec<Holdings>) -> Holdings {
        site.result_holdings(passed, self.templates)
    }
}
