#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocstringPlace {
    Absent,
    BeforeBody,
    Last,
}

impl DocstringPlace {
    pub fn position(self, header_length: usize, item_count: usize) -> Option<usize> {
        let position = header_length + 1;
        let documents = match self {
            Self::Absent => false,
            Self::BeforeBody => item_count > position + 1,
            Self::Last => item_count == position + 1,
        };
        documents.then_some(position)
    }
}

#[cfg(test)]
mod tests {
    use super::DocstringPlace;

    #[test]
    fn a_docstring_before_the_body_needs_a_form_after_it() {
        assert_eq!(
            (
                DocstringPlace::BeforeBody.position(1, 3),
                DocstringPlace::BeforeBody.position(1, 4)
            ),
            (None, Some(2))
        );
    }

    #[test]
    fn a_last_docstring_ends_the_form() {
        assert_eq!(
            (
                DocstringPlace::Last.position(1, 3),
                DocstringPlace::Last.position(1, 4)
            ),
            (Some(2), None)
        );
    }

    #[test]
    fn an_absent_docstring_has_no_position() {
        assert_eq!(DocstringPlace::Absent.position(1, 4), None);
    }
}
