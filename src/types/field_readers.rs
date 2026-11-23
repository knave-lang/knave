use std::rc::Rc;

use super::declarations::Declarations;
use super::field_reader::FieldReader;
use super::inferred_at::InferredAt;
use super::typed::TypedProgram;

pub(super) struct FieldReaders;

impl FieldReaders {
    pub(super) fn may_exist(declarations: &Declarations) -> bool {
        let field_traits = declarations.traits().rust_paths().field_traits();
        !field_traits.bindings().is_empty()
    }

    pub(super) fn of(program: &TypedProgram) -> Vec<FieldReader> {
        if !Self::may_exist(&program.declarations) {
            return Vec::new();
        }
        let field_traits = program.declarations.traits().rust_paths().field_traits();
        let definitions = (program.modules.iter()).flat_map(|module| &module.definitions);
        let params = definitions.flat_map(|definition| {
            (definition.scheme.params.iter()).map(move |param| (definition.binding, param))
        });
        params
            .flat_map(|(binding, param)| {
                param.bounds.iter().filter_map(move |bound| {
                    let field = field_traits.bindings().field_of(bound.binding)?;
                    Some(FieldReader {
                        binding,
                        owner: param.id,
                        read: bound.argument.clone()?,
                        field: Rc::clone(field),
                        trait_binding: bound.binding,
                        inferred_at: InferredAt::of(&bound.origins),
                    })
                })
            })
            .collect()
    }
}
