use std::path::PathBuf;

use crate::core::ModulePath;

pub enum DottedPathTarget {
    Reachable(ModulePath),
    Unreachable(PathBuf),
    Absent,
}
