use crate::core::ModulePath;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::interpret::{LimitProblems, ListNesting};
use crate::modules::ModuleSources;
use crate::source::{SourceMap, Span};

pub struct ExpansionLimit;

impl ExpansionLimit {
    pub fn stopped(failure: &Diagnostic) -> bool {
        Self::ran_too_many_steps(failure)
            || failure.title() == LimitProblems::BUILT_LIST_NESTS_TOO_DEEP
    }

    pub fn ran_too_many_steps(failure: &Diagnostic) -> bool {
        failure.code() == Some(&DiagnosticCode::MACRO_STEP_LIMIT)
    }

    pub fn at_user_call(failure: Diagnostic, call_site: Span, sources: &SourceMap) -> Diagnostic {
        let culprit = sources.file(failure.primary().span().source()).name();
        if failure.title() == LimitProblems::BUILT_LIST_NESTS_TOO_DEEP && Self::in_prelude(culprit)
        {
            LimitProblems::value_nests_too_deep(ListNesting::LIMIT, call_site)
        } else {
            failure
        }
    }

    fn in_prelude(file_name: &str) -> bool {
        file_name
            .rsplit_once('.')
            .is_some_and(|(module, _)| ModuleSources::is_embedded_prelude(&ModulePath::new(module)))
    }
}
