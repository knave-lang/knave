use std::rc::Rc;

use anstream::ColorChoice;

use super::diagnostic_output::{DiagnosticOutput, RenderedDiagnostics};
use super::interruption::Interruption;

pub struct TerminalSettings {
    diagnostics: Rc<dyn DiagnosticOutput>,
    interruption: Interruption,
}

impl TerminalSettings {
    pub fn rendered(standard_error: ColorChoice) -> Self {
        Self::reporting_to(Rc::new(RenderedDiagnostics::new(standard_error)))
    }

    pub fn reporting_to(diagnostics: Rc<dyn DiagnosticOutput>) -> Self {
        Self {
            diagnostics,
            interruption: Interruption::default(),
        }
    }

    pub fn stopping_on(self, interruption: Interruption) -> Self {
        Self {
            interruption,
            ..self
        }
    }

    pub fn rerouted_to(&self, diagnostics: Rc<dyn DiagnosticOutput>) -> Self {
        Self {
            diagnostics,
            interruption: self.interruption.clone(),
        }
    }

    pub fn diagnostics(&self) -> &Rc<dyn DiagnosticOutput> {
        &self.diagnostics
    }

    pub fn interruption(&self) -> &Interruption {
        &self.interruption
    }
}
