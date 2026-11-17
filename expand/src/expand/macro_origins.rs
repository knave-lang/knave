use super::binder_reach::BinderReach;
use super::call_origins::CallOrigins;
use super::introduced_names::IntroducedNames;
use super::project_introductions::ProjectIntroductions;
use crate::reader::Symbol;
use crate::scope::Scopes;
use crate::source::Span;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct MacroOrigins {
    pub calls: CallOrigins,
    pub introduced: IntroducedNames,
    #[serde(skip)]
    pub project_introductions: ProjectIntroductions,
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
        let introducing_macro = self.project_introductions.writer_of(binder)?;
        Some(BinderReach::InMacro {
            binder: self.introduced.at(span, binder.name()).unwrap_or(span),
            call: span,
            introducing_macro,
        })
    }
}
