use super::Expander;
use super::problems::ExpandProblems;
use super::shape::Shape;
use crate::core::{BindingKind, KernelType, Namespace, TypeBinder};
use crate::diagnostic::Diagnostic;
use crate::reader::Form;
use crate::scope::ScopeKind;
use crate::source::Span;

pub(super) struct Parameterized {
    pub binders: Vec<TypeBinder>,
    pub body: Vec<Form>,
}

impl Expander<'_> {
    const PARAMETERIZED_PARTS: usize = 3;

    pub(super) fn misplaced_where(&self, body: &[Form]) -> Option<Span> {
        let items = body.first()?.as_list()?;
        let opens_with_where = items.first().is_some_and(|head| self.is_where(head));
        (items.len() >= Self::PARAMETERIZED_PARTS && opens_with_where).then(|| items[0].span())
    }

    pub(super) fn head_name<'f>(
        &self,
        shape: &Shape,
        head: &'f Form,
    ) -> Result<&'f Form, Diagnostic> {
        Ok(Self::head_parts(shape, head)?.0)
    }

    pub(super) fn lower_head(
        &mut self,
        shape: &Shape,
        head: &Form,
        body: Vec<Form>,
    ) -> Result<Parameterized, Diagnostic> {
        let (_, binders) = Self::head_parts(shape, head)?;
        self.lower_parameterized(binders, body)
    }

    pub(super) fn lower_parameters(
        &mut self,
        where_items: &[Form],
        malformed: impl Fn() -> Diagnostic,
    ) -> Result<Parameterized, Diagnostic> {
        let binders = where_items[1].as_list().ok_or_else(&malformed)?.to_vec();
        let body = where_items[2..].to_vec();
        self.lower_parameterized(binders, body)
    }

    fn lower_parameterized(
        &mut self,
        mut binders: Vec<Form>,
        mut body: Vec<Form>,
    ) -> Result<Parameterized, Diagnostic> {
        if let Some(binder) = binders.iter().find(|binder| {
            binder.as_list().is_some_and(|parts| {
                parts.len() > 1
                    && parts
                        .first()
                        .and_then(Form::as_symbol)
                        .is_some_and(|symbol| symbol.name().starts_with('\''))
            })
        }) {
            return Err(ExpandProblems::bounded_lifetime(binder.span()));
        }
        let scope = self.tables.scopes.fresh(ScopeKind::Local);
        binders
            .iter_mut()
            .chain(body.iter_mut())
            .for_each(|form| form.add_scope(scope));
        let variables = binders
            .iter()
            .map(|binder| self.define_binder_variable(binder))
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        let binders = binders
            .iter()
            .zip(variables)
            .map(|(binder, variable)| self.lower_type_binder(binder, variable))
            .collect::<Result<Vec<TypeBinder>, Diagnostic>>()?;
        Ok(Parameterized { binders, body })
    }

    fn head_parts<'f>(shape: &Shape, head: &'f Form) -> Result<(&'f Form, Vec<Form>), Diagnostic> {
        match head.as_list() {
            Some([name, parameters @ ..]) => {
                shape.symbol(name)?;
                Ok((name, parameters.to_vec()))
            }
            Some([]) => Err(shape.malformed(head)),
            None => Ok((head, Vec::new())),
        }
    }

    pub(super) fn is_where(&self, head: &Form) -> bool {
        head.as_symbol()
            .and_then(|symbol| self.bindings().resolve(symbol, Namespace::Type))
            .is_some_and(|binding| self.kind(binding) == BindingKind::KernelType(KernelType::Where))
    }
}
