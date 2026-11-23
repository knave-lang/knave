#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParameterRole {
    FlowsToResult,
    FlowsThroughSharedArguments,
    HeldByResult,
    HigherRanked,
    Reassigned,
    StoredValue,
    StoredTarget,
}
