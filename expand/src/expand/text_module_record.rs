use crate::core::ModulePath;
use crate::diagnostic::Diagnostic;

pub(super) enum TextModuleRecord {
    New,
    Separate,
    BindsTo {
        kept: ModulePath,
        diagnostic: Option<Diagnostic>,
    },
}
