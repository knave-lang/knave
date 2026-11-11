use std::rc::Rc;

use super::DiagnosticCode;

pub struct CodeStage {
    prefix: &'static str,
}

impl CodeStage {
    pub const MACROS: Self = Self { prefix: "L03" };
    pub const LINTS: Self = Self { prefix: "L08" };

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
            .filter(|(_, code)| code.as_str().starts_with(self.prefix))
            .map(|(constant, code)| (Self::source_name(constant), code))
    }

    fn source_name(constant: &str) -> String {
        constant.replace('_', "-").to_lowercase()
    }
}
