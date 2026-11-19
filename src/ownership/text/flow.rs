use super::node_id::NodeId;
use super::source::Source;
use super::transfer::Transfer;
use crate::source::Span;

#[derive(Clone, Copy, Debug)]
pub struct Flow {
    pub source: Source,
    pub target: NodeId,
    pub transfer: Transfer,
    pub span: Span,
}
