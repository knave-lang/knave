use rustc_hash::{FxHashMap, FxHashSet};

use super::field_reader::FieldReader;
use super::graph::DefinitionGraph;
use super::inferred_at::InferredAt;
use super::pinning::InstantiationReport;
use super::reaching_structs::ReachingStructs;
use super::ty::GenericId;
use super::typed::TypedProgram;
use crate::core::BindingId;

type ReaderKey = (BindingId, GenericId);

struct SoleOwner {
    field: BindingId,
    owner: Option<BindingId>,
}

pub(super) struct PinnedOwners<'r> {
    report: &'r InstantiationReport,
    typed: &'r TypedProgram,
    graph: &'r DefinitionGraph<'r>,
}

impl<'r> PinnedOwners<'r> {
    pub(super) fn new(
        report: &'r InstantiationReport,
        typed: &'r TypedProgram,
        graph: &'r DefinitionGraph<'r>,
    ) -> Self {
        Self {
            report,
            typed,
            graph,
        }
    }

    pub(super) fn of(&self, readers: &[FieldReader]) -> FxHashMap<ReaderKey, BindingId> {
        let sole: FxHashMap<ReaderKey, SoleOwner> = (readers.iter())
            .map(|reader| (Self::key(reader), self.sole_owner(reader)))
            .collect();
        (sole.iter())
            .filter_map(|(key, found)| {
                let owner = found.owner?;
                self.group_agrees(key.0, found, &sole)
                    .then_some((*key, owner))
            })
            .collect()
    }

    pub(super) fn key(reader: &FieldReader) -> ReaderKey {
        (reader.binding, reader.owner)
    }

    fn sole_owner(&self, reader: &FieldReader) -> SoleOwner {
        let structs = ReachingStructs::new(self.report).of(reader.binding, reader.owner);
        let owners: FxHashSet<BindingId> = (structs.iter().flatten())
            .map(|(owner, _)| *owner)
            .collect();
        let owner = match Vec::from_iter(owners).as_slice() {
            [sole]
                if structs.is_some()
                    && self.declares(*sole, reader)
                    && self.passes_inferred_owners(reader) =>
            {
                Some(*sole)
            }
            _ => None,
        };
        SoleOwner {
            field: reader.trait_binding,
            owner,
        }
    }

    fn declares(&self, owner: BindingId, reader: &FieldReader) -> bool {
        let structs = &self.typed.declarations.types().structs;
        let declared = structs
            .get(owner)
            .and_then(|found| found.field(&reader.field));
        declared.is_some()
    }

    fn group_agrees(
        &self,
        reader: BindingId,
        pinned: &SoleOwner,
        sole: &FxHashMap<ReaderKey, SoleOwner>,
    ) -> bool {
        let group = (self.graph.components().iter()).find(|group| group.contains(&reader));
        let members: FxHashSet<BindingId> = group.into_iter().flatten().copied().collect();
        let others = (sole.iter()).filter(|((member, _), found)| {
            *member != reader && members.contains(member) && found.field == pinned.field
        });
        others
            .into_iter()
            .all(|(_, found)| found.owner == pinned.owner)
    }

    fn passes_inferred_owners(&self, reader: &FieldReader) -> bool {
        let callers =
            ReachingStructs::new(self.report).generic_callers(reader.binding, reader.owner);
        callers.into_iter().all(|caller| {
            let param = self.typed.generics.get(&caller);
            let bounds = param
                .map(|param| param.bounds.as_slice())
                .unwrap_or_default();
            (bounds.iter()).any(|bound| {
                bound.binding == reader.trait_binding
                    && InferredAt::of(&bound.origins).is_inferred()
            })
        })
    }
}
