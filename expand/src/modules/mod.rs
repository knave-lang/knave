mod libraries;
mod registry;
mod search_scope;
mod sources;

pub use libraries::{LibraryFile, LibraryRoot, LibraryTable};
pub use registry::{ModuleRecord, ModuleRegistry};
pub use search_scope::SearchScope;
pub use sources::{
    BuiltinModule, HiddenSourceModule, LocalModuleClash, ModuleSource, ModuleSources,
};
