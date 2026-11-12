#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OneLineRule {
    AnyBody,
    OneBodyForm,
}

impl OneLineRule {
    pub fn allows(self, header_length: usize, item_count: usize) -> bool {
        let body_forms = item_count.saturating_sub(header_length + 1);
        match self {
            Self::AnyBody => true,
            Self::OneBodyForm => body_forms <= 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OneLineRule;

    #[test]
    fn any_body_allows_one_line_at_any_length() {
        assert!(OneLineRule::AnyBody.allows(1, 5));
    }

    #[test]
    fn one_body_form_allows_one_line_with_one_body_form() {
        assert!(OneLineRule::OneBodyForm.allows(1, 3));
    }

    #[test]
    fn one_body_form_refuses_one_line_with_two_body_forms() {
        assert!(!OneLineRule::OneBodyForm.allows(1, 4));
    }
}
