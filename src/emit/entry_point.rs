use crate::core::BindingId;

pub struct EntryPoint {
    function: BindingId,
    body_name: Option<String>,
}

impl EntryPoint {
    pub fn new(function: BindingId) -> Self {
        Self {
            function,
            body_name: None,
        }
    }

    pub fn function(&self) -> BindingId {
        self.function
    }

    pub fn is_result_main(&self, binding: BindingId) -> bool {
        binding == self.function && self.body_name.is_some()
    }

    pub fn body_name(&self) -> Option<&str> {
        self.body_name.as_deref()
    }

    pub fn name_body(&mut self, body_name: Option<String>) {
        self.body_name = body_name;
    }
}
