mod constructors;
mod guard;
mod matrix;
mod pattern;
mod query;
mod usefulness;
mod witness;

use rustc_hash::FxHashSet;

pub(super) use constructors::ConstructorCatalog;
use guard::PatternGuard;
use matrix::PatternMatrix;
use pattern::CoverPattern;
use query::Query;
use usefulness::Usefulness;
pub(super) use witness::WitnessText;

use super::problems::TypeProblems;
use crate::core::BindingArena;
use crate::diagnostic::Diagnostics;
use crate::source::Span;
use crate::types::{Type, TypedArm};

pub(super) struct MatchCoverage<'a> {
    bindings: &'a BindingArena,
    catalog: ConstructorCatalog<'a>,
    text: WitnessText<'a>,
}

pub(super) struct CoverageVerdict {
    unreachable_arms: Vec<UnreachableArm>,
    missing_cases: Option<Vec<String>>,
    exhaustive_without_guards: bool,
}

struct UnreachableArm {
    pattern: Span,
    covered_by: Option<Span>,
}

impl<'a> MatchCoverage<'a> {
    pub(super) fn new(
        bindings: &'a BindingArena,
        catalog: ConstructorCatalog<'a>,
        text: WitnessText<'a>,
    ) -> Self {
        Self {
            bindings,
            catalog,
            text,
        }
    }

    pub(super) fn judge(&self, scrutinee: &Type, arms: &[TypedArm]) -> CoverageVerdict {
        let scrutinee = self.catalog.peeled(scrutinee);
        let mut usefulness = Usefulness::new(&self.catalog);
        let mut matrix = PatternMatrix::default();
        let mut unreachable_arms = Vec::new();
        for (position, arm) in arms.iter().enumerate() {
            let pattern = CoverPattern::of(&arm.pattern, self.bindings);
            if usefulness.is_useful(&matrix, Self::single_query(&pattern, &scrutinee)) {
                matrix.push(pattern);
            } else {
                unreachable_arms.push(UnreachableArm {
                    pattern: arm.pattern.span,
                    covered_by: self.covering_arm(&arms[..position], &pattern, &scrutinee),
                });
            }
        }
        CoverageVerdict {
            unreachable_arms,
            missing_cases: self.missing_cases(&mut usefulness, &matrix, scrutinee.clone()),
            exhaustive_without_guards: self.is_exhaustive_without_guards(
                &mut usefulness,
                &scrutinee,
                arms,
            ),
        }
    }

    fn single_query<'q>(pattern: &'q CoverPattern, scrutinee: &'q Type) -> Query<'q> {
        Query::new(
            std::slice::from_ref(pattern),
            std::slice::from_ref(scrutinee),
        )
    }

    fn covering_arm(
        &self,
        previous: &[TypedArm],
        pattern: &CoverPattern,
        scrutinee: &Type,
    ) -> Option<Span> {
        let mut usefulness = Usefulness::new(&self.catalog);
        previous
            .iter()
            .find(|arm| {
                let earlier: PatternMatrix =
                    std::iter::once(CoverPattern::of(&arm.pattern, self.bindings)).collect();
                !usefulness.is_useful(&earlier, Self::single_query(pattern, scrutinee))
            })
            .map(|arm| arm.pattern.span)
    }

    fn missing_cases(
        &self,
        usefulness: &mut Usefulness,
        matrix: &PatternMatrix,
        scrutinee: Type,
    ) -> Option<Vec<String>> {
        if !usefulness.is_useful(matrix, Self::single_query(&CoverPattern::Wild, &scrutinee)) {
            return None;
        }
        let mut seen = FxHashSet::default();
        let cases = usefulness
            .witnesses(matrix, &[scrutinee])
            .iter()
            .map(|row| self.text.show(&row[0]))
            .filter(|case| seen.insert(case.clone()))
            .collect();
        Some(cases)
    }

    fn is_exhaustive_without_guards(
        &self,
        usefulness: &mut Usefulness,
        scrutinee: &Type,
        arms: &[TypedArm],
    ) -> bool {
        let unguarded: PatternMatrix = arms
            .iter()
            .filter(|arm| !PatternGuard::of(&arm.pattern).is_needed())
            .map(|arm| CoverPattern::of(&arm.pattern, self.bindings))
            .collect();
        !usefulness.is_useful(
            &unguarded,
            Self::single_query(&CoverPattern::Wild, scrutinee),
        )
    }
}

impl CoverageVerdict {
    pub(super) fn report(
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

    pub(super) fn is_exhaustive_without_guards(&self) -> bool {
        self.exhaustive_without_guards
    }
}
