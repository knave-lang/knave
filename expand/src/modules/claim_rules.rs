use crate::core::ModulePath;

pub(super) struct ClaimRules {
    pub(super) program: fn(&ModulePath) -> bool,
    pub(super) library: fn(&ModulePath) -> bool,
}
