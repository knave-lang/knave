mod binding_table;
mod saved_names;
mod scope_chain;
mod scope_kind;
mod scope_set;
mod scopes;

pub use binding_table::{BindingTable, Phase, Resolution};
pub use saved_names::{NameTables, SavedNames};
pub use scope_chain::{ChainPrefix, ScopeChain};
pub(crate) use scope_kind::ScopeKind;
pub use scope_set::{Scope, ScopeSet};
pub(crate) use scopes::Scopes;
