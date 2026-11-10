use super::unreachable_arm::UnreachableArm;
use crate::diagnostic::Diagnostics;
use crate::source::Span;
use crate::types::problems::TypeProblems;

pub(in crate::types) struct CoverageVerdict {
    unreachable_arms: Vec<UnreachableArm>,
    missing_cases: Option<Vec<String>>,
    exhaustive_without_guards: bool,
}

impl CoverageVerdict {
    pub(super) fn new(
        unreachable_arms: Vec<UnreachableArm>,
        missing_cases: Option<Vec<String>>,
    ) -> Self {
        Self {
            unreachable_arms,
            missing_cases,
            exhaustive_without_guards: false,
        }
    }

    pub(super) fn mark_exhaustive_without_guards(&mut self) {
        self.exhaustive_without_guards = true;
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
        if let Some(cases) = &self.missing_cases {
            diagnostics.push(TypeProblems::non_exhaustive_match(cases, span));
        }
    }

    pub(in crate::types) fn is_exhaustive_without_guards(&self) -> bool {
        self.exhaustive_without_guards
    }
}
