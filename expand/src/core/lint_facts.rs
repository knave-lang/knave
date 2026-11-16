use super::ImportedBindings;
use crate::source::Span;

#[derive(Clone, Debug, Default)]
pub struct LintFacts(Option<Box<RecordedFacts>>);

#[derive(Clone, Debug, Default)]
struct RecordedFacts {
    imports: Vec<Import>,
}

impl LintFacts {
    pub fn record_import(&mut self, import: Import) {
        self.recorded().imports.push(import);
    }

    pub fn imports(&self) -> &[Import] {
        self.0.as_deref().map_or(&[], |facts| &facts.imports)
    }

    fn recorded(&mut self) -> &mut RecordedFacts {
        self.0.get_or_insert_with(Box::default)
    }
}

#[derive(Clone, Debug)]
pub struct Import {
    pub span: Span,
    pub names: Box<[ImportedName]>,
}

#[derive(Clone, Debug)]
pub struct ImportedName {
    pub span: Span,
    pub bindings: ImportedBindings,
}

#[cfg(test)]
mod tests {
    use super::{LintFacts, RecordedFacts};

    #[test]
    fn a_module_that_records_nothing_holds_one_pointer_of_lint_facts() {
        assert_eq!(
            std::mem::size_of::<LintFacts>(),
            std::mem::size_of::<Box<RecordedFacts>>()
        );
    }
}
