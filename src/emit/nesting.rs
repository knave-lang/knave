use super::literal::Literal;

pub trait Nesting<'a> {
    fn open(&mut self, called_after: Option<&'a str>);

    fn close(&mut self);

    fn depth(&self) -> usize;

    fn count_argument(&mut self) {}

    fn hold_text(&mut self, _text: &'a str) {}

    fn reach(&mut self, _text: &'a str, _position: usize, _literal: Literal) {}
}

#[derive(Default)]
pub struct Depth {
    open: usize,
}

impl Nesting<'_> for Depth {
    fn open(&mut self, _called_after: Option<&str>) {
        self.open += 1;
    }

    fn close(&mut self) {
        self.open = self.open.saturating_sub(1);
    }

    fn depth(&self) -> usize {
        self.open
    }
}
