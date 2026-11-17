use super::ModulePath;

pub(super) struct PathPart;

impl PathPart {
    const PARENT: &'static str = "..";
    const PLACE_KEEPING: [&'static str; 2] = [".", ""];

    pub(super) fn all_names(path: &str) -> bool {
        path.split(ModulePath::SEPARATOR).all(Self::is_name)
    }

    pub(super) fn names_in_place(path: &str) -> Option<Vec<&str>> {
        let leaves_place = path.starts_with(ModulePath::SEPARATOR)
            || path
                .split(ModulePath::SEPARATOR)
                .any(|part| part == Self::PARENT);
        if leaves_place {
            return None;
        }
        Some(
            path.split(ModulePath::SEPARATOR)
                .filter(|part| Self::is_name(part))
                .collect(),
        )
    }

    fn is_name(part: &str) -> bool {
        part != Self::PARENT && !Self::PLACE_KEEPING.contains(&part)
    }
}
