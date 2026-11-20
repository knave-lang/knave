use super::access::Access;
use super::conversion::Conversion;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Piece {
    Literal(String),
    Placeholder {
        index: usize,
        access: Access,
        conversion: Option<Conversion>,
    },
    TypePlaceholder {
        name: String,
    },
    BinderPlaceholder {
        name: String,
    },
    FormatPlaceholder {
        index: usize,
    },
}

impl Piece {
    pub fn conversions_in(pieces: &[Self]) -> impl Iterator<Item = (usize, Conversion)> + '_ {
        pieces.iter().filter_map(Self::conversion)
    }

    pub fn conversion(&self) -> Option<(usize, Conversion)> {
        match self {
            Self::Placeholder {
                index,
                conversion: Some(conversion),
                ..
            } => Some((*index, *conversion)),
            _ => None,
        }
    }

    pub fn unconverted(&self) -> Self {
        match self {
            Self::Placeholder { index, access, .. } => Self::Placeholder {
                index: *index,
                access: *access,
                conversion: None,
            },
            other => other.clone(),
        }
    }

    pub(super) fn access_for(&self, argument: usize) -> Option<Access> {
        match self {
            Self::Placeholder { access, .. } if self.is_placeholder_of(argument) => Some(*access),
            _ => None,
        }
    }

    pub fn is_placeholder_of(&self, argument: usize) -> bool {
        matches!(self, Self::Placeholder { index, .. } if *index == argument)
    }
}
