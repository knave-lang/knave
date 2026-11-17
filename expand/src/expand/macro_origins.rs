use super::binder_reach::BinderReach;
use super::call_origins::CallOrigins;
use super::introduced_names::IntroducedNames;
use super::introducing_macros::IntroducingMacros;
use crate::reader::Symbol;
use crate::scope::Scopes;
use crate::source::Span;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct MacroOrigins {
    pub calls: CallOrigins,
    pub introduced: IntroducedNames,
    #[serde(skip)]
    pub introducing_macros: IntroducingMacros,
}

impl MacroOrigins {
    pub(super) fn reach(
        &self,
        scopes: &Scopes,
        binder: &Symbol,
        span: Span,
    ) -> Option<BinderReach> {
        if !scopes.introduced(binder) {
            return Some(BinderReach::Written { binder: span });
        }
        let written = self
            .introduced
            .first_written_at(span, binder.name())
            .unwrap_or(span);
        self.introducing_macros.reach_of(binder, written, span)
    }
}
