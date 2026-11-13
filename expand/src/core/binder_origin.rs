#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BinderOrigin {
    Written,
    Introduced,
    NamedByCompiler,
}

impl BinderOrigin {
    pub fn is_introduced(self) -> bool {
        self == Self::Introduced
    }

    pub fn is_named_by_compiler(self) -> bool {
        self == Self::NamedByCompiler
    }
}
