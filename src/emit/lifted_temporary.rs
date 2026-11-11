use super::fragment::Fragment;
use super::let_statement::LetStatement;

pub struct LiftedTemporary {
    name: String,
}

impl LiftedTemporary {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn statement(&self) -> Vec<Fragment> {
        LetStatement::declared(self.name.as_str()).fragments()
    }
}
