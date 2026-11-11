use std::collections::BTreeSet;

use crate::core::{BindingId, Literal, PrimitiveType};
use crate::source::{SourceId, Span};
use crate::types::Type;
use crate::types::coverage::constructor_catalog::ConstructorCatalog;
use crate::types::coverage::constructor_tag::ConstructorTag;
use crate::types::coverage::cover_pattern::CoverPattern;
use crate::types::coverage::missing_case_search::MissingCaseSearch;
use crate::types::coverage::pattern_matrix::PatternMatrix;
use crate::types::coverage::query::Query;
use crate::types::coverage::usefulness::Usefulness;
use crate::types::coverage::witness::Witness;
use crate::types::enums::{
    EnumDefinition, EnumTable, VariantDefinition, VariantPayload, VariantPayloads,
};
use crate::types::peeling::ReferencePeeling;
use crate::types::structs::{StructDefinition, StructTable};
use crate::types::unifier::Unifier;

pub(super) struct SearchFixture {
    unifier: Unifier,
    structs: StructTable,
    enums: EnumTable,
    inferred_boxes: BTreeSet<(BindingId, usize, usize)>,
}

impl SearchFixture {
    const SIDE: u32 = 0;
    const TWIN: u32 = 1;
    const CHAIN: u32 = 2;

    pub(super) fn new() -> Self {
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

    pub(super) fn twin_type() -> Type {
        Self::named(Self::TWIN)
    }

    pub(super) fn chain_type() -> Type {
        Self::named(Self::CHAIN)
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

    pub(super) fn is_useful(
        &self,
        arms: Vec<CoverPattern>,
        query: CoverPattern,
        scrutinee: Type,
    ) -> bool {
        let catalog = self.catalog();
        let matrix: PatternMatrix = arms.into_iter().collect();
        let types = [scrutinee];
        Usefulness::new(&catalog).is_useful(&matrix, Query::new(&[query], &types))
    }

    pub(super) fn side(index: usize) -> CoverPattern {
        CoverPattern::constructed(Self::variant(Self::SIDE, index), Vec::new())
    }

    pub(super) fn twin(
        first: CoverPattern,
        second: CoverPattern,
        flag: CoverPattern,
    ) -> CoverPattern {
        let tag = ConstructorTag::Struct(Self::binding(Self::TWIN));
        CoverPattern::constructed(tag, vec![first, second, flag])
    }

    pub(super) fn link(next: CoverPattern, flag: CoverPattern) -> CoverPattern {
        CoverPattern::constructed(Self::variant(Self::CHAIN, 0), vec![next, flag])
    }

    pub(super) fn end() -> CoverPattern {
        CoverPattern::constructed(Self::variant(Self::CHAIN, 1), Vec::new())
    }

    fn variant(owner: u32, index: usize) -> ConstructorTag {
        ConstructorTag::Variant {
            owner: Self::binding(owner),
            index,
        }
    }

    pub(super) fn flag(value: bool) -> CoverPattern {
        CoverPattern::from(Literal::Bool(value))
    }

    pub(super) fn twin_arms() -> Vec<CoverPattern> {
        let wild = CoverPattern::wild;
        vec![
            Self::twin(Self::side(0), wild(), wild()),
            Self::twin(Self::side(1), Self::side(0), wild()),
            Self::twin(Self::side(1), wild(), Self::flag(false)),
            Self::twin(Self::side(1), Self::side(1), Self::flag(true)),
        ]
    }

    pub(super) fn chain_arms() -> Vec<CoverPattern> {
        let wild = CoverPattern::wild;
        vec![
            Self::end(),
            Self::link(Self::end(), wild()),
            Self::link(Self::link(wild(), wild()), Self::flag(true)),
            Self::link(wild(), Self::flag(false)),
        ]
    }

    pub(super) fn missing_cases(
        &self,
        arms: Vec<CoverPattern>,
        scrutinee: Type,
    ) -> Vec<Vec<Witness>> {
        let catalog = self.catalog();
        let matrix: PatternMatrix = arms.into_iter().collect();
        MissingCaseSearch::new(&catalog).witnesses(&matrix, &[scrutinee])
    }
}
