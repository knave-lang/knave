use super::introduced_name::IntroducedName;
use super::macro_origin::MacroOrigin;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct OriginRow {
    pub origin: MacroOrigin,
    pub introduced: Vec<IntroducedName>,
}
