use std::path::PathBuf;

use super::unlisted_library::UnlistedLibrary;

use crate::core::ModulePath;

pub enum UnnamedPartTarget<'s> {
    Reachable(ModulePath),
    Unreachable(PathBuf),
    BeyondTaskReach(PathBuf),
    LibraryFile {
        file: PathBuf,
        unlisted: UnlistedLibrary<'s>,
    },
    Absent,
}
