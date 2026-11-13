pub trait Access {
    fn verb(&self) -> &'static str;
}

pub struct Change;

pub struct Move;

pub struct Borrowing;

impl Access for Change {
    fn verb(&self) -> &'static str {
        "change"
    }
}

impl Access for Move {
    fn verb(&self) -> &'static str {
        "move"
    }
}

impl Access for Borrowing {
    fn verb(&self) -> &'static str {
        "borrow"
    }
}
