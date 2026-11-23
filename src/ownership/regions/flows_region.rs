use super::result_borrows::LambdaFlows;
use crate::types::ResultRegion;

impl From<&LambdaFlows> for ResultRegion {
    fn from(flows: &LambdaFlows) -> Self {
        ResultRegion::new(flows.flows.clone(), flows.higher_ranked.clone())
    }
}
