use super::flow::Flow;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RefusalCause {
    Rule,
    StatedLimit,
}

#[derive(Clone, Copy, Debug)]
pub struct Refusal<'g> {
    pub flow: &'g Flow,
    pub cause: RefusalCause,
}
