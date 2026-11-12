use super::DiagnosticCode;

macro_rules! written_explanations {
    ($($code:literal,)*) => {
        impl Explanation {
            const WRITTEN: &[(&str, &str)] =
                &[$(($code, include_str!(concat!("explanations/", $code, ".md")))),*];
        }
    };
}

written_explanations! {
    "L0106",
    "L0201",
    "L0208",
    "L0210",
    "L0351",
    "L0401",
    "L0402",
    "L0433",
    "L0501",
    "L0514",
}

pub struct Explanation {
    code: DiagnosticCode,
    constant: &'static str,
}

impl Explanation {
    const CONSTANT_WORD_SEPARATOR: char = '_';
    const DISPLAY_WORD_SEPARATOR: &str = " ";

    pub fn of_code(code: &str) -> Option<Self> {
        Self::all().find(|explanation| explanation.code.as_str() == code)
    }

    pub fn all() -> impl Iterator<Item = Self> {
        DiagnosticCode::named().map(|(constant, code)| Self { code, constant })
    }

    pub fn code(&self) -> &DiagnosticCode {
        &self.code
    }

    pub fn display_name(&self) -> String {
        self.constant
            .replace(Self::CONSTANT_WORD_SEPARATOR, Self::DISPLAY_WORD_SEPARATOR)
            .to_lowercase()
    }

    pub fn text(&self) -> Option<&'static str> {
        Self::WRITTEN
            .iter()
            .find(|(code, _)| *code == self.code.as_str())
            .map(|(_, text)| *text)
    }
}

#[cfg(test)]
mod tests {
    use super::Explanation;

    #[test]
    fn the_display_name_spells_the_constant_in_lowercase_words() {
        let explanation = Explanation::of_code("L0501").expect("L0501 is a code");
        assert_eq!(explanation.display_name(), "use after move");
    }

    #[test]
    fn a_code_outside_the_table_has_no_explanation() {
        assert!(Explanation::of_code("L0999").is_none());
    }
}
