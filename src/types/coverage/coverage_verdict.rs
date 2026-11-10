use super::exhaustiveness::Exhaustiveness;
use super::unreachable_arm::UnreachableArm;
use crate::diagnostic::Diagnostics;
use crate::source::Span;
use crate::types::problems::TypeProblems;

pub(in crate::types) struct CoverageVerdict {
    unreachable_arms: Vec<UnreachableArm>,
    exhaustiveness: Exhaustiveness,
}

impl CoverageVerdict {
    pub(super) fn new(
        unreachable_arms: Vec<UnreachableArm>,
        exhaustiveness: Exhaustiveness,
    ) -> Self {
        Self {
            unreachable_arms,
            exhaustiveness,
        }
    }

    pub(in crate::types) fn report(
        &self,
        span: Span,
        diagnostics: &mut Diagnostics,
        warnings: &mut Diagnostics,
    ) {
        for arm in &self.unreachable_arms {
            warnings.push(TypeProblems::unreachable_arm(arm.pattern, arm.covered_by));
        }
        self.exhaustiveness.report(span, diagnostics);
    }

    pub(in crate::types) fn is_exhaustive_without_guards(&self) -> bool {
        self.exhaustiveness.is_complete_without_guards()
    }
}

#[cfg(test)]
mod tests {
    use super::CoverageVerdict;
    use crate::diagnostic::Diagnostics;
    use crate::source::{SourceId, Span};
    use crate::types::coverage::exhaustiveness::Exhaustiveness;
    use crate::types::coverage::unreachable_arm::UnreachableArm;

    struct VerdictFixture {
        diagnostics: Diagnostics,
        warnings: Diagnostics,
    }

    impl VerdictFixture {
        fn new() -> Self {
            Self {
                diagnostics: Diagnostics::default(),
                warnings: Diagnostics::default(),
            }
        }

        fn span(start: usize) -> Span {
            Span::new(SourceId::new(0), start..start + 1)
        }

        fn unreachable_arm() -> UnreachableArm {
            UnreachableArm {
                pattern: Self::span(4),
                covered_by: Some(Self::span(1)),
            }
        }

        fn report(&mut self, verdict: &CoverageVerdict) {
            verdict.report(Self::span(0), &mut self.diagnostics, &mut self.warnings);
        }
    }

    #[test]
    fn each_unreachable_arm_reports_a_warning() {
        let mut fixture = VerdictFixture::new();
        let arms = vec![
            VerdictFixture::unreachable_arm(),
            VerdictFixture::unreachable_arm(),
        ];
        fixture.report(&CoverageVerdict::new(arms, Exhaustiveness::Complete));
        assert_eq!(fixture.warnings.len(), 2);
        assert!(fixture.diagnostics.is_empty());
    }

    #[test]
    fn missing_cases_report_one_error() {
        let mut fixture = VerdictFixture::new();
        let missing = Exhaustiveness::Missing(vec!["dot".to_string(), "(circle _)".to_string()]);
        fixture.report(&CoverageVerdict::new(Vec::new(), missing));
        assert_eq!(fixture.diagnostics.len(), 1);
        assert!(fixture.diagnostics.has_errors());
        assert!(fixture.warnings.is_empty());
    }

    #[test]
    fn a_match_exhaustive_only_with_guards_reports_nothing() {
        let mut fixture = VerdictFixture::new();
        fixture.report(&CoverageVerdict::new(
            Vec::new(),
            Exhaustiveness::OnlyWithGuards,
        ));
        assert!(fixture.diagnostics.is_empty());
        assert!(fixture.warnings.is_empty());
    }

    #[test]
    fn only_a_complete_match_is_exhaustive_without_guards() {
        let verdict = |exhaustiveness| CoverageVerdict::new(Vec::new(), exhaustiveness);
        assert!(verdict(Exhaustiveness::Complete).is_exhaustive_without_guards());
        assert!(!verdict(Exhaustiveness::OnlyWithGuards).is_exhaustive_without_guards());
        assert!(
            !verdict(Exhaustiveness::Missing(vec!["dot".to_string()]))
                .is_exhaustive_without_guards()
        );
    }
}
