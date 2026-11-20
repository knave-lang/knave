use crate::core::{Piece, PieceLiterals, PrintFormat};
use crate::types::{RustTypeTable, Type, TypedArgument};

pub struct PrintFormats<'t> {
    rust_types: &'t RustTypeTable,
    arguments: &'t [TypedArgument],
}

impl<'t> PrintFormats<'t> {
    pub fn new(rust_types: &'t RustTypeTable, arguments: &'t [TypedArgument]) -> Self {
        Self {
            rust_types,
            arguments,
        }
    }

    fn of(&self, ty: &Type) -> PrintFormat {
        let mut printed = ty;
        while let Type::Ref(_, inner) | Type::Mut(_, inner) = printed {
            printed = inner;
        }
        match printed {
            Type::Named(binding, _) => self
                .rust_types
                .get(*binding)
                .map_or(PrintFormat::Display, |rust_type| rust_type.format),
            _ => PrintFormat::Display,
        }
    }
}

impl PieceLiterals for PrintFormats<'_> {
    fn literal(&self, piece: &Piece) -> Option<String> {
        let Piece::FormatPlaceholder { index } = piece else {
            return None;
        };
        let argument = self.arguments.get(*index)?;
        Some(self.of(&argument.value.ty).placeholder().to_string())
    }
}
