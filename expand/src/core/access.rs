#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Access {
    Passed,
    Borrowed,
    Owned,
}

impl Access {
    pub(super) const BORROW: char = '&';
    pub(super) const OWNED: &'static str = "owned ";

    pub(super) fn marker(&self) -> String {
        match self {
            Self::Passed => String::new(),
            Self::Borrowed => Self::BORROW.to_string(),
            Self::Owned => Self::OWNED.to_string(),
        }
    }
}
