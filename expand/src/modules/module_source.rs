use crate::core::BuiltinModule;

pub enum ModuleSource {
    Builtin(BuiltinModule),
    Text { name: String, text: String },
}
