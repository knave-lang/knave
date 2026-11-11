#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct FormShape(u32);

impl FormShape {
    pub const LARGEST_STORED: usize = Self::LARGEST_FORM_COUNT as usize;
    const NESTING_BITS: u32 = 13;
    const NESTING_MASK: u32 = (1 << Self::NESTING_BITS) - 1;
    const LARGEST_FORM_COUNT: u32 = u32::MAX >> Self::NESTING_BITS;
    const ONE_FORM: usize = 1;

    pub fn nesting(self) -> u32 {
        self.0 & Self::NESTING_MASK
    }

    pub fn stored_form_count(self) -> usize {
        (self.0 >> Self::NESTING_BITS) as usize
    }

    pub(super) fn leaf() -> Self {
        Self::new(0, Self::ONE_FORM)
    }

    pub(super) fn of_children(children: impl Iterator<Item = Self>) -> Self {
        let (deepest, forms) = children.fold((0, Self::ONE_FORM), |(deepest, forms), child| {
            (
                deepest.max(child.nesting()),
                forms + child.stored_form_count(),
            )
        });
        Self::new(deepest + 1, forms)
    }

    pub(super) fn form_count(self) -> Option<usize> {
        let forms = self.stored_form_count();
        (forms < Self::LARGEST_STORED).then_some(forms)
    }

    fn new(nesting: u32, forms: usize) -> Self {
        let nesting = nesting.min(Self::NESTING_MASK);
        let forms = u32::try_from(forms).map_or(Self::LARGEST_FORM_COUNT, |forms| {
            forms.min(Self::LARGEST_FORM_COUNT)
        });
        Self(forms << Self::NESTING_BITS | nesting)
    }
}

#[cfg(test)]
mod tests {
    use super::FormShape;
    use crate::reader::{Form, FormKind, Symbol};
    use crate::source::{SourceId, Span};

    #[test]
    fn a_stored_count_stops_at_the_largest() {
        let span = Span::new(SourceId::new(0), 0..1);
        let leaf = Form::new(FormKind::Symbol(Symbol::new("leaf")), span);
        let list = Form::new(FormKind::List(vec![leaf; FormShape::LARGEST_STORED]), span);
        assert_eq!(
            FormShape::from(&list).stored_form_count(),
            FormShape::LARGEST_STORED
        );
    }

    #[test]
    fn a_count_past_what_a_form_stores_is_kept_at_the_largest() {
        let shape = FormShape::new(3, (1 << 32) + 1);
        assert_eq!(shape.form_count(), None);
        assert_eq!(shape.nesting(), 3);
    }

    #[test]
    fn a_count_that_fits_32_bits_but_not_the_form_is_kept_at_the_largest() {
        let shape = FormShape::new(3, u32::MAX as usize);
        assert_eq!(shape.form_count(), None);
        assert_eq!(shape.nesting(), 3);
    }

    #[test]
    fn a_count_below_the_largest_is_stored() {
        let count = FormShape::LARGEST_STORED - 1;
        assert_eq!(FormShape::new(0, count).form_count(), Some(count));
    }

    #[test]
    fn a_nesting_past_what_a_form_stores_is_kept_at_the_largest() {
        let shape = FormShape::new(u32::MAX, 5);
        assert_eq!(shape.nesting(), FormShape::NESTING_MASK);
        assert_eq!(shape.form_count(), Some(5));
    }
}
