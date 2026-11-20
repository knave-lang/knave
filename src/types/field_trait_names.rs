use knave_expand::mangle::NameMangler;

use super::field_reader::FieldReader;
use super::typed::TypedProgram;
use crate::core::CopyTrait;

pub(super) struct FieldTraitNames<'p> {
    program: &'p TypedProgram,
}

impl<'p> FieldTraitNames<'p> {
    pub(super) fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub(super) fn are_free(&self, reader: &FieldReader) -> bool {
        self.trait_name_is_free(reader) && !self.is_trait_method(&NameMangler::value(&reader.field))
    }

    pub(super) fn copy_takes_getter(&self, reader: &FieldReader) -> bool {
        CopyTrait::BROUGHT_METHODS.contains(&&*NameMangler::value(&reader.field))
    }

    fn trait_name_is_free(&self, reader: &FieldReader) -> bool {
        let trait_name = NameMangler::type_name(&format!("has-{}", reader.field));
        let types = (self.program.modules.iter())
            .flat_map(|module| module.structs.iter().chain(&module.enums))
            .map(|binding| NameMangler::type_name(self.program.bindings.get(*binding).name()));
        let traits = (self.program.declarations.traits().definitions.iter())
            .map(|definition| definition.rust_name.to_string());
        !types.chain(traits).any(|taken| taken == trait_name)
    }

    fn is_trait_method(&self, getter: &str) -> bool {
        let mut methods = self.program.declarations.traits().methods.keys();
        methods
            .any(|method| NameMangler::value(self.program.bindings.get(*method).name()) == getter)
    }
}
