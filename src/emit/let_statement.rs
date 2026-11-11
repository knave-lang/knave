use super::fragment::{Code, Fragment};

pub struct LetStatement {
    binding: String,
    value: Option<Code>,
}

impl LetStatement {
    pub fn new(binding: impl Into<String>, value: Code) -> Self {
        Self {
            binding: binding.into(),
            value: Some(value),
        }
    }

    pub fn declared(binding: impl Into<String>) -> Self {
        Self {
            binding: binding.into(),
            value: None,
        }
    }

    pub fn fragments(self) -> Vec<Fragment> {
        let Some(value) = self.value else {
            return vec![Fragment::Text(format!("let {};", self.binding))];
        };
        let mut fragments = vec![Fragment::Text(format!("let {} = ", self.binding))];
        fragments.extend(value.fragments);
        fragments.push(Fragment::Text(";".to_string()));
        fragments
    }
}

#[cfg(test)]
mod tests {
    use super::LetStatement;
    use crate::emit::fragment::Code;

    #[test]
    fn writes_the_binding_and_its_value() {
        let fragments = LetStatement::new("mut total: i64", Code::atom("0")).fragments();
        assert_eq!(Code::text_of(&fragments), "let mut total: i64 = 0;");
    }

    #[test]
    fn declares_a_binding_without_a_value() {
        let fragments = LetStatement::declared("owner").fragments();
        assert_eq!(Code::text_of(&fragments), "let owner;");
    }
}
