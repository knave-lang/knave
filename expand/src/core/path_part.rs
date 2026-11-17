use super::ModulePath;

pub(super) enum PathPart {
    Current,
    Parent,
    Empty,
    Named,
}

impl PathPart {
    const CURRENT: &'static str = ".";
    const PARENT: &'static str = "..";
    const EMPTY: &'static str = "";

    pub(super) fn parts(path: &str) -> impl Iterator<Item = PathPart> {
        path.split(ModulePath::SEPARATOR).map(Self::parse)
    }

    fn parse(part: &str) -> Self {
        match part {
            Self::CURRENT => Self::Current,
            Self::PARENT => Self::Parent,
            Self::EMPTY => Self::Empty,
            _ => Self::Named,
        }
    }

    pub(super) fn is_name(&self) -> bool {
        matches!(self, Self::Named)
    }
}
