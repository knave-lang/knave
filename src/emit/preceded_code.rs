use super::fragment::{Code, Fragment};
use super::let_statement::LetStatement;

pub struct PrecededCode {
    statements: Vec<Fragment>,
    code: Code,
}

impl PrecededCode {
    pub fn new(statements: Vec<Fragment>, code: Code) -> Self {
        Self { statements, code }
    }

    pub fn plain(code: Code) -> Self {
        Self::new(Vec::new(), code)
    }

    pub fn needs_binding(&self) -> bool {
        !self.statements.is_empty() || !self.code.is_plain_name()
    }

    pub fn bind(self, binding: &str) -> Vec<Fragment> {
        let mut fragments = self.statements;
        fragments.extend(LetStatement::new(binding, self.code).fragments());
        fragments
    }

    pub fn into_code(self) -> Code {
        assert!(
            self.statements.is_empty(),
            "only a binding of the code can run the statements before it"
        );
        self.code
    }
}

#[cfg(test)]
mod tests {
    use super::PrecededCode;
    use crate::emit::fragment::{Code, Fragment};

    struct PrecededFixture;

    impl PrecededFixture {
        fn with_statement(code: &str) -> PrecededCode {
            let statement = Fragment::Text("let owner = make();".to_string());
            PrecededCode::new(vec![statement], Code::atom(code))
        }
    }

    #[test]
    fn a_plain_name_needs_no_binding() {
        assert!(!PrecededCode::plain(Code::atom("total")).needs_binding());
    }

    #[test]
    fn an_expression_needs_a_binding() {
        assert!(PrecededCode::plain(Code::atom("next()")).needs_binding());
    }

    #[test]
    fn a_name_with_statements_needs_a_binding() {
        assert!(PrecededFixture::with_statement("owner").needs_binding());
    }

    #[test]
    fn binds_after_its_statements() {
        let fragments = PrecededFixture::with_statement("owner.get()").bind("value");
        assert_eq!(
            Code::text_of(&fragments),
            "let owner = make();let value = owner.get();"
        );
    }

    #[test]
    fn gives_back_code_without_statements() {
        let code = PrecededCode::plain(Code::atom("total")).into_code();
        assert_eq!(code.text(), "total");
    }

    #[test]
    #[should_panic(expected = "only a binding of the code can run the statements before it")]
    fn refuses_to_drop_its_statements() {
        PrecededFixture::with_statement("owner").into_code();
    }
}
