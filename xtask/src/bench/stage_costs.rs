use std::collections::BTreeMap;
use std::fmt;

use super::allocator_share::AllocatorShare;
use crate::failure::TaskFailure;

#[derive(Default)]
pub struct StageCosts {
    stages: BTreeMap<String, StageCost>,
}

#[derive(Default)]
struct StageCost {
    inclusive: Option<u64>,
    allocator: Option<u64>,
}

#[derive(Debug)]
pub enum StageCostProblem {
    ShareWithoutStage(String),
    ShareAboveStage {
        stage: String,
        share: u64,
        inclusive: u64,
    },
}

impl StageCosts {
    pub fn add_inclusive(&mut self, stage: String, cost: u64) {
        let inclusive = &mut self.stages.entry(stage).or_default().inclusive;
        *inclusive.get_or_insert(0) += cost;
    }

    pub fn add_allocator(&mut self, stage: String, cost: u64) {
        let allocator = &mut self.stages.entry(stage).or_default().allocator;
        *allocator.get_or_insert(0) += cost;
    }

    pub fn into_rows(self) -> Result<BTreeMap<String, u64>, StageCostProblem> {
        let mut rows = BTreeMap::new();
        for (stage, cost) in self.stages {
            rows.extend(cost.rows(stage)?);
        }
        Ok(rows)
    }
}

impl StageCost {
    fn rows(self, stage: String) -> Result<Vec<(String, u64)>, StageCostProblem> {
        let Some(inclusive) = self.inclusive else {
            return Err(StageCostProblem::ShareWithoutStage(stage));
        };
        let Some(allocator) = self.allocator else {
            return Ok(vec![(stage, inclusive)]);
        };
        let Some(own) = inclusive.checked_sub(allocator) else {
            return Err(StageCostProblem::ShareAboveStage {
                stage,
                share: allocator,
                inclusive,
            });
        };
        Ok(vec![
            (AllocatorShare::row_of(&stage), allocator),
            (stage, own),
        ])
    }
}

impl fmt::Display for StageCostProblem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ShareWithoutStage(stage) => write!(
                formatter,
                "the profile gives a `{stage}` allocator share but no call into the `{stage}` stage"
            ),
            Self::ShareAboveStage {
                stage,
                share,
                inclusive,
            } => write!(
                formatter,
                "the `{stage}` allocator share of {share} instructions exceeds the stage's {inclusive}"
            ),
        }
    }
}

impl From<StageCostProblem> for TaskFailure {
    fn from(problem: StageCostProblem) -> Self {
        Self::new(problem.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::StageCosts;

    struct CostsFixture;

    impl CostsFixture {
        fn costs(inclusive: &[(&str, u64)], allocator: &[(&str, u64)]) -> StageCosts {
            let mut costs = StageCosts::default();
            for (stage, cost) in inclusive {
                costs.add_inclusive(stage.to_string(), *cost);
            }
            for (stage, cost) in allocator {
                costs.add_allocator(stage.to_string(), *cost);
            }
            costs
        }

        fn failure(costs: StageCosts) -> String {
            costs
                .into_rows()
                .err()
                .map(|problem| problem.to_string())
                .unwrap_or_default()
        }

        fn rows(costs: StageCosts) -> Vec<(String, u64)> {
            costs.into_rows().unwrap().into_iter().collect()
        }
    }

    #[test]
    fn moves_a_stage_s_allocator_share_to_its_own_row() {
        let costs = CostsFixture::costs(&[("build", 30), ("build", 5)], &[("build", 8)]);
        assert_eq!(
            CostsFixture::rows(costs),
            [
                ("build".to_string(), 27),
                ("build allocator".to_string(), 8)
            ]
        );
    }

    #[test]
    fn gives_no_allocator_row_to_a_stage_without_a_share() {
        let costs = CostsFixture::costs(&[("check", 30)], &[]);
        assert_eq!(CostsFixture::rows(costs), [("check".to_string(), 30)]);
    }

    #[test]
    fn rejects_a_share_larger_than_its_stage() {
        let costs = CostsFixture::costs(&[("build", 3)], &[("build", 8)]);
        let failure = CostsFixture::failure(costs);
        assert!(
            failure.contains("`build` allocator share of 8 instructions exceeds"),
            "{failure}"
        );
    }

    #[test]
    fn rejects_a_share_with_no_stage() {
        let costs = CostsFixture::costs(&[], &[("emit", 8)]);
        let failure = CostsFixture::failure(costs);
        assert!(
            failure.contains("`emit` allocator share but no call into the `emit` stage"),
            "{failure}"
        );
    }
}
