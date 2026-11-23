#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParameterRole {
    FlowsToResult,
    FlowsThroughSharedArguments,
    HigherRanked,
    Reassigned,
    StoredValue,
    StoredTarget,
}
