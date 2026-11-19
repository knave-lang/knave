use super::node_id::NodeId;
use super::reaching_text::ReachingText;

#[derive(Clone, Copy, Debug)]
pub enum Source {
    Slot(NodeId),
    Text(ReachingText),
}
