pub struct SetAsideInput<'a> {
    names: &'a [&'a str],
    source: &'a str,
    setter: &'a str,
    terms: SetAsideTerms,
}

enum SetAsideTerms {
    Definitions,
    Documents,
}

impl<'a> SetAsideInput<'a> {
    const LISTED_NAMES: usize = 3;

    pub fn new(names: &'a [&'a str], source: &'a str, setter: &'a str) -> Self {
        Self {
            names,
            source,
            setter,
            terms: SetAsideTerms::Definitions,
        }
    }

    pub fn documents(terms: &'a [&'a str], source: &'a str, setter: &'a str) -> Self {
        Self {
            terms: SetAsideTerms::Documents,
            ..Self::new(terms, source, setter)
        }
    }

    pub(super) fn title(&self) -> String {
        format!("set aside {} after `{}`", self.listed_names(), self.setter)
    }

    pub(super) fn replaced_note(&self, replacing: &str) -> String {
        let listed = self.listed_names();
        match self.terms {
            SetAsideTerms::Definitions => {
                format!("`{replacing}` replaced the input that defined {listed}")
            }
            SetAsideTerms::Documents => {
                format!("a `document` of `{replacing}` replaced the input that documented {listed}")
            }
        }
    }

    pub(super) fn enter_again(&self) -> String {
        let listed = self.listed_names();
        match self.terms {
            SetAsideTerms::Definitions => {
                let pronoun = if self.names.len() > 1 { "them" } else { "it" };
                format!("enter {listed} again to use {pronoun}")
            }
            SetAsideTerms::Documents => {
                format!("enter the `document` of {listed} again to keep it")
            }
        }
    }

    fn listed_names(&self) -> String {
        let quoted: Vec<String> = self.names.iter().map(|name| format!("`{name}`")).collect();
        match quoted.as_slice() {
            [] => format!("`{}`", self.source),
            [only] => only.clone(),
            shown if shown.len() <= Self::LISTED_NAMES => {
                let (last, rest) = shown.split_last().expect("two names or more");
                format!("{} and {last}", rest.join(", "))
            }
            more => format!(
                "{} and {} more",
                more[..Self::LISTED_NAMES].join(", "),
                more.len() - Self::LISTED_NAMES
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SetAsideFixture;

    impl SetAsideFixture {
        const SOURCE: &str = "<input 4>";
        const SETTER: &str = "<input 5>";
    }

    #[test]
    fn a_set_aside_definition_says_what_defined_it() {
        let names = ["k"];
        let input = SetAsideInput::new(&names, SetAsideFixture::SOURCE, SetAsideFixture::SETTER);
        assert_eq!(
            input.replaced_note("g"),
            "`g` replaced the input that defined `k`"
        );
        assert_eq!(input.enter_again(), "enter `k` again to use it");
    }

    #[test]
    fn a_set_aside_document_says_what_documented_it() {
        let terms = ["k"];
        let input =
            SetAsideInput::documents(&terms, SetAsideFixture::SOURCE, SetAsideFixture::SETTER);
        assert_eq!(
            input.replaced_note("g"),
            "a `document` of `g` replaced the input that documented `k`"
        );
        assert_eq!(
            input.enter_again(),
            "enter the `document` of `k` again to keep it"
        );
    }

    #[test]
    fn the_title_names_the_input_that_set_the_terms_aside() {
        let terms = ["k"];
        let input =
            SetAsideInput::documents(&terms, SetAsideFixture::SOURCE, SetAsideFixture::SETTER);
        assert_eq!(input.title(), "set aside `k` after `<input 5>`");
    }
}
