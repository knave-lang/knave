mod constructor;
mod constructor_catalog;
mod constructor_tag;
mod cover_pattern;
mod coverage_verdict;
mod exhaustiveness;
mod missing_case_search;
mod pattern_guard;
mod pattern_matrix;
mod query;
#[cfg(test)]
mod search_fixture;
mod unreachable_arm;
mod usefulness;
mod witness;
mod witness_text;

use rustc_hash::FxHashSet;

pub(super) use constructor_catalog::ConstructorCatalog;
use cover_pattern::CoverPattern;
use coverage_verdict::CoverageVerdict;
use exhaustiveness::Exhaustiveness;
use missing_case_search::MissingCaseSearch;
use pattern_guard::PatternGuard;
use pattern_matrix::PatternMatrix;
use query::Query;
use unreachable_arm::UnreachableArm;
use usefulness::Usefulness;
pub(super) use witness_text::WitnessText;

use crate::core::BindingArena;
use crate::source::Span;
use crate::types::{Type, TypedArm};

pub(super) struct MatchCoverage<'a> {
    bindings: &'a BindingArena,
    catalog: ConstructorCatalog<'a>,
    text: WitnessText<'a>,
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
        let usefulness = Usefulness::new(&self.catalog);
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
        let exhaustiveness = match self.missing_cases(&usefulness, &matrix, scrutinee.clone()) {
            Some(cases) => Exhaustiveness::Missing(cases),
            None if self.is_exhaustive_without_guards(&usefulness, &scrutinee, arms) => {
                Exhaustiveness::Complete
            }
            None => Exhaustiveness::OnlyWithGuards,
        };
        CoverageVerdict::new(unreachable_arms, exhaustiveness)
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
        let usefulness = Usefulness::new(&self.catalog);
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
        usefulness: &Usefulness,
        matrix: &PatternMatrix,
        scrutinee: Type,
    ) -> Option<Vec<String>> {
        let wild = CoverPattern::wild();
        if !usefulness.is_useful(matrix, Self::single_query(&wild, &scrutinee)) {
            return None;
        }
        let mut seen = FxHashSet::default();
        let cases = MissingCaseSearch::new(&self.catalog)
            .witnesses(matrix, &[scrutinee])
            .iter()
            .map(|row| self.text.show(&row[0]))
            .filter(|case| seen.insert(case.clone()))
            .collect();
        Some(cases)
    }

    fn is_exhaustive_without_guards(
        &self,
        usefulness: &Usefulness,
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
            Self::single_query(&CoverPattern::wild(), scrutinee),
        )
    }
}
