use super::binder_reach::BinderReach;
use super::binder_site::BinderSite;
use super::call_origins::CallOrigins;
use super::introduced_names::IntroducedNames;
use super::introducing_macros::IntroducingMacros;
use crate::scope::Scopes;
use crate::source::SourceMap;

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
        sources: &SourceMap,
        site: &BinderSite<'_>,
    ) -> Option<BinderReach> {
        let binder = site.symbol();
        if !scopes.introductions().carried_by(binder) {
            return Some(BinderReach::Written {
                binder: site.span(),
            });
        }
        let writer = self.introducing_macros.project_writer_of(binder)?;
        let written = self
            .introduced
            .first_written_at(site.span(), binder.name())
            .unwrap_or(site.span());
        if writer.passed_in(site, sources, written) {
            return Some(BinderReach::PassedIn {
                binder: written,
                call: site.span(),
            });
        }
        Some(BinderReach::InMacro {
            binder: written,
            call: site.span(),
            introducing_macro: writer.name(),
        })
    }
}
