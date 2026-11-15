#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RoleProblem {
    Missing(&'static str),
    Repeated(String),
    Unknown(String),
    TwoRoles(String),
    NoMacro(String),
    Malformed,
}
