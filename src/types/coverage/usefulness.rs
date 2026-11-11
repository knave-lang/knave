use super::constructor::Constructor;
use super::constructor_catalog::ConstructorCatalog;
use super::constructor_tag::ConstructorTag;
use super::pattern_matrix::PatternMatrix;
use super::query::Query;

pub(super) struct Usefulness<'c, 'a> {
    catalog: &'c ConstructorCatalog<'a>,
}

impl<'c, 'a> Usefulness<'c, 'a> {
    pub(super) fn new(catalog: &'c ConstructorCatalog<'a>) -> Self {
        Self { catalog }
    }

    pub(super) fn is_useful(&self, matrix: &PatternMatrix, query: Query) -> bool {
        if query.is_empty() {
            return matrix.is_empty();
        }
        if matrix.is_empty() {
            return true;
        }
        if matrix.has_wild_row() {
            return false;
        }
        let head = query.first_pattern();
        if head.is_wild() {
            return self.is_useful_wild(matrix, query);
        }
        match head.head_tag() {
            Some(tag) => self.is_useful_tagged(matrix, query, tag),
            None => self.is_useful_literal(matrix, query),
        }
    }

    fn is_useful_wild(&self, matrix: &PatternMatrix, query: Query) -> bool {
        match self.catalog.constructors(query.first_type()) {
            Some(constructors) if matrix.heads_cover(&constructors) => constructors
                .iter()
                .any(|constructor| self.is_useful_constructor(matrix, query, constructor)),
            _ => self.is_useful(&matrix.default_rows(), query.rest()),
        }
    }

    fn is_useful_constructor(
        &self,
        matrix: &PatternMatrix,
        query: Query,
        constructor: &Constructor,
    ) -> bool {
        let expanded = query.expanded(constructor, &constructor.wild_arguments());
        self.is_useful_specialized(matrix, constructor, expanded.query())
    }

    fn is_useful_tagged(&self, matrix: &PatternMatrix, query: Query, tag: ConstructorTag) -> bool {
        let constructor = self
            .catalog
            .constructors(query.first_type())
            .and_then(|constructors| {
                constructors
                    .into_iter()
                    .find(|constructor| constructor.tag() == tag)
            });
        match constructor {
            Some(constructor) => {
                let expanded = query.expanded(&constructor, query.first_pattern().arguments());
                self.is_useful_specialized(matrix, &constructor, expanded.query())
            }
            None => false,
        }
    }

    fn is_useful_specialized(
        &self,
        matrix: &PatternMatrix,
        constructor: &Constructor,
        query: Query,
    ) -> bool {
        self.is_useful(&matrix.specialize(constructor), query)
    }

    fn is_useful_literal(&self, matrix: &PatternMatrix, query: Query) -> bool {
        let specialized = matrix.specialize_literal(query.first_pattern());
        self.is_useful(&specialized, query.rest())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::Usefulness;
    use crate::core::{BindingId, Literal, PrimitiveType};
    use crate::source::{SourceId, Span};
    use crate::types::Type;
    use crate::types::coverage::constructor_catalog::ConstructorCatalog;
    use crate::types::coverage::constructor_tag::ConstructorTag;
    use crate::types::coverage::cover_pattern::CoverPattern;
    use crate::types::coverage::pattern_matrix::PatternMatrix;
    use crate::types::coverage::query::Query;
    use crate::types::enums::{
        EnumDefinition, EnumTable, VariantDefinition, VariantPayload, VariantPayloads,
    };
    use crate::types::peeling::ReferencePeeling;
    use crate::types::structs::{StructDefinition, StructTable};
    use crate::types::unifier::Unifier;

    struct SearchFixture {
        unifier: Unifier,
        structs: StructTable,
        enums: EnumTable,
        inferred_boxes: BTreeSet<(BindingId, usize, usize)>,
    }

    impl SearchFixture {
        const SIDE: u32 = 0;
        const TWIN: u32 = 1;
        const CHAIN: u32 = 2;

        fn new() -> Self {
            let mut enums = EnumTable::default();
            enums.add(
                Self::binding(Self::SIDE),
                Self::enumeration(vec![VariantPayload::None, VariantPayload::None]),
            );
            enums.add(
                Self::binding(Self::CHAIN),
                Self::enumeration(vec![
                    VariantPayload::Positional(vec![Self::named(Self::CHAIN), Self::boolean()]),
                    VariantPayload::None,
                ]),
            );
            let mut structs = StructTable::default();
            structs.add(
                Self::binding(Self::TWIN),
                Self::structure(vec![
                    Self::named(Self::SIDE),
                    Self::named(Self::SIDE),
                    Self::boolean(),
                ]),
            );
            Self {
                unifier: Unifier::default(),
                structs,
                enums,
                inferred_boxes: BTreeSet::new(),
            }
        }

        fn binding(index: u32) -> BindingId {
            BindingId::from_index(index)
        }

        fn named(index: u32) -> Type {
            Type::Named(Self::binding(index), Vec::new())
        }

        fn boolean() -> Type {
            Type::Primitive(PrimitiveType::Bool)
        }

        fn span() -> Span {
            Span::new(SourceId::new(0), 0..0)
        }

        fn enumeration(payloads: Vec<VariantPayload>) -> EnumDefinition {
            EnumDefinition {
                lifetimes: Vec::new(),
                params: Vec::new(),
                variants: payloads
                    .into_iter()
                    .enumerate()
                    .map(|(index, payload)| VariantDefinition {
                        name: format!("variant{index}").into(),
                        payload,
                        payload_spans: Vec::new(),
                        function_spans: Vec::new(),
                        span: Self::span(),
                    })
                    .collect(),
                rust: None,
                definer: None,
                derives: Vec::new(),
                derived_traits: Vec::new(),
                blocked: Vec::new(),
                written_derive: None,
            }
        }

        fn structure(fields: Vec<Type>) -> StructDefinition {
            StructDefinition {
                lifetimes: Vec::new(),
                params: Vec::new(),
                fields: fields
                    .into_iter()
                    .enumerate()
                    .map(|(index, field)| (format!("field{index}").into(), field))
                    .collect(),
                field_spans: Vec::new(),
                function_spans: Vec::new(),
                derives: Vec::new(),
                derived_traits: Vec::new(),
                blocked: Vec::new(),
                written_derive: None,
            }
        }

        fn catalog(&self) -> ConstructorCatalog<'_> {
            ConstructorCatalog::new(
                ReferencePeeling::new(&self.unifier),
                &self.structs,
                VariantPayloads::new(&self.enums, None, &self.inferred_boxes),
            )
        }

        fn is_useful(&self, arms: Vec<CoverPattern>, query: CoverPattern, scrutinee: u32) -> bool {
            let catalog = self.catalog();
            let matrix: PatternMatrix = arms.into_iter().collect();
            let types = [Self::named(scrutinee)];
            Usefulness::new(&catalog).is_useful(&matrix, Query::new(&[query], &types))
        }

        fn side(index: usize) -> CoverPattern {
            CoverPattern::constructed(Self::variant(Self::SIDE, index), Vec::new())
        }

        fn twin(first: CoverPattern, second: CoverPattern, flag: CoverPattern) -> CoverPattern {
            let tag = ConstructorTag::Struct(Self::binding(Self::TWIN));
            CoverPattern::constructed(tag, vec![first, second, flag])
        }

        fn link(next: CoverPattern, flag: CoverPattern) -> CoverPattern {
            CoverPattern::constructed(Self::variant(Self::CHAIN, 0), vec![next, flag])
        }

        fn end() -> CoverPattern {
            CoverPattern::constructed(Self::variant(Self::CHAIN, 1), Vec::new())
        }

        fn variant(owner: u32, index: usize) -> ConstructorTag {
            ConstructorTag::Variant {
                owner: Self::binding(owner),
                index,
            }
        }

        fn flag(value: bool) -> CoverPattern {
            CoverPattern::from(Literal::Bool(value))
        }

        fn twin_arms() -> Vec<CoverPattern> {
            let wild = CoverPattern::wild;
            vec![
                Self::twin(Self::side(0), wild(), wild()),
                Self::twin(Self::side(1), Self::side(0), wild()),
                Self::twin(Self::side(1), wild(), Self::flag(false)),
                Self::twin(Self::side(1), Self::side(1), Self::flag(true)),
            ]
        }

        fn chain_arms() -> Vec<CoverPattern> {
            let wild = CoverPattern::wild;
            vec![
                Self::end(),
                Self::link(Self::end(), wild()),
                Self::link(Self::link(wild(), wild()), Self::flag(true)),
                Self::link(wild(), Self::flag(false)),
            ]
        }
    }

    #[test]
    fn columns_of_one_type_are_searched_apart() {
        let fixture = SearchFixture::new();
        let exhaustive = fixture.is_useful(
            SearchFixture::twin_arms(),
            CoverPattern::wild(),
            SearchFixture::TWIN,
        );
        assert!(!exhaustive);
    }

    #[test]
    fn a_nested_recursive_enum_keeps_its_wildcard_rows() {
        let fixture = SearchFixture::new();
        let exhaustive = fixture.is_useful(
            SearchFixture::chain_arms(),
            CoverPattern::wild(),
            SearchFixture::CHAIN,
        );
        assert!(!exhaustive);
    }

    #[test]
    fn a_missing_case_of_a_recursive_enum_is_useful() {
        let fixture = SearchFixture::new();
        let mut arms = SearchFixture::chain_arms();
        arms.remove(2);
        assert!(fixture.is_useful(arms, CoverPattern::wild(), SearchFixture::CHAIN));
    }

    #[test]
    fn a_missing_case_beside_a_column_of_the_same_type_is_useful() {
        let fixture = SearchFixture::new();
        let mut arms = SearchFixture::twin_arms();
        arms.remove(3);
        assert!(fixture.is_useful(arms, CoverPattern::wild(), SearchFixture::TWIN));
    }

    #[test]
    fn an_arm_that_earlier_arms_cover_together_is_not_useful() {
        let fixture = SearchFixture::new();
        let wild = CoverPattern::wild;
        let covered =
            SearchFixture::twin(SearchFixture::side(1), wild(), SearchFixture::flag(true));
        assert!(!fixture.is_useful(SearchFixture::twin_arms(), covered, SearchFixture::TWIN));
    }

    #[test]
    fn an_arm_with_a_case_no_earlier_arm_covers_is_useful() {
        let fixture = SearchFixture::new();
        let wild = CoverPattern::wild;
        let arms = vec![SearchFixture::twin(SearchFixture::side(0), wild(), wild())];
        let new_case = SearchFixture::twin(SearchFixture::side(1), wild(), wild());
        assert!(fixture.is_useful(arms, new_case, SearchFixture::TWIN));
    }
}
