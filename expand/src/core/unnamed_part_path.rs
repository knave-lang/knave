use std::fmt;
use std::rc::Rc;

use super::ModulePath;
use super::path_part::PathPart;

#[derive(Debug, PartialEq)]
pub struct UnnamedPartPath(Rc<str>);

impl UnnamedPartPath {
    pub fn check(written: &str) -> Result<ModulePath, Self> {
        if PathPart::all_names(written) {
            return Ok(ModulePath::new(written));
        }
        Err(Self(written.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UnnamedPartPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::UnnamedPartPath;
    use crate::core::ModulePath;

    #[test]
    fn a_single_dot_part_is_not_a_name() {
        assert!(UnnamedPartPath::check("./shapes").is_err());
    }

    #[test]
    fn a_double_dot_part_is_not_a_name() {
        assert!(UnnamedPartPath::check("text/../shapes").is_err());
    }

    #[test]
    fn a_doubled_separator_leaves_an_empty_part() {
        assert!(UnnamedPartPath::check("text//shapes").is_err());
    }

    #[test]
    fn an_absolute_path_starts_with_an_empty_part() {
        assert!(UnnamedPartPath::check("/text/shapes").is_err());
    }

    #[test]
    fn a_trailing_separator_leaves_an_empty_part() {
        assert!(UnnamedPartPath::check("text/shapes/").is_err());
    }

    #[test]
    fn a_path_whose_names_hold_dots_is_a_module_path() {
        let written = UnnamedPartPath::check(".shapes/a..b/shapes.");
        assert_eq!(written, Ok(ModulePath::new(".shapes/a..b/shapes.")));
    }
}
