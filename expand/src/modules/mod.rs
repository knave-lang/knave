mod libraries;
mod module_scopes;
mod registry;
mod search_scope;
mod sources;

pub use libraries::{LibraryFile, LibraryRoot, LibraryTable};
pub use module_scopes::ModuleScopes;
pub use registry::{ModuleRecord, ModuleRegistry};
pub use search_scope::SearchScope;
pub use sources::{
    HiddenSourceModule, LocalModuleClash, ModuleSource, ModuleSources, PublicPrelude,
};
