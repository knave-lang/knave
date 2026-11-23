use rustc_hash::{FxHashMap, FxHashSet};

use super::flow_origin::FlowOrigin;
use super::text_names::TextNames;
use crate::core::{BindingId, ModuleId};
use crate::modules::ModuleSources;
use crate::types::{Type, TypedProgram};

pub struct WrittenBindings {
    values: FxHashSet<BindingId>,
    trait_methods: FxHashMap<BindingId, BindingId>,
    receivers: FxHashSet<BindingId>,
    generated: FxHashSet<BindingId>,
}

impl WrittenBindings {
    pub fn of(program: &TypedProgram) -> Self {
        let values = program
            .modules
            .iter()
            .filter(|module| !ModuleSources::is_prelude(&module.path))
            .flat_map(|module| &module.definitions)
            .filter(|definition| !matches!(definition.value.ty, Type::Function(..)))
            .map(|definition| definition.binding)
            .collect();
        let trait_methods = Self::trait_methods(program);
        let names = TextNames::new(program);
        let generated = program
            .modules
            .iter()
            .flat_map(|module| &module.definitions)
            .map(|definition| definition.binding)
            .filter(|binding| names.is_generated(*binding) && !trait_methods.contains_key(binding))
            .collect();
        Self {
            values,
            trait_methods,
            receivers: Self::receivers(program),
            generated,
        }
    }

    pub fn origin(&self, function: BindingId) -> FlowOrigin {
        if self.generated.contains(&function) {
            FlowOrigin::Generated
        } else {
            FlowOrigin::Written
        }
    }

    pub fn is_value(&self, binding: BindingId) -> bool {
        self.values.contains(&binding)
    }

    pub fn has_receiver(&self, method: BindingId) -> bool {
        self.receivers.contains(&method)
    }

    pub fn trait_method(&self, binding: BindingId) -> Option<BindingId> {
        self.trait_methods.get(&binding).copied()
    }

    fn trait_methods(program: &TypedProgram) -> FxHashMap<BindingId, BindingId> {
        let written = |module: &ModuleId| {
            program
                .modules
                .iter()
                .any(|typed| typed.id == *module && !ModuleSources::is_prelude(&typed.path))
        };
        program
            .declarations
            .traits()
            .definitions()
            .iter()
            .filter(|definition| written(&definition.module))
            .flat_map(|definition| {
                definition.methods.iter().flat_map(|method| {
                    let name = program.bindings.get(method.binding).name();
                    program
                        .trait_method_bodies(definition.binding, name)
                        .into_iter()
                        .chain([method.binding])
                        .map(|body| (body, method.binding))
                })
            })
            .collect()
    }

    fn receivers(program: &TypedProgram) -> FxHashSet<BindingId> {
        let traits = program.declarations.traits();
        let implemented = traits
            .impls()
            .iter()
            .flat_map(|implementation| &implementation.methods)
            .filter(|method| method.receiver.is_some())
            .map(|method| method.binding);
        let defaults = traits
            .definitions()
            .iter()
            .flat_map(|definition| &definition.methods)
            .filter(|method| method.receiver.is_some())
            .filter_map(|method| method.default);
        implemented.chain(defaults).collect()
    }
}
