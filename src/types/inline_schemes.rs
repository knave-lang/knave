#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SchemeRecording {
    #[default]
    Dropped,
    Kept,
}

pub trait InlineSchemes {
    const RECORDING: SchemeRecording;
}

pub struct DroppedInlineSchemes;

pub struct KeptInlineSchemes;

impl InlineSchemes for DroppedInlineSchemes {
    const RECORDING: SchemeRecording = SchemeRecording::Dropped;
}

impl InlineSchemes for KeptInlineSchemes {
    const RECORDING: SchemeRecording = SchemeRecording::Kept;
}
