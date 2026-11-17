use std::fmt;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ModulePath(Rc<str>);

impl ModulePath {
    pub const SEPARATOR: &str = "/";

    pub fn new(path: impl Into<Rc<str>>) -> Self {
        Self(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ModulePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
