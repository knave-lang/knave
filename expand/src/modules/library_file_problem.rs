use std::path::PathBuf;

use super::missing_root_module::MissingRootModule;

pub enum LibraryFileProblem<'t> {
    MissingRootModule(MissingRootModule<'t>),
    OutsideCheckout(PathBuf),
}
