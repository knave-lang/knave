use super::binder_reach::BinderReach;
use super::call_origins::CallOrigins;
use super::introduced_names::IntroducedNames;
use super::macro_call_lookup::MacroCallLookup;
use crate::reader::Symbol;
use crate::scope::Scopes;
use crate::source::Span;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct MacroOrigins {
    pub calls: CallOrigins,
    pub introduced: IntroducedNames,
}

impl MacroOrigins {
    pub(super) fn reach(
        &self,
        scopes: &Scopes,
        binder: &Symbol,
        span: Span,
    ) -> Option<BinderReach> {
        if !scopes.introduced(binder) {
            return Some(BinderReach::written_at(span));
        }
        let origin = self
            .calls
            .introducing_macro(span)
            .filter(|origin| origin.in_project)?;
        Some(BinderReach {
            binder: self.introduced.at(span, binder.name()).unwrap_or(span),
            introducing_macro: Some(origin.name),
        })
    }
}
