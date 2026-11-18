use super::module_load_site::ModuleLoadSite;
use super::module_path_problems::ModulePathProblems;
use super::text_module_record::TextModuleRecord;
use crate::source::SourceMap;

pub(super) struct SecondModulePath {
    first: ModuleLoadSite,
    second: ModuleLoadSite,
}

impl SecondModulePath {
    pub(super) fn new(first: ModuleLoadSite, second: ModuleLoadSite) -> Self {
        Self { first, second }
    }

    pub(super) fn record(self, sources: &SourceMap) -> TextModuleRecord {
        if self.parts_a_library_from_generated_code(sources) {
            return TextModuleRecord::Separate;
        }
        let (kept, refused) = if self.keeps_the_library_path() {
            (&self.second, &self.first)
        } else {
            (&self.first, &self.second)
        };
        let reported =
            Self::in_program_files(refused, sources) && !Self::is_generated(kept, sources);
        let diagnostic = reported.then(|| {
            ModulePathProblems::file_of_another_module(&kept.path, &refused.path, refused.span)
        });
        TextModuleRecord::BindsTo {
            kept: self.first.path,
            diagnostic,
        }
    }

    fn in_program_files(site: &ModuleLoadSite, sources: &SourceMap) -> bool {
        site.importer.is_none() && !Self::is_generated(site, sources)
    }

    fn is_generated(site: &ModuleLoadSite, sources: &SourceMap) -> bool {
        sources.file(site.span.source()).is_generated()
    }

    fn parts_a_library_from_generated_code(&self, sources: &SourceMap) -> bool {
        let generated_local =
            |site: &ModuleLoadSite| site.owner.is_none() && Self::is_generated(site, sources);
        (generated_local(&self.first) && self.second.owner.is_some())
            || (generated_local(&self.second) && self.first.owner.is_some())
    }

    fn keeps_the_library_path(&self) -> bool {
        self.first.owner.is_none() && self.second.owner.is_some()
    }
}
