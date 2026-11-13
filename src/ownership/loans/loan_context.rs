use super::call_site::CallSite;
use super::holdings::Holdings;
use crate::types::{CopyRules, TemplateBorrows, TypedExpr, TypedProgram};

pub struct LoanContext<'p> {
    templates: &'p TemplateBorrows,
    copy: CopyRules<'p>,
}

impl<'p> LoanContext<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        Self {
            templates: &program.template_borrows,
            copy: CopyRules::new(&program.declarations, &program.generics),
        }
    }

    pub fn moves(&self, value: &TypedExpr) -> bool {
        !self.copy.is_copy(&value.ty)
    }

    pub fn result_holdings(&self, site: &CallSite<'_>, passed: Vec<Holdings>) -> Holdings {
        if !site.returns_a_borrow() || !self.is_template(site.callee()) {
            return Holdings::default();
        }
        let mut lent = passed
            .into_iter()
            .enumerate()
            .filter(|(position, _)| {
                site.parameter(*position)
                    .is_none_or(|parameter| parameter.holds_reference())
            })
            .map(|(_, held)| held);
        match (lent.next(), lent.next()) {
            (Some(only), None) => only,
            _ => Holdings::default(),
        }
    }

    fn is_template(&self, callee: &TypedExpr) -> bool {
        self.templates.read_template(callee, false, |_| true)
    }
}
