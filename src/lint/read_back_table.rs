use rustc_hash::FxHashMap;

use super::row_index::RowIndex;
use super::spanned_forms::SpannedForms;
use crate::core::BindingId;
use crate::interpret::{ListNesting, NestedTooDeep, Value};
use crate::reader::{Form, FormKind, Number, NumberValue};
use crate::source::Span;

#[derive(Default)]
pub struct ReadBackTable {
    rows: Vec<TableRow>,
    open_rows: Vec<RowIndex>,
}

struct TableRow {
    form: Option<Form>,
    parent: Option<RowIndex>,
    depth: usize,
    role: SymbolRole,
    binding: Option<BindingId>,
}

#[derive(Clone, Copy, PartialEq)]
enum SymbolRole {
    Binds,
    Reads,
    Neither,
}

impl ReadBackTable {
    const NO_PARENT: i128 = -1;

    pub fn open(&mut self) -> RowIndex {
        let row = self.push(None, SymbolRole::Neither, None);
        self.open_rows.push(row);
        row
    }

    pub fn close(&mut self, row: RowIndex, form: &Form) {
        let closed = self.open_rows.pop();
        debug_assert_eq!(
            closed,
            Some(row),
            "rows close in the reverse order they open"
        );
        self.rows[row.position()].form = Some(form.clone());
    }

    pub fn leaf(&mut self, form: &Form) {
        self.push(Some(form.clone()), SymbolRole::Neither, None);
    }

    pub fn binder(&mut self, form: &Form, binding: BindingId) {
        self.push(Some(form.clone()), SymbolRole::Binds, Some(binding));
    }

    pub fn reference(&mut self, form: &Form, binding: BindingId) {
        self.push(Some(form.clone()), SymbolRole::Reads, Some(binding));
    }

    pub fn into_value(self, span: Span) -> Result<Value, NestedTooDeep> {
        let reads = self.read_counts();
        let entries = self
            .rows
            .iter()
            .map(|row| Self::entry(row, &reads))
            .collect();
        ListNesting::form_list(entries, span)
    }

    fn push(
        &mut self,
        form: Option<Form>,
        role: SymbolRole,
        binding: Option<BindingId>,
    ) -> RowIndex {
        let parent = self.open_rows.last().copied();
        let depth = parent.map_or(0, |parent| self.rows[parent.position()].depth + 1);
        let row = RowIndex::new(self.rows.len());
        self.rows.push(TableRow {
            form,
            parent,
            depth,
            role,
            binding,
        });
        row
    }

    fn read_counts(&self) -> FxHashMap<BindingId, usize> {
        let mut reads = FxHashMap::default();
        let read_bindings = self
            .rows
            .iter()
            .filter(|row| row.role == SymbolRole::Reads)
            .filter_map(|row| row.binding);
        for binding in read_bindings {
            *reads.entry(binding).or_insert(0) += 1;
        }
        reads
    }

    fn entry(row: &TableRow, reads: &FxHashMap<BindingId, usize>) -> Form {
        let form = row.form.clone().expect("every opened row is closed");
        let span = form.span();
        let read_count = match (row.role, row.binding) {
            (SymbolRole::Binds, Some(binding)) => reads.get(&binding).copied().unwrap_or(0),
            _ => 0,
        };
        let parent = row
            .parent
            .map_or(Self::NO_PARENT, |parent| parent.position() as i128);
        let parts = vec![
            form,
            Self::number(parent, span),
            Self::number(row.depth as i128, span),
            row.role.symbol(span),
            Self::number(read_count as i128, span),
        ];
        SpannedForms::list(parts, span)
    }

    fn number(value: i128, span: Span) -> Form {
        let number = Number::new(value.to_string(), NumberValue::Integer(value));
        Form::new(FormKind::Number(number), span)
    }
}

impl SymbolRole {
    fn symbol(self, span: Span) -> Form {
        let spelled = match self {
            Self::Binds => "binds",
            Self::Reads => "reads",
            Self::Neither => "none",
        };
        SpannedForms::symbol(spelled, span)
    }
}

#[cfg(test)]
mod tests {
    use super::ReadBackTable;
    use crate::lint::expanded_source::ExpandedSource;
    use crate::reader::{Form, FormKind, Symbol};
    use crate::source::{SourceId, Span};

    struct TableFixture;

    impl TableFixture {
        const ROLE_FROM_END: usize = 1;
        const READS_FROM_END: usize = 0;

        fn rows(source: &str) -> Vec<String> {
            ExpandedSource::expand(source).table_rows("area")
        }

        fn row_of<'r>(rows: &'r [String], form: &str) -> Vec<&'r str> {
            Self::column_of(rows, form, Self::ROLE_FROM_END)
        }

        fn reads_of<'r>(rows: &'r [String], form: &str) -> Vec<&'r str> {
            Self::column_of(rows, form, Self::READS_FROM_END)
        }

        fn column_of<'r>(rows: &'r [String], form: &str, from_end: usize) -> Vec<&'r str> {
            rows.iter()
                .filter(|row| row.starts_with(&format!("{form} ")))
                .map(|row| {
                    row.rsplit(' ')
                        .nth(from_end)
                        .expect("a row has every column")
                })
                .collect()
        }
    }

    #[test]
    #[should_panic(expected = "rows close in the reverse order they open")]
    fn closing_an_outer_row_first_is_refused() {
        let span = Span::new(SourceId::new(0), 0..0);
        let form = Form::new(FormKind::Symbol(Symbol::new("row")), span);
        let mut table = ReadBackTable::default();
        let outer = table.open();
        table.open();
        table.close(outer, &form);
    }

    #[test]
    fn a_local_binder_binds_and_its_use_reads() {
        let rows = TableFixture::rows("(defn area (width) (let (unused 1) width))");
        assert_eq!(TableFixture::row_of(&rows, "unused"), ["binds"]);
        assert_eq!(TableFixture::row_of(&rows, "width"), ["binds", "reads"]);
    }

    #[test]
    fn a_binder_counts_the_reads_of_its_binding() {
        let rows = TableFixture::rows(
            "(defn area (width) (let (twice 1 unused 2) (+ twice twice width)))",
        );
        assert_eq!(TableFixture::reads_of(&rows, "twice"), ["2", "0", "0"]);
        assert_eq!(TableFixture::reads_of(&rows, "unused"), ["0"]);
    }

    #[test]
    fn a_kernel_head_is_neither() {
        let rows = TableFixture::rows("(defn area (width) width)");
        assert_eq!(TableFixture::row_of(&rows, "fn"), ["none"]);
    }

    #[test]
    fn a_child_points_at_its_parent_one_level_deeper() {
        let rows = TableFixture::rows("(defn area (width) width)");
        assert_eq!(rows[0].split(' ').rev().nth(3), Some("-1"));
        assert!(rows.iter().skip(1).all(|row| !row.contains(" -1 ")));
    }

    #[test]
    fn a_pattern_variable_binds_and_a_short_variant_reads_its_variant() {
        let rows = TableFixture::rows(
            "(defenum shape (circle i64) dot)\n(defn area (form) (match form ((circle radius) radius) (dot 0)))",
        );
        assert_eq!(TableFixture::row_of(&rows, "radius"), ["binds", "reads"]);
        assert_eq!(TableFixture::row_of(&rows, "circle"), ["reads"]);
    }

    #[test]
    fn a_quoted_datum_adds_no_rows() {
        let rows = TableFixture::rows(
            "(use code value-eq)\n(defn area (name) (value-eq name (quote total)))",
        );
        assert!(TableFixture::row_of(&rows, "total").is_empty());
    }
}
