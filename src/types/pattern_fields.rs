pub enum PatternFields<'a> {
    Named(Vec<&'a str>),
    Positional(usize),
}

impl PatternFields<'_> {
    pub fn count(&self) -> usize {
        match self {
            Self::Named(names) => names.len(),
            Self::Positional(count) => *count,
        }
    }
}
