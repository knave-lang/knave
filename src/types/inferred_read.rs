use crate::source::Span;

#[derive(Clone, Copy, Debug)]
pub struct InferredRead {
    read: Span,
    stage: ReadStage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReadStage {
    Open,
    Generalized,
    Settled,
}

impl InferredRead {
    pub fn open(read: Span) -> Self {
        Self {
            read,
            stage: ReadStage::Open,
        }
    }

    pub fn settled(read: Span) -> Self {
        Self {
            read,
            stage: ReadStage::Settled,
        }
    }

    pub fn generalized(self) -> Self {
        match self.stage {
            ReadStage::Open => Self {
                stage: ReadStage::Generalized,
                ..self
            },
            ReadStage::Generalized | ReadStage::Settled => self,
        }
    }

    pub fn read(&self) -> Span {
        self.read
    }

    pub fn waits_to_settle(&self) -> bool {
        self.stage == ReadStage::Generalized
    }
}
