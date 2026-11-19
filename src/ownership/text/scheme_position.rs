use super::access::Access;
use super::representation::Representation;
use super::type_path::TypePath;
use crate::types::GenericId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchemePosition {
    Generic {
        generic: GenericId,
        outer: TypePath,
        inner: TypePath,
        access: Access,
    },
    Written {
        path: TypePath,
        written: Representation,
    },
}

impl SchemePosition {
    pub fn path(&self) -> TypePath {
        match self {
            Self::Generic { outer, inner, .. } => outer.joined(inner),
            Self::Written { path, .. } => path.clone(),
        }
    }
}
