use super::node_id::NodeId;
use super::reaching_text::ReachingText;
use super::representation::Representation;
use super::slot::Slot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassRole {
    NeedsOwned,
    KeepsView,
    Stored,
    Parameter { tied: Option<NodeId> },
    Local,
    Held,
}

impl ClassRole {
    pub fn unseen(slot: &Slot) -> Self {
        if slot.is_stored_place() {
            Self::Stored
        } else {
            Self::Held
        }
    }

    pub fn decide(
        self,
        reaching: ReachingText,
        reaching_at: impl Fn(NodeId) -> ReachingText,
    ) -> Representation {
        match self {
            Self::NeedsOwned => Representation::Owned,
            Self::KeepsView => Representation::View,
            Self::Stored => Representation::of_stored(reaching),
            Self::Parameter { tied } => tied.map_or(Representation::View, |variable| {
                Representation::of_tied_parameter(reaching_at(variable))
            }),
            Self::Local => Representation::of_local(reaching),
            Self::Held => Representation::of_held(reaching),
        }
    }
}
