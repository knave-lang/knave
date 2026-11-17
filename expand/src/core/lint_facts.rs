use super::{ImportedBindings, ModuleId};
use crate::source::{SourceMap, Span};

#[derive(Clone, Debug, Default)]
pub struct LintFacts(Option<Box<RecordedFacts>>);

#[derive(Clone, Debug, Default)]
struct RecordedFacts {
    uses: Uses,
}

#[derive(Clone, Debug)]
enum Uses {
    Written(Vec<WrittenUse>),
    Read(Vec<Import>),
}

impl Default for Uses {
    fn default() -> Self {
        Self::Written(Vec::new())
    }
}

impl LintFacts {
    pub fn record_use(&mut self, written: WrittenUse) {
        let Uses::Written(uses) = &mut self.recorded().uses else {
            panic!("a module records its uses before the read-out reads them");
        };
        uses.push(written);
    }

    pub fn written_uses(&self) -> &[WrittenUse] {
        match self.0.as_deref().map(|facts| &facts.uses) {
            Some(Uses::Written(uses)) => uses,
            _ => &[],
        }
    }

    pub fn read_uses(&mut self, read: impl FnMut(WrittenUse) -> Import) {
        let Some(facts) = self.0.as_deref_mut() else {
            return;
        };
        let Uses::Written(written) = &mut facts.uses else {
            panic!("the read-out reads a module's uses once");
        };
        let imports = std::mem::take(written).into_iter().map(read).collect();
        facts.uses = Uses::Read(imports);
    }

    pub fn imports(&self) -> &[Import] {
        match self.0.as_deref().map(|facts| &facts.uses) {
            Some(Uses::Read(imports)) => imports,
            _ => &[],
        }
    }

    fn recorded(&mut self) -> &mut RecordedFacts {
        self.0.get_or_insert_with(Box::default)
    }
}

#[derive(Clone, Debug)]
pub struct WrittenUse {
    pub span: Span,
    pub module: ModuleId,
    pub names: Box<[WrittenName]>,
}

#[derive(Clone, Debug)]
pub struct WrittenName {
    pub span: Span,
    pub bindings: ImportedBindings,
}

impl WrittenName {
    pub fn text<'s>(&self, sources: &'s SourceMap) -> &'s str {
        &sources.file(self.span.source()).text()[self.span.range()]
    }
}

#[derive(Clone, Debug)]
pub struct Import {
    pub span: Span,
    pub names: Box<[ImportedName]>,
    pub module_read: bool,
}

#[derive(Clone, Debug)]
pub struct ImportedName {
    pub written: WrittenName,
    pub used: bool,
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
