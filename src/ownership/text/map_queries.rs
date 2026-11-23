use super::text_position::TextPositions;
use super::text_types::TextTypes;
use super::type_path::TypePath;
use crate::types::Type;

pub struct MapQueries<'p> {
    types: TextTypes<'p>,
}

impl<'p> MapQueries<'p> {
    pub fn new(types: TextTypes<'p>) -> Self {
        Self { types }
    }

    pub fn of(&self, parameters: &[Type]) -> Vec<usize> {
        let Some(key) = parameters
            .iter()
            .find_map(|parameter| self.key_of(parameter))
        else {
            return Vec::new();
        };
        parameters
            .iter()
            .enumerate()
            .filter(|(_, parameter)| Self::refers_to(parameter, &key))
            .map(|(index, _)| index)
            .collect()
    }

    pub fn pair(&self, parameters: &[Type]) -> Option<(usize, usize)> {
        let (map, key) = parameters
            .iter()
            .enumerate()
            .find_map(|(index, parameter)| Some((index, self.key_of(parameter)?)))?;
        let query = parameters
            .iter()
            .enumerate()
            .position(|(index, parameter)| index != map && Self::refers_to(parameter, &key))?;
        Some((map, query))
    }

    fn key_of(&self, parameter: &Type) -> Option<Vec<TypePath>> {
        let collection = self.types.collection_of(Self::referent(parameter))?;
        collection
            .is_map()
            .then(|| Self::text_paths(collection.key()))
    }

    fn refers_to(parameter: &Type, key: &[TypePath]) -> bool {
        matches!(parameter, Type::Ref(_, query) if Self::text_paths(query) == key)
    }

    fn referent(ty: &Type) -> &Type {
        match ty {
            Type::Ref(_, inner) | Type::Mut(_, inner) => Self::referent(inner),
            other => other,
        }
    }

    fn text_paths(ty: &Type) -> Vec<TypePath> {
        TextPositions::of(ty)
            .into_iter()
            .map(|position| position.path)
            .collect()
    }
}
