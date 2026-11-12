use crate::source::Span;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct IntroducedName {
    pub name: String,
    pub written: Span,
}
