#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParameterRole {
    FlowsToResult,
    FlowsThroughSharedArguments,
    HeldByResult,
    LeftOutOfResult,
    HigherRanked,
    Reassigned,
    StoredValue,
    StoredTarget,
}
