use super::bindings::Bindings;
use super::docstring_notes::DocstringNotes;
use crate::core::{ModuleId, ModulePath, Namespace};
use crate::modules::{ModuleRegistry, ModuleSources};

pub(super) struct NotesModule<'t> {
    modules: &'t ModuleRegistry,
    bindings: Bindings<'t>,
}

impl<'t> NotesModule<'t> {
    pub(super) fn new(modules: &'t ModuleRegistry, bindings: Bindings<'t>) -> Self {
        Self { modules, bindings }
    }

    pub(super) fn find(&self) -> Option<ModuleId> {
        let prelude = self
            .modules
            .find(&ModulePath::new(ModuleSources::PRELUDE))?;
        let page_macro = self
            .modules
            .get(prelude)
            .exported(DocstringNotes::PAGE_MACRO, Namespace::Value)?;
        let page_module = self.bindings.definition_module(page_macro)?;
        let key = self
            .modules
            .get(page_module)
            .exported(DocstringNotes::DOCSTRINGS_KEY, Namespace::Value)?;
        self.bindings.definition_module(key)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::expand::Expander;
    use crate::source::SourceMap;

    struct NotesModuleFixture {
        sources: SourceMap,
    }

    impl NotesModuleFixture {
        const MODULE: &str = "notes";

        fn new() -> Self {
            Self {
                sources: SourceMap::default(),
            }
        }

        fn defines_the_docstrings_key(&mut self) -> bool {
            let entry = self.sources.add(Self::MODULE, "");
            let mut expander = Expander::new(
                &mut self.sources,
                ModuleSources::new(PathBuf::new(), PathBuf::new()),
            );
            expander.load_source_module(ModulePath::new(Self::MODULE), &[entry]);
            let bindings = expander.bindings();
            let notes = NotesModule::new(&expander.modules, bindings).find();
            notes.is_some_and(|module| {
                let key = expander
                    .modules
                    .get(module)
                    .exported(DocstringNotes::DOCSTRINGS_KEY, Namespace::Value);
                key.and_then(|key| bindings.definition_module(key)) == Some(module)
            })
        }

        fn found_without_a_prelude(&mut self) -> Option<ModuleId> {
            let expander = Expander::new(
                &mut self.sources,
                ModuleSources::new(PathBuf::new(), PathBuf::new()),
            );
            let empty = ModuleRegistry::default();
            NotesModule::new(&empty, expander.bindings()).find()
        }
    }

    #[test]
    fn the_notes_module_is_the_prelude_module_that_defines_the_docstrings_key() {
        assert!(NotesModuleFixture::new().defines_the_docstrings_key());
    }

    #[test]
    fn a_registry_without_the_prelude_has_no_notes_module() {
        assert_eq!(NotesModuleFixture::new().found_without_a_prelude(), None);
    }
}
