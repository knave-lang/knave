use super::loan_facts::LoanFacts;
use crate::ownership::OwnershipChecker;
use crate::source::SourceMap;
use crate::types::{TypedProgram, TypedSource};

pub struct LoanSource {
    program: TypedProgram,
    sources: SourceMap,
}

impl LoanSource {
    const FILE: &str = "fixture.knv";

    pub fn new(text: &str) -> Self {
        let mut program = TypedSource::expand(text).typed();
        OwnershipChecker::waived(&mut program);
        let mut sources = SourceMap::default();
        sources.add(Self::FILE, text);
        Self { program, sources }
    }

    pub fn report(&self) -> String {
        LoanFacts::of(&self.program).report(&self.sources)
    }

    pub fn line_of(&self, access: &str) -> String {
        self.report()
            .lines()
            .find(|line| line.contains(access))
            .unwrap_or_else(|| panic!("the report has a line with `{access}`:\n{}", self.report()))
            .trim()
            .to_string()
    }
}
