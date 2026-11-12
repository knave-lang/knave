use crate::finding::Finding;

pub struct KnownClasses {
    codes: &'static [&'static str],
}

impl KnownClasses {
    pub const BEFORE_THE_LOAN_RULE: Self = Self {
        codes: &["E0382", "E0499", "E0502", "E0505", "E0506"],
    };

    pub fn explain(&self, finding: &Finding) -> bool {
        finding
            .evidence
            .iter()
            .all(|rejection| self.codes.contains(&rejection.code_text()))
    }
}

#[cfg(test)]
mod tests {
    use super::KnownClasses;
    use crate::finding_fixture::FindingFixture;

    #[test]
    fn explains_a_finding_only_when_every_cited_code_is_known() {
        let classes = KnownClasses::BEFORE_THE_LOAN_RULE;
        assert!(classes.explain(&FindingFixture::citing("seed-1", &["E0502", "E0499"])));
        assert!(!classes.explain(&FindingFixture::citing("seed-1", &["E0502", "E0716"])));
    }
}
