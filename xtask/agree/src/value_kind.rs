#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    Number,
    Numbers,
    Car,
    ElementBorrow,
    MakeBorrow,
    Tag,
    HeldBorrows,
    Reader,
    Changer,
}

impl ValueKind {
    pub fn names(self) -> &'static [&'static str] {
        match self {
            Self::Number => &["n", "k"],
            Self::Numbers => &["v", "w", "u"],
            Self::Car => &["c", "d"],
            Self::ElementBorrow => &["r", "s"],
            Self::MakeBorrow => &["m", "label"],
            Self::Tag => &["t"],
            Self::HeldBorrows => &["held"],
            Self::Reader => &["size-of"],
            Self::Changer => &["grow-it"],
        }
    }
}
