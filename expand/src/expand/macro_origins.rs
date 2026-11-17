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
        if !scopes.introduced(binder) {
            return Some(BinderReach::Written {
                binder: site.span(),
            });
        }
        let writer = self.introducing_macros.project_writer_of(binder)?;
        let written = self
            .introduced
            .first_written_at(site.span(), binder.name())
            .unwrap_or(site.span());
        let passed_in = writer
            .generator_call
            .is_some_and(|generator_call| generator_call.contains(written))
            && site.is_spelled_at(sources, written);
        if passed_in {
            return Some(BinderReach::Written { binder: written });
        }
        Some(BinderReach::InMacro {
            binder: written,
            call: site.span(),
            introducing_macro: writer.name,
        })
    }
}
