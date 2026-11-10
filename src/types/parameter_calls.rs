use super::called_parameters::CalledParameters;
use super::checker::TypeChecker;
use super::problems::TypeProblems;
use crate::core::{BindingArena, BindingId, CoreModule};
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::source::Span;

pub(super) struct ParameterCallRequest {
    pub callee: BindingId,
    pub position: usize,
    pub argument: Span,
}

impl ParameterCallRequest {
    fn explains(&self, diagnostic: &Diagnostic) -> bool {
        diagnostic.code() == Some(&DiagnosticCode::MISMATCHED_TYPES)
            && diagnostic.primary().span() == self.argument
    }
}

struct ParameterCallLabels<'m> {
    calls: CalledParameters<'m>,
    bindings: &'m BindingArena,
}

impl TypeChecker {
    pub(super) fn label_parameter_calls(&mut self, modules: &[CoreModule]) {
        if self.parameter_call_requests.is_empty() {
            return;
        }
        let requests = std::mem::take(&mut self.parameter_call_requests);
        let labels = ParameterCallLabels {
            calls: CalledParameters::new(modules, &self.bindings),
            bindings: &self.bindings,
        };
        let diagnostics = std::mem::take(&mut self.diagnostics);
        self.diagnostics = diagnostics.map(|diagnostic| labels.label(&requests, diagnostic));
    }
}

impl ParameterCallLabels<'_> {
    fn label(&self, requests: &[ParameterCallRequest], diagnostic: Diagnostic) -> Diagnostic {
        let Some(request) = requests
            .iter()
            .find(|request| request.explains(&diagnostic))
        else {
            return diagnostic;
        };
        let Some(call) = self.calls.first_call_of(request.callee, request.position) else {
            return diagnostic;
        };
        TypeProblems::parameter_call(diagnostic, self.bindings.get(request.callee).name(), &call)
    }
}
