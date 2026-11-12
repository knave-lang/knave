use super::binding_symbols::BindingSymbols;
use super::read_back_table::ReadBackTable;
use super::row_index::RowIndex;
use super::spanned_forms::SpannedForms;
use crate::core::{BindingArena, BindingId};
use crate::reader::{Form, FormKind, Symbol};
use crate::source::Span;

pub struct ReadBackWriter<'w, 's> {
    arena: &'w BindingArena,
    symbols: &'w mut BindingSymbols<'s>,
    table: &'w mut ReadBackTable,
}

impl<'w, 's> ReadBackWriter<'w, 's> {
    pub fn new(
        arena: &'w BindingArena,
        symbols: &'w mut BindingSymbols<'s>,
        table: &'w mut ReadBackTable,
    ) -> Self {
        Self {
            arena,
            symbols,
            table,
        }
    }

    pub fn open(&mut self) -> RowIndex {
        self.table.open()
    }

    pub fn close(&mut self, row: RowIndex, parts: Vec<Form>, span: Span) -> Form {
        let form = SpannedForms::list(parts, span);
        self.table.close(row, &form);
        form
    }

    pub fn leaf(&mut self, form: Form) -> Form {
        self.table.leaf(&form);
        form
    }

    pub fn binder(&mut self, binding: BindingId, span: Span) -> Form {
        let form = self.symbol(binding, None, span);
        self.table.binder(&form, binding);
        form
    }

    pub fn reference(&mut self, binding: BindingId, written: Option<&str>, span: Span) -> Form {
        let form = self.symbol(binding, written, span);
        self.table.reference(&form, binding);
        form
    }

    fn symbol(&mut self, binding: BindingId, written: Option<&str>, span: Span) -> Form {
        let symbol = self.symbols.symbol(binding, self.arena);
        let symbol = match written {
            Some(name) => Symbol::with_scopes(name, symbol.scopes().clone()),
            None => symbol,
        };
        Form::new(FormKind::Symbol(symbol), span)
    }
}
