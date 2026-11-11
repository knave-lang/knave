use super::{
    DefinitionQueries, ExpansionCallbacks, GlobalValues, Host, ModuleNotes, ModuleQueries,
    NameComparison, SourceQueries, Value, Warnings,
};
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub(super) struct PartialHost {
    offers_groups: bool,
}

impl PartialHost {
    pub(super) fn offering_nothing() -> Self {
        Self {
            offers_groups: false,
        }
    }

    pub(super) fn offering_its_groups() -> Self {
        Self {
            offers_groups: true,
        }
    }
}

impl GlobalValues for PartialHost {
    fn global_value(&mut self, _binding: BindingId, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::Bool(true))
    }
}

impl Warnings for PartialHost {
    fn report_warning(&mut self, _warning: Diagnostic) {}
}

impl Host for PartialHost {
    fn global_values(&mut self) -> Option<&mut dyn GlobalValues> {
        if self.offers_groups { Some(self) } else { None }
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
        if self.offers_groups { Some(self) } else { None }
    }
}
