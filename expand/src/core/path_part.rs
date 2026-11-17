use super::ModulePath;

pub(super) struct PathPart;

impl PathPart {
    const NOT_NAMES: [&'static str; 3] = [".", "..", ""];

    pub(super) fn all_names(path: &str) -> bool {
        path.split(ModulePath::SEPARATOR)
            .all(|part| !Self::NOT_NAMES.contains(&part))
    }
}
