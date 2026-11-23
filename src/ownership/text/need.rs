#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Need {
    Owned,
    View,
    Lent,
    OwnedOrMixed,
}

impl Need {
    pub fn stronger(self, other: Self) -> Self {
        self.min(other)
    }
}

#[cfg(test)]
mod tests {
    use super::Need;

    #[test]
    fn an_owned_need_outranks_a_view() {
        assert_eq!(Need::View.stronger(Need::Owned), Need::Owned);
    }

    #[test]
    fn a_view_outranks_owned_or_mixed_text() {
        assert_eq!(Need::OwnedOrMixed.stronger(Need::View), Need::View);
    }
}
