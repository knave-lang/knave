use super::{
    DefinitionQueries, ExpansionCallbacks, GlobalValues, Host, ModuleNotes, ModuleQueries,
    NameComparison, SourceQueries, Warnings,
};
use crate::diagnostic::Diagnostic;

pub(super) struct PartialHost {
    offers_warnings: bool,
}

impl PartialHost {
    pub(super) fn offering_nothing() -> Self {
        Self {
            offers_warnings: false,
        }
    }

    pub(super) fn offering_warnings() -> Self {
        Self {
            offers_warnings: true,
        }
    }
}

impl Warnings for PartialHost {
    fn report_warning(&mut self, _warning: Diagnostic) {}
}

impl Host for PartialHost {
    fn global_values(&mut self) -> Option<&mut dyn GlobalValues> {
        None
    }

    fn module_queries(&mut self) -> Option<&mut dyn ModuleQueries> {
        None
    }

    fn definition_queries(&mut self) -> Option<&mut dyn DefinitionQueries> {
        None
    }

    fn module_notes(&mut self) -> Option<&mut dyn ModuleNotes> {
        None
    }

    fn name_comparison(&mut self) -> Option<&mut dyn NameComparison> {
        None
    }

    fn expansion_callbacks(&mut self) -> Option<&mut dyn ExpansionCallbacks> {
        None
    }

    fn source_queries(&mut self) -> Option<&mut dyn SourceQueries> {
        None
    }

    fn warnings(&mut self) -> Option<&mut dyn Warnings> {
        if self.offers_warnings {
            Some(self)
        } else {
            None
        }
    }
}
