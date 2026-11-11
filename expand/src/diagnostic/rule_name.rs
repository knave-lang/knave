use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RuleName(Rc<str>);

impl RuleName {
    pub fn new(name: &str) -> Self {
        Self(Rc::from(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
