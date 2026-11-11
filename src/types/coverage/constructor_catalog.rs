use super::constructor::Constructor;
use super::constructor_tag::ConstructorTag;
use super::pattern_matrix::PatternMatrix;
use crate::core::{BindingId, PrimitiveType};
use crate::types::Type;
use crate::types::peeling::ReferencePeeling;
use crate::types::structs::StructTable;
use crate::types::variant_payloads::VariantPayloads;

pub(in crate::types) struct ConstructorCatalog<'a> {
    peeling: ReferencePeeling<'a>,
    structs: &'a StructTable,
    payloads: VariantPayloads<'a>,
}

impl<'a> ConstructorCatalog<'a> {
    pub(in crate::types) fn new(
        peeling: ReferencePeeling<'a>,
        structs: &'a StructTable,
        payloads: VariantPayloads<'a>,
    ) -> Self {
        Self {
            peeling,
            structs,
            payloads,
        }
    }

    pub(super) fn constructors_named_by(
        &self,
        ty: &Type,
        matrix: &PatternMatrix,
    ) -> Option<Vec<Constructor>> {
        self.constructors(ty).filter(|constructors| {
            constructors
                .iter()
                .all(|constructor| matrix.heads_name(constructor.tag()))
        })
    }

    pub(super) fn constructors(&self, ty: &Type) -> Option<Vec<Constructor>> {
        match self.peeled(ty) {
            Type::Primitive(PrimitiveType::Bool) => Some(Self::booleans()),
            Type::Named(owner, arguments) => self.named_constructors(owner, &arguments),
            _ => None,
        }
    }

    pub(super) fn peeled(&self, ty: &Type) -> Type {
        self.peeling.peel(ty).0
    }

    fn booleans() -> Vec<Constructor> {
        [false, true]
            .into_iter()
            .map(|value| Constructor::new(ConstructorTag::Bool(value), Vec::new()))
            .collect()
    }

    fn named_constructors(&self, owner: BindingId, arguments: &[Type]) -> Option<Vec<Constructor>> {
        match self.payloads.variant_count(owner) {
            Some(count) => Some(self.variant_constructors(owner, count, arguments)),
            None => self
                .struct_constructor(owner, arguments)
                .map(|constructor| vec![constructor]),
        }
    }

    fn variant_constructors(
        &self,
        owner: BindingId,
        count: usize,
        arguments: &[Type],
    ) -> Vec<Constructor> {
        (0..count)
            .map(|index| {
                Constructor::new(
                    ConstructorTag::Variant { owner, index },
                    self.payloads.visible(owner, index, arguments),
                )
            })
            .collect()
    }

    fn struct_constructor(&self, owner: BindingId, arguments: &[Type]) -> Option<Constructor> {
        let definition = self.structs.get(owner)?;
        let fields = definition
            .instantiated_fields(arguments)
            .into_iter()
            .map(|(_, field_type)| field_type)
            .collect();
        Some(Constructor::new(ConstructorTag::Struct(owner), fields))
    }
}

#[cfg(test)]
mod tests {
    use crate::types::coverage::cover_pattern::CoverPattern;
    use crate::types::coverage::search_fixture::SearchFixture;

    #[test]
    fn a_column_whose_rows_name_every_constructor_is_split_by_all_of_them() {
        let fixture = SearchFixture::new();
        let arms = vec![
            SearchFixture::end(),
            SearchFixture::link(CoverPattern::wild(), CoverPattern::wild()),
        ];
        assert_eq!(
            fixture.constructors_named_by(arms, SearchFixture::chain_type()),
            Some(2)
        );
    }

    #[test]
    fn a_column_whose_rows_miss_a_constructor_is_not_split() {
        let fixture = SearchFixture::new();
        let arms = vec![SearchFixture::end()];
        assert_eq!(
            fixture.constructors_named_by(arms, SearchFixture::chain_type()),
            None
        );
    }
}
