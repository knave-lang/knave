use rustc_hash::FxHashMap;

use super::introduced_name::IntroducedName;
use crate::source::Span;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct OwnIntroductions {
    names: FxHashMap<(Span, String), Span>,
}

impl OwnIntroductions {
    pub fn at(&self, call_site: Span, name: &str) -> Option<Span> {
        self.names.get(&(call_site, name.to_string())).copied()
    }

    pub fn record(&mut self, call_site: Span, name: &str, written: Span) {
        self.names
            .entry((call_site, name.to_string()))
            .or_insert(written);
    }

    pub fn into_names_by_call_site(self) -> FxHashMap<Span, Vec<IntroducedName>> {
        let mut names_by_call_site: FxHashMap<Span, Vec<IntroducedName>> = FxHashMap::default();
        for ((call_site, name), written) in self.names {
            names_by_call_site
                .entry(call_site)
                .or_default()
                .push(IntroducedName { name, written });
        }
        for names in names_by_call_site.values_mut() {
            names.sort_by(|left, right| left.name.cmp(&right.name));
        }
        names_by_call_site
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}
