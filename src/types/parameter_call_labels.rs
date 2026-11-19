use super::checker::TypeChecker;
use super::parameter_call_request::ParameterCallRequest;
use super::parameter_call_search::CalledParameters;
use super::problems::TypeProblems;
use crate::core::CoreModule;
use crate::diagnostic::Diagnostic;

struct ParameterCallLabels<'m> {
    calls: CalledParameters<'m>,
}

impl TypeChecker {
    pub(super) fn label_parameter_calls(&mut self, modules: &[CoreModule]) {
        if self.deferred.label_requests.is_empty() {
            return;
        }
        let requests = std::mem::take(&mut self.deferred.label_requests);
        let diagnostics = std::mem::take(&mut self.diagnostics);
        let labels = ParameterCallLabels {
            calls: self.called_parameters(modules),
        };
        let labeled = diagnostics.map(|diagnostic| labels.label(&requests, diagnostic));
        self.diagnostics = labeled;
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
        TypeProblems::parameter_call(diagnostic, self.calls.function_name(request.callee), &call)
    }
}
