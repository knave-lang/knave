#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PrimitiveType {
    Bool,
    Char,
    Str,
    String,
    Unit,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    F32,
    F64,
}

struct PrimitiveRow {
    primitive: PrimitiveType,
    name: &'static str,
    integer_range: Option<(i128, i128)>,
}

impl PrimitiveType {
    pub const ALIASES: [(&'static str, PrimitiveType); 1] = [("f64", PrimitiveType::F64)];

    const ROWS: [PrimitiveRow; 19] = [
        PrimitiveRow {
            primitive: PrimitiveType::Bool,
            name: "bool",
            integer_range: None,
        },
        PrimitiveRow {
            primitive: PrimitiveType::Char,
            name: "char",
            integer_range: None,
        },
        PrimitiveRow {
            primitive: PrimitiveType::Str,
            name: "str",
            integer_range: None,
        },
        PrimitiveRow {
            primitive: PrimitiveType::String,
            name: "string",
            integer_range: None,
        },
        PrimitiveRow {
            primitive: PrimitiveType::Unit,
            name: "unit",
            integer_range: None,
        },
        PrimitiveRow {
            primitive: PrimitiveType::I8,
            name: "i8",
            integer_range: Some((i8::MIN as i128, i8::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::I16,
            name: "i16",
            integer_range: Some((i16::MIN as i128, i16::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::I32,
            name: "i32",
            integer_range: Some((i32::MIN as i128, i32::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::I64,
            name: "i64",
            integer_range: Some((i64::MIN as i128, i64::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::I128,
            name: "i128",
            integer_range: Some((i128::MIN, i128::MAX)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::Isize,
            name: "isize",
            integer_range: Some((i64::MIN as i128, i64::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::U8,
            name: "u8",
            integer_range: Some((0, u8::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::U16,
            name: "u16",
            integer_range: Some((0, u16::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::U32,
            name: "u32",
            integer_range: Some((0, u32::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::U64,
            name: "u64",
            integer_range: Some((0, u64::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::U128,
            name: "u128",
            integer_range: Some((0, i128::MAX)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::Usize,
            name: "usize",
            integer_range: Some((0, u64::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::F32,
            name: "f32",
            integer_range: None,
        },
        PrimitiveRow {
            primitive: PrimitiveType::F64,
            name: "float",
            integer_range: None,
        },
    ];

    pub fn all() -> impl Iterator<Item = PrimitiveType> {
        Self::ROWS.iter().map(|row| row.primitive)
    }

    pub fn name(self) -> &'static str {
        self.row().name
    }

    pub fn rust_name(self) -> &'static str {
        match self {
            PrimitiveType::String => "String",
            PrimitiveType::Unit => "()",
            PrimitiveType::F64 => "f64",
            other => other.name(),
        }
    }

    pub fn integer_range(self) -> Option<(i128, i128)> {
        self.row().integer_range
    }

    pub fn is_integer(self) -> bool {
        self.integer_range().is_some()
    }

    pub fn is_float(self) -> bool {
        matches!(self, PrimitiveType::F32 | PrimitiveType::F64)
    }

    fn row(self) -> &'static PrimitiveRow {
        Self::ROWS
            .iter()
            .find(|row| row.primitive == self)
            .expect("every primitive type has a row in the primitive table")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::PrimitiveType;

    #[test]
    fn a_float_is_spelled_float_and_emits_f64() {
        assert_eq!(PrimitiveType::F64.name(), "float");
        assert_eq!(PrimitiveType::F64.rust_name(), "f64");
        assert_eq!(PrimitiveType::F32.name(), "f32");
        assert_eq!(PrimitiveType::F32.rust_name(), "f32");
    }

    #[test]
    fn a_string_and_unit_emit_their_rust_names() {
        assert_eq!(PrimitiveType::String.rust_name(), "String");
        assert_eq!(PrimitiveType::Unit.rust_name(), "()");
        assert_eq!(PrimitiveType::I64.rust_name(), "i64");
    }

    #[test]
    fn every_spelling_names_one_primitive() {
        let mut names = BTreeSet::new();
        for primitive in PrimitiveType::all() {
            assert!(
                names.insert(primitive.name()),
                "{} repeats",
                primitive.name()
            );
        }
        for (alias, _) in PrimitiveType::ALIASES {
            assert!(names.insert(alias), "{alias} repeats a canonical name");
        }
    }
}
