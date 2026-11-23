use super::PrimitiveNames;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
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
    names: PrimitiveNames,
    integer_range: Option<(i128, i128)>,
}

impl PrimitiveType {
    const ROWS: [PrimitiveRow; 19] = [
        PrimitiveRow {
            primitive: PrimitiveType::Bool,
            names: PrimitiveNames::same("bool"),
            integer_range: None,
        },
        PrimitiveRow {
            primitive: PrimitiveType::Char,
            names: PrimitiveNames::same("char"),
            integer_range: None,
        },
        PrimitiveRow {
            primitive: PrimitiveType::Str,
            names: PrimitiveNames::same("str"),
            integer_range: None,
        },
        PrimitiveRow {
            primitive: PrimitiveType::String,
            names: PrimitiveNames::same("string").emitted_as("String"),
            integer_range: None,
        },
        PrimitiveRow {
            primitive: PrimitiveType::Unit,
            names: PrimitiveNames::same("unit").emitted_as("()"),
            integer_range: None,
        },
        PrimitiveRow {
            primitive: PrimitiveType::I8,
            names: PrimitiveNames::same("i8"),
            integer_range: Some((i8::MIN as i128, i8::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::I16,
            names: PrimitiveNames::same("i16"),
            integer_range: Some((i16::MIN as i128, i16::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::I32,
            names: PrimitiveNames::same("i32"),
            integer_range: Some((i32::MIN as i128, i32::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::I64,
            names: PrimitiveNames::same("i64").shown_as("int"),
            integer_range: Some((i64::MIN as i128, i64::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::I128,
            names: PrimitiveNames::same("i128"),
            integer_range: Some((i128::MIN, i128::MAX)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::Isize,
            names: PrimitiveNames::same("isize"),
            integer_range: Some((i64::MIN as i128, i64::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::U8,
            names: PrimitiveNames::same("u8"),
            integer_range: Some((0, u8::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::U16,
            names: PrimitiveNames::same("u16"),
            integer_range: Some((0, u16::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::U32,
            names: PrimitiveNames::same("u32"),
            integer_range: Some((0, u32::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::U64,
            names: PrimitiveNames::same("u64"),
            integer_range: Some((0, u64::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::U128,
            names: PrimitiveNames::same("u128"),
            integer_range: Some((0, i128::MAX)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::Usize,
            names: PrimitiveNames::same("usize"),
            integer_range: Some((0, u64::MAX as i128)),
        },
        PrimitiveRow {
            primitive: PrimitiveType::F32,
            names: PrimitiveNames::same("f32"),
            integer_range: None,
        },
        PrimitiveRow {
            primitive: PrimitiveType::F64,
            names: PrimitiveNames::same("f64").shown_as("float"),
            integer_range: None,
        },
    ];

    pub fn all() -> impl Iterator<Item = PrimitiveType> {
        Self::ROWS.iter().map(|row| row.primitive)
    }

    pub fn names(self) -> PrimitiveNames {
        self.row().names
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
