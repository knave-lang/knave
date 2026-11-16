#[derive(Clone, Copy)]
pub struct OneLineMeasure {
    allowed: bool,
    declared_prefix: usize,
}

impl OneLineMeasure {
    pub fn allowed() -> Self {
        Self {
            allowed: true,
            declared_prefix: 0,
        }
    }

    pub fn refused() -> Self {
        Self {
            allowed: false,
            ..Self::allowed()
        }
    }

    pub fn declared_by(self, declared_prefix: usize) -> Self {
        Self {
            declared_prefix,
            ..self
        }
    }

    pub fn is_allowed(self) -> bool {
        self.allowed
    }

    pub fn declared_length(self, written_length: usize) -> usize {
        written_length + self.declared_prefix
    }
}
