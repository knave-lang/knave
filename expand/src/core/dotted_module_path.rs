use std::fmt;
use std::rc::Rc;

use super::ModulePath;
use super::path_part::PathPart;

#[derive(Debug, PartialEq)]
pub struct DottedModulePath(Rc<str>);

impl DottedModulePath {
    pub fn check(written: &str) -> Result<ModulePath, Self> {
        if PathPart::parts(written).any(|part| part.is_dot()) {
            return Err(Self(written.into()));
        }
        Ok(ModulePath::new(written))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn without_dot_parts(&self) -> Option<ModulePath> {
        let mut names = Vec::new();
        for part in PathPart::parts(&self.0) {
            match part {
                PathPart::Current => {}
                PathPart::Parent => {
                    names.pop()?;
                }
                PathPart::Named(name) => names.push(name),
            }
        }
        (!names.is_empty()).then(|| ModulePath::new(names.join(ModulePath::SEPARATOR)))
    }
}

impl fmt::Display for DottedModulePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::DottedModulePath;
    use crate::core::ModulePath;

    #[test]
    fn a_written_path_with_a_single_dot_part_is_dotted() {
        assert!(DottedModulePath::check("./shapes").is_err());
    }

    #[test]
    fn a_written_path_with_a_double_dot_part_is_dotted() {
        assert!(DottedModulePath::check("text/../shapes").is_err());
    }

    #[test]
    fn a_written_path_whose_names_hold_dots_is_a_module_path() {
        let written = DottedModulePath::check(".shapes/a..b/shapes.");
        assert_eq!(written, Ok(ModulePath::new(".shapes/a..b/shapes.")));
    }

    #[test]
    fn dropping_dot_parts_drops_each_single_dot_and_the_name_before_each_double_dot() {
        let dotted = DottedModulePath::check("./text/./pad/../shapes").expect_err("dotted");
        assert_eq!(
            dotted.without_dot_parts(),
            Some(ModulePath::new("text/shapes"))
        );
    }

    #[test]
    fn a_path_that_leaves_its_root_keeps_no_path_without_dot_parts() {
        let dotted = DottedModulePath::check("../shapes").expect_err("dotted");
        assert_eq!(dotted.without_dot_parts(), None);
    }

    #[test]
    fn a_path_that_names_its_root_keeps_no_path_without_dot_parts() {
        let dotted = DottedModulePath::check("shapes/..").expect_err("dotted");
        assert_eq!(dotted.without_dot_parts(), None);
    }
}
