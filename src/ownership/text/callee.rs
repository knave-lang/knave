use super::representation::Access;
use super::text_program::TextProgram;
use crate::core::BindingId;
use crate::types::{GenericId, GenericParam, Type, TypedExpr, TypedExprKind, TypedLeaf};

pub struct Callee<'t> {
    declared: &'t Type,
    parameters: Vec<BindingId>,
    function: Option<BindingId>,
    inline: bool,
    generics: &'t [GenericParam],
}

impl<'t> Callee<'t> {
    pub fn of(callee: &'t TypedExpr, program: &TextProgram<'t>) -> Option<Self> {
        match &callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => {
                let definition = program.definition(*binding)?;
                let callee = match &definition.value.kind {
                    TypedExprKind::Fn(lambda) => Self {
                        declared: &definition.scheme.body,
                        parameters: lambda
                            .params
                            .iter()
                            .map(|parameter| parameter.binding)
                            .collect(),
                        function: Some(*binding),
                        inline: false,
                        generics: &[],
                    },
                    _ => Self::template(&definition.scheme.body),
                };
                Some(Self {
                    generics: &definition.scheme.params,
                    ..callee
                })
            }
            TypedExprKind::Leaf(TypedLeaf::Rust(_)) => Some(Self {
                inline: true,
                ..Self::template(&callee.ty)
            }),
            _ => None,
        }
    }

    pub fn signature(&self) -> Option<(&'t [Type], &'t Type)> {
        match self.declared {
            Type::Function(_, parameters, result) => Some((parameters, result)),
            _ => None,
        }
    }

    pub fn parameter(&self, index: usize) -> Option<BindingId> {
        self.parameters.get(index).copied()
    }

    pub fn function(&self) -> Option<BindingId> {
        self.function
    }

    pub fn needs_owned_text(&self, generic: GenericId, program: &TextProgram<'_>) -> bool {
        self.generics
            .iter()
            .filter(|param| param.id == generic)
            .flat_map(|param| &param.bounds)
            .any(|bound| program.owns_text_for(bound))
    }

    pub fn is_inline(&self) -> bool {
        self.inline
    }

    pub fn whole_generic(declared: &Type) -> Option<(GenericId, Access)> {
        match declared {
            Type::Generic(generic) => Some((*generic, Access::Direct)),
            Type::Ref(_, inner) => {
                Self::whole_generic(inner).map(|(generic, _)| (generic, Access::Referenced))
            }
            _ => None,
        }
    }

    fn template(declared: &'t Type) -> Self {
        Self {
            declared,
            parameters: Vec::new(),
            function: None,
            inline: false,
            generics: &[],
        }
    }
}
