use crate::core::ModulePath;
use crate::diagnostic::Diagnostic;

pub(super) enum TextModuleRecord {
    New,
    Separate,
    BindsTo {
        loaded: ModulePath,
        diagnostic: Option<Diagnostic>,
    },
}
