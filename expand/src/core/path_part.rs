use super::ModulePath;

pub(super) enum PathPart<'t> {
    Current,
    Parent,
    Named(&'t str),
}

impl<'t> PathPart<'t> {
    const CURRENT: &'static str = ".";
    const PARENT: &'static str = "..";

    pub(super) fn parts(path: &'t str) -> impl Iterator<Item = PathPart<'t>> {
        path.split(ModulePath::SEPARATOR).map(Self::parse)
    }

    fn parse(part: &'t str) -> Self {
        match part {
            Self::CURRENT => Self::Current,
            Self::PARENT => Self::Parent,
            name => Self::Named(name),
        }
    }

    pub(super) fn is_dot(&self) -> bool {
        !matches!(self, Self::Named(_))
    }
}
