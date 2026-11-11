use super::fragment::Code;
use super::preceded_code::PrecededCode;

pub enum TemplateArguments {
    Written(Vec<Code>),
    Bound(Vec<PrecededCode>),
}

impl TemplateArguments {
    pub fn none() -> Self {
        Self::Written(Vec::new())
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Written(codes) => codes.len(),
            Self::Bound(codes) => codes.len(),
        }
    }

    pub fn replace(&mut self, position: usize, code: Code) {
        match self {
            Self::Written(codes) => codes[position] = code,
            Self::Bound(codes) => codes[position] = PrecededCode::plain(code),
        }
    }

    pub fn bound(self) -> Self {
        match self {
            Self::Written(codes) => {
                Self::Bound(codes.into_iter().map(PrecededCode::plain).collect())
            }
            bound => bound,
        }
    }

    pub fn into_substituted(self) -> Result<Vec<Code>, Vec<PrecededCode>> {
        match self {
            Self::Written(codes) => Ok(codes),
            Self::Bound(codes) if codes.iter().any(PrecededCode::needs_binding) => Err(codes),
            Self::Bound(codes) => Ok(codes
                .into_iter()
                .filter_map(PrecededCode::into_name)
                .collect()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TemplateArguments;
    use crate::emit::fragment::{Code, Fragment};
    use crate::emit::preceded_code::PrecededCode;

    struct ArgumentsFixture;

    impl ArgumentsFixture {
        fn written(texts: &[&str]) -> TemplateArguments {
            TemplateArguments::Written(texts.iter().map(|text| Code::atom(*text)).collect())
        }

        fn bound(texts: &[&str]) -> TemplateArguments {
            TemplateArguments::Bound(
                texts
                    .iter()
                    .map(|text| PrecededCode::plain(Code::atom(*text)))
                    .collect(),
            )
        }

        fn with_statement() -> TemplateArguments {
            let statement = Fragment::Text("let owner = make();".to_string());
            TemplateArguments::Bound(vec![PrecededCode::new(
                vec![statement],
                Code::atom("owner"),
            )])
        }

        fn substituted(arguments: TemplateArguments) -> Option<Vec<String>> {
            let codes = arguments.into_substituted().ok()?;
            Some(codes.iter().map(|code| code.text().to_string()).collect())
        }
    }

    #[test]
    fn substitutes_written_code_when_the_template_does_not_bind() {
        let arguments = ArgumentsFixture::written(&["next()"]);
        assert_eq!(
            ArgumentsFixture::substituted(arguments),
            Some(vec!["next()".to_string()])
        );
    }

    #[test]
    fn binds_written_code_when_the_template_binds() {
        let arguments = ArgumentsFixture::written(&["next()"]);
        assert_eq!(ArgumentsFixture::substituted(arguments.bound()), None);
    }

    #[test]
    fn substitutes_written_names_even_when_the_template_binds() {
        let arguments = ArgumentsFixture::written(&["total"]);
        assert_eq!(
            ArgumentsFixture::substituted(arguments.bound()),
            Some(vec!["total".to_string()])
        );
    }

    #[test]
    fn substitutes_bound_names() {
        let arguments = ArgumentsFixture::bound(&["total", "count"]);
        assert_eq!(
            ArgumentsFixture::substituted(arguments.bound()),
            Some(vec!["total".to_string(), "count".to_string()])
        );
    }

    #[test]
    fn binds_arguments_that_carry_statements() {
        assert_eq!(
            ArgumentsFixture::substituted(ArgumentsFixture::with_statement().bound()),
            None
        );
    }

    #[test]
    fn replaces_a_bound_argument_with_plain_code() {
        let mut arguments = ArgumentsFixture::with_statement();
        arguments.replace(0, Code::atom("body"));
        assert_eq!(
            ArgumentsFixture::substituted(arguments.bound()),
            Some(vec!["body".to_string()])
        );
    }

    #[test]
    fn counts_its_arguments() {
        assert_eq!(ArgumentsFixture::written(&["a", "b"]).len(), 2);
    }
}
