use std::path::PathBuf;

use crate::core::ModulePath;

pub enum UnnamedPartTarget {
    Reachable(ModulePath),
    Unreachable(PathBuf),
    BeyondTaskReach(PathBuf),
    Absent,
}
