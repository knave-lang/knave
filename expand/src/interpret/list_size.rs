use super::list_item::ListItem;

pub struct ListSize;

impl ListSize {
    pub const EMPTY: usize = 1;
    pub const ATOM: usize = 1;

    pub fn of_items<I: ListItem>(items: &[I]) -> usize {
        items
            .iter()
            .fold(Self::EMPTY, |size, item| size + item.list_size())
    }

    pub fn of_texts(count: usize) -> usize {
        Self::EMPTY + count * Self::ATOM
    }

    pub fn appended<I: ListItem>(size: usize, extra: &[I]) -> usize {
        size.saturating_add(Self::of_items(extra) - Self::EMPTY)
    }

    pub fn stored(size: usize) -> u32 {
        u32::try_from(size).unwrap_or(u32::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::ListSize;
    use crate::interpret::Value;
    use crate::reader::{Form, FormKind, Symbol};
    use crate::source::{SourceId, Span};

    struct SizedValues;

    impl SizedValues {
        fn span() -> Span {
            Span::new(SourceId::new(0), 0..1)
        }

        fn leaf() -> Form {
            Form::new(FormKind::Symbol(Symbol::new("leaf")), Self::span())
        }

        fn pair() -> Form {
            Form::new(
                FormKind::List(vec![Self::leaf(), Self::leaf()]),
                Self::span(),
            )
        }
    }

    #[test]
    fn a_size_past_what_a_list_stores_is_kept_at_the_largest() {
        assert_eq!(ListSize::stored((1 << 32) + 1), u32::MAX);
    }

    #[test]
    fn a_list_of_forms_counts_itself_and_every_form_of_each_tree() {
        let forms = [SizedValues::pair(), SizedValues::leaf()];
        assert_eq!(ListSize::of_items(&forms), 1 + 3 + 1);
    }

    #[test]
    fn a_list_of_texts_counts_itself_and_one_value_for_each_text() {
        assert_eq!(ListSize::of_texts(4), 1 + 4);
    }

    #[test]
    fn appending_adds_the_size_of_each_new_item() {
        let extra = [SizedValues::pair()];
        assert_eq!(ListSize::appended(5, &extra), 5 + 3);
    }

    #[test]
    fn a_value_that_is_not_code_counts_one() {
        assert_eq!(ListSize::of_items(&[Value::Unit, Value::Bool(true)]), 1 + 2);
    }
}
