use super::fragment::{Code, Fragment, Precedence};
use super::template_arguments::TemplateArguments;
use crate::core::{ParsedTemplate, PrimitiveType, RustTemplate};
use crate::types::{Type, TypedArgument};

pub struct DoubleNegation<'a> {
    template: &'a RustTemplate,
    arguments: &'a [TypedArgument],
}

impl<'a> DoubleNegation<'a> {
    const NEGATION: char = '!';

    pub fn new(template: &'a RustTemplate, arguments: &'a [TypedArgument]) -> Self {
        Self {
            template,
            arguments,
        }
    }

    pub fn cancelled(&self, codes: &TemplateArguments) -> Option<Code> {
        let [argument] = self.arguments else {
            return None;
        };
        let negates_a_bool = argument.value.ty == Type::Primitive(PrimitiveType::Bool)
            && ParsedTemplate::parse(&self.template.template)
                .is_ok_and(|parsed| parsed.operation().is_logical_not());
        let negated = codes
            .written(0)
            .filter(|code| code.precedence == Precedence::Prefix)?;
        let fragments = Self::without_negation(&negated.fragments)?;
        negates_a_bool.then_some(Code {
            fragments,
            precedence: Precedence::Atom,
            bare_struct_literal: negated.bare_struct_literal,
        })
    }

    fn without_negation(fragments: &[Fragment]) -> Option<Vec<Fragment>> {
        let (first, rest) = fragments.split_first()?;
        let stripped = match first {
            Fragment::Text(text) => {
                let inner = text.strip_prefix(Self::NEGATION)?;
                if inner.starts_with(Self::NEGATION) {
                    return None;
                }
                Fragment::Text(inner.to_string())
            }
            Fragment::Spanned(span, inner) => {
                Fragment::Spanned(*span, Self::without_negation(inner)?)
            }
            Fragment::Newline | Fragment::Indented(_) => return None,
        };
        Some(
            std::iter::once(stripped)
                .chain(rest.iter().cloned())
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::DoubleNegation;
    use crate::emit::fragment::Fragment;
    use crate::source::{SourceId, Span};

    struct NegationFixture;

    impl NegationFixture {
        fn without_negation(fragments: Vec<Fragment>) -> Option<String> {
            let stripped = DoubleNegation::without_negation(&fragments)?;
            Some(stripped.iter().map(Self::text).collect())
        }

        fn text(fragment: &Fragment) -> String {
            match fragment {
                Fragment::Text(text) => text.clone(),
                Fragment::Spanned(_, inner) => inner.iter().map(Self::text).collect(),
                Fragment::Newline | Fragment::Indented(_) => String::new(),
            }
        }
    }

    #[test]
    fn drops_one_negation_from_the_first_text() {
        let fragments = vec![Fragment::Text("!f(".into()), Fragment::Text("x)".into())];
        assert_eq!(
            NegationFixture::without_negation(fragments),
            Some("f(x)".into())
        );
    }

    #[test]
    fn drops_a_negation_inside_a_spanned_fragment() {
        let fragments = vec![Fragment::Spanned(
            Span::new(SourceId::new(0), 0..2),
            vec![Fragment::Text("!x".into())],
        )];
        assert_eq!(
            NegationFixture::without_negation(fragments),
            Some("x".into())
        );
    }

    #[test]
    fn keeps_a_double_negation_and_a_text_that_starts_on_a_new_line() {
        assert_eq!(
            NegationFixture::without_negation(vec![Fragment::Text("!!x".into())]),
            None
        );
        assert_eq!(
            NegationFixture::without_negation(vec![Fragment::Newline]),
            None
        );
        assert_eq!(
            NegationFixture::without_negation(vec![Fragment::Text("x".into())]),
            None
        );
    }
}
