pub struct NeededTrait<'a> {
    name: &'a str,
    bound: String,
}

impl<'a> NeededTrait<'a> {
    pub fn new(name: &'a str, bound: String) -> Self {
        Self { name, bound }
    }

    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn bound(&self) -> &str {
        &self.bound
    }
}
