use std::rc::Rc;

use super::DiagnosticCode;

pub struct CodeStage {
    member: DiagnosticCode,
}

impl CodeStage {
    pub const MACROS: Self = Self {
        member: DiagnosticCode::MACRO_FAILED,
    };
    pub const LINTS: Self = Self {
        member: DiagnosticCode::UNMATCHED_LINK,
    };
    const STAGE_PREFIX_LENGTH: usize = 3;

    pub fn code_named(&self, name: &str) -> Option<DiagnosticCode> {
        self.codes()
            .find(|(source_name, _)| source_name == name)
            .map(|(_, code)| code)
    }

    pub fn names(&self) -> Vec<Rc<str>> {
        self.codes()
            .map(|(source_name, _)| Rc::from(source_name))
            .collect()
    }

    pub fn name_of(&self, wanted: &DiagnosticCode) -> Option<String> {
        self.codes()
            .find(|(_, code)| code == wanted)
            .map(|(source_name, _)| source_name)
    }

    fn codes(&self) -> impl Iterator<Item = (String, DiagnosticCode)> {
        DiagnosticCode::named()
            .filter(|(_, code)| self.holds(code))
            .map(|(constant, code)| (Self::source_name(constant), code))
    }

    fn holds(&self, code: &DiagnosticCode) -> bool {
        Self::stage_prefix(code) == Self::stage_prefix(&self.member)
    }

    fn stage_prefix(code: &DiagnosticCode) -> &str {
        &code.as_str()[..Self::STAGE_PREFIX_LENGTH]
    }

    fn source_name(constant: &str) -> String {
        constant.replace('_', "-").to_lowercase()
    }
}
