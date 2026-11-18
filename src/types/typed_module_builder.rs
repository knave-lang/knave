use rustc_hash::FxHashMap;

use super::typed::{TypedDefinition, TypedModule};
use crate::core::{BindingId, CoreModule, ItemKind};

pub(super) struct TypedModuleBuilder;

impl TypedModuleBuilder {
    pub(super) fn build(
        module: &CoreModule,
        definitions: &mut FxHashMap<BindingId, TypedDefinition>,
    ) -> TypedModule {
        let mut typed = TypedModule {
            id: module.id,
            path: module.path.clone(),
            definitions: Vec::new(),
            structs: Vec::new(),
            enums: Vec::new(),
            exports: module.exports().bindings().collect(),
        };
        for item in &module.items {
            match &item.kind {
                ItemKind::Def(binding, _) => typed.definitions.extend(definitions.remove(binding)),
                ItemKind::Struct(binding, ..) => typed.structs.push(*binding),
                ItemKind::Enum(binding, ..) => typed.enums.push(*binding),
                ItemKind::Signature(..)
                | ItemKind::RustTrait(..)
                | ItemKind::RustType(..)
                | ItemKind::Trait(..)
                | ItemKind::Impl(..)
                | ItemKind::Derive(..)
                | ItemKind::MismatchHelp(..) => {}
            }
        }
        typed
    }
}
