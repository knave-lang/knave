use crate::core::Namespace;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SavedNamespace(u32);

impl SavedNamespace {
    const VALUE: u32 = 0;
    const TYPE: u32 = 1;

    pub(super) fn of(namespace: Namespace) -> Self {
        match namespace {
            Namespace::Value => Self(Self::VALUE),
            Namespace::Type => Self(Self::TYPE),
        }
    }

    pub(super) fn read(code: u32) -> Self {
        Self(code)
    }

    pub(super) fn code(self) -> u32 {
        self.0
    }

    pub(super) fn namespace(self) -> Namespace {
        match self.0 {
            Self::VALUE => Namespace::Value,
            _ => Namespace::Type,
        }
    }
}
