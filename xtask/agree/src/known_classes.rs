use knave_expand::diagnostic::DiagnosticCode;

pub struct KnownClass {
    codes: &'static [&'static str],
}

pub struct KnownClasses {
    classes: &'static [KnownClass],
}

impl KnownClass {
    pub const ALIASING: Self = Self {
        codes: &["E0499", "E0502", "E0505", "E0506"],
    };

    fn knows(&self, code: &DiagnosticCode) -> bool {
        self.codes.contains(&code.as_str())
    }
}

impl KnownClasses {
    pub const BEFORE_THE_LOAN_RULE: Self = Self {
        classes: &[KnownClass::ALIASING],
    };

    pub fn know(&self, code: Option<&DiagnosticCode>) -> bool {
        code.is_some_and(|code| self.classes.iter().any(|class| class.knows(code)))
    }
}

#[cfg(test)]
mod tests {
    use super::KnownClasses;
    use crate::finding_fixture::FindingFixture;

    #[test]
    fn explains_a_finding_only_when_every_cited_code_is_known() {
        let classes = KnownClasses::BEFORE_THE_LOAN_RULE;
        assert!(FindingFixture::citing("seed-1", &["E0502", "E0499"]).is_explained_by(&classes));
        assert!(!FindingFixture::citing("seed-1", &["E0502", "E0716"]).is_explained_by(&classes));
        assert!(!classes.know(None));
    }
}
