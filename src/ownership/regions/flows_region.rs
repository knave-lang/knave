use super::lambda_flows::LambdaFlows;
use crate::types::ResultRegion;

impl From<&LambdaFlows> for ResultRegion {
    fn from(flows: &LambdaFlows) -> Self {
        ResultRegion::new(flows.flows.clone(), flows.higher_ranked.clone())
    }
}
