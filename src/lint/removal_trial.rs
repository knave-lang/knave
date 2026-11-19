use crate::core::BindingId;

pub enum TrialVerdict<Kept> {
    Kept(Kept),
    Rejected(Vec<BindingId>),
}

pub trait RemovalTrial {
    type Kept;

    fn verdict(&mut self, removed: &[BindingId]) -> TrialVerdict<Self::Kept>;
}

impl TrialVerdict<()> {
    pub fn keeping<Kept>(self, kept: Kept) -> TrialVerdict<Kept> {
        match self {
            Self::Kept(()) => TrialVerdict::Kept(kept),
            Self::Rejected(culprits) => TrialVerdict::Rejected(culprits),
        }
    }
}
