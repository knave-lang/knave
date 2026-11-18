use super::real_path::RealPath;
use crate::core::BuiltinModule;

pub enum ModuleSource {
    Builtin(BuiltinModule),
    Text {
        name: String,
        text: String,
        file: Option<RealPath>,
    },
}
