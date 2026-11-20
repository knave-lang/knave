use knave_expand::mangle::NameMangler;

use super::field_getter_name::FieldGetterName;
use super::field_reader::FieldReader;
use super::field_traits::FieldTraits;
use super::typed::TypedProgram;
use crate::core::{CopyTrait, ModulePath};

pub(super) struct FieldTraitNames<'p> {
    program: &'p TypedProgram,
}

impl<'p> FieldTraitNames<'p> {
    pub(super) fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub(super) fn are_free(&self, reader: &FieldReader) -> bool {
        self.trait_path_is_unique(reader)
            && self.module_name_is_free()
            && !self.is_trait_method(&FieldGetterName::of(&reader.field))
    }

    pub(super) fn copy_takes_getter(&self, reader: &FieldReader) -> bool {
        CopyTrait::BROUGHT_METHODS.contains(&&*FieldGetterName::of(&reader.field))
    }

    fn trait_path_is_unique(&self, reader: &FieldReader) -> bool {
        let traits = self.program.declarations.traits();
        let path_of = |binding| traits.table().get(binding).map(|found| &found.path);
        let Some(own) = path_of(reader.trait_binding) else {
            return false;
        };
        let mut others = (traits.rust_paths().field_traits().bindings().iter())
            .map(|(_, binding)| binding)
            .filter(|binding| *binding != reader.trait_binding);
        !others.any(|binding| path_of(binding) == Some(own))
    }

    fn module_name_is_free(&self) -> bool {
        let mut roots = (self.program.modules.iter())
            .filter_map(|module| module.path.as_str().split(ModulePath::SEPARATOR).next());
        !roots.any(|root| NameMangler::value(root) == FieldTraits::MODULE)
    }

    fn is_trait_method(&self, getter: &str) -> bool {
        let mut methods = self.program.declarations.traits().methods.keys();
        methods
            .any(|method| NameMangler::value(self.program.bindings.get(*method).name()) == getter)
    }
}
