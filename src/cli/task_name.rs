#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskName(String);

impl TaskName {
    pub fn new(name: String) -> Self {
        Self(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
