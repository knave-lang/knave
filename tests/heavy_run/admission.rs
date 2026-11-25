#[derive(Debug, PartialEq)]
pub enum Admission {
    Allowed,
    OnOwnerOverride,
    Refused,
}
