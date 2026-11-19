use super::reaching_text::ReachingText;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug)]
pub enum Source {
    Slot(NodeId),
    Text(ReachingText),
}
