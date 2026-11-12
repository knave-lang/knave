use super::call_origins::CallOrigins;
use super::introduced_names::IntroducedNames;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct MacroOrigins {
    pub calls: CallOrigins,
    pub introduced: IntroducedNames,
}
