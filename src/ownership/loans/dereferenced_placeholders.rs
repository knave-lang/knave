use crate::core::{Access, ParsedTemplate, Piece};

pub struct DereferencedPlaceholders<'t> {
    template: &'t ParsedTemplate,
}

impl<'t> DereferencedPlaceholders<'t> {
    const AUTODEREFERENCING: [char; 2] = ['.', '['];
    const RANGE: &'static str = "..";

    pub fn new(template: &'t ParsedTemplate) -> Self {
        Self { template }
    }

    pub fn dereference_every_use(&self, argument: usize) -> bool {
        self.template.dereferences(argument) || self.receive_every_use(argument)
    }

    fn receive_every_use(&self, argument: usize) -> bool {
        let pieces = self.template.pieces();
        let mut uses = pieces
            .iter()
            .enumerate()
            .filter(|(_, piece)| matches!(piece, Piece::Placeholder { index, .. } if *index == argument))
            .peekable();
        uses.peek().is_some()
            && uses.all(|(position, piece)| Self::receives(piece, pieces.get(position + 1)))
    }

    fn receives(piece: &Piece, next: Option<&Piece>) -> bool {
        let passed =
            matches!(piece, Piece::Placeholder { access, .. } if *access == Access::Passed);
        passed
            && matches!(next, Some(Piece::Literal(literal))
                if literal.starts_with(Self::AUTODEREFERENCING) && !literal.starts_with(Self::RANGE))
    }
}

#[cfg(test)]
mod tests {
    use super::DereferencedPlaceholders;
    use crate::core::ParsedTemplate;

    struct PlaceholderFixture;

    impl PlaceholderFixture {
        fn dereferenced(text: &str) -> bool {
            let template = ParsedTemplate::parse(text).expect("the template is well formed");
            DereferencedPlaceholders::new(&template).dereference_every_use(0)
        }
    }

    #[test]
    fn a_receiver_or_a_dereference_reaches_the_referent() {
        assert!(PlaceholderFixture::dereferenced("{0}.trim()"));
        assert!(PlaceholderFixture::dereferenced("{0}.len() + {0}.len()"));
        assert!(PlaceholderFixture::dereferenced("*{0}"));
        assert!(PlaceholderFixture::dereferenced("&{0}[{1}]"));
    }

    #[test]
    fn a_use_that_passes_the_value_on_keeps_the_reference_whole() {
        assert!(!PlaceholderFixture::dereferenced("{0}"));
        assert!(!PlaceholderFixture::dereferenced(
            "if {0}.is_empty() {{ {0} }} else {{ {0} }}"
        ));
        assert!(!PlaceholderFixture::dereferenced("{0}..{1}"));
        assert!(!PlaceholderFixture::dereferenced("{&0}.len()"));
        assert!(!PlaceholderFixture::dereferenced("{1}.len()"));
    }
}
