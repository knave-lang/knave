use std::rc::Rc;

use crate::core::{CollectionForm, Expr, ExprKind, KeyedPath, MapOperation, Push};
use crate::diagnostic::Diagnostic;
use crate::expand::Expander;
use crate::expand::shape::Shape;
use crate::reader::Form;

pub(super) struct CollectionLowering<'e, 'x> {
    expander: &'e mut Expander<'x>,
}

impl<'e, 'x> CollectionLowering<'e, 'x> {
    pub(super) fn new(expander: &'e mut Expander<'x>) -> Self {
        Self { expander }
    }

    pub(super) fn lower(
        &mut self,
        form: CollectionForm,
        shape: &Shape,
    ) -> Result<ExprKind, Diagnostic> {
        match form {
            CollectionForm::Assoc => self.keyed(shape).map(ExprKind::Assoc),
            CollectionForm::MapAt => self.map_at(shape),
            CollectionForm::PushAt => self.push_at(shape),
            CollectionForm::SetAt => self.keyed(shape).map(ExprKind::SetAt),
        }
    }

    fn keyed(&mut self, shape: &Shape) -> Result<KeyedPath, Diagnostic> {
        let items = shape.exactly(4)?;
        let value = self.expander.lower_expr(&items[1])?;
        let keys = self.expressions(shape.list(&items[2])?)?;
        let item = self.expander.lower_expr(&items[3])?;
        Ok(KeyedPath {
            value: Box::new(value),
            keys,
            item: Box::new(item),
        })
    }

    fn map_at(&mut self, shape: &Shape) -> Result<ExprKind, Diagnostic> {
        let items = shape.at_least(3)?;
        let operation = items[1]
            .as_symbol()
            .and_then(|symbol| MapOperation::of(symbol.name()))
            .ok_or_else(|| shape.malformed(&items[1]))?;
        let value = self.expander.lower_expr(&items[2])?;
        let arguments = self.expressions(&items[3..])?;
        Ok(ExprKind::MapAt(operation, Box::new(value), arguments))
    }

    fn push_at(&mut self, shape: &Shape) -> Result<ExprKind, Diagnostic> {
        let items = shape.exactly(3)?;
        let written_by = self.macro_writing(&items[0]);
        let target = self.expander.lower_expr(&items[1])?;
        let item = self.expander.lower_expr(&items[2])?;
        Ok(ExprKind::PushAt(Push {
            target: Box::new(target),
            item: Box::new(item),
            written_by,
        }))
    }

    fn macro_writing(&self, head: &Form) -> Option<Rc<str>> {
        let calls = &self.expander.tables.macro_origins.calls;
        calls.last_expanded_at(head.span())
    }

    fn expressions(&mut self, forms: &[Form]) -> Result<Vec<Expr>, Diagnostic> {
        forms
            .iter()
            .map(|form| self.expander.lower_expr(form))
            .collect()
    }
}
