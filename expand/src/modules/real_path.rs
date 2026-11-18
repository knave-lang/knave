use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RealPath(PathBuf);

impl RealPath {
    pub(super) fn new(canonical: PathBuf) -> Self {
        Self(canonical)
    }
}
