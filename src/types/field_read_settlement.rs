use rustc_hash::FxHashMap;

use super::declarations::Declarations;
use super::field_owner_problems::FieldOwnerProblems;
use super::field_reader::FieldReader;
use super::field_readers::FieldReaders;
use super::field_traits::FieldTraitPlan;
use super::inferred_bound_problems::InferredBoundProblems;
use super::pinning::InstantiationReport;
use super::reaching_read::ReachingRead;
use super::reaching_reads::ReachingReads;
use super::read_settlements::ReadSettlements;
use super::reader_shape::ReaderShape;
use super::refusal_wording::RefusalWording;
use super::ty::{GenericId, Type};
use super::typed::{TypedExpr, TypedExprKind, TypedProgram, TypedStructExpr};
use super::unborrowed::Unborrowed;
use super::written_refusal::{ReadRefusal, WrittenRefusal};
use crate::core::{BindingId, FieldLabel};
use crate::diagnostic::Diagnostics;

pub struct FieldReadSettlement<'r> {
    report: &'r InstantiationReport,
}

#[derive(Default)]
struct Tally {
    settlements: ReadSettlements,
    errors: Diagnostics,
    plans: FxHashMap<BindingId, FieldTraitPlan>,
}

impl<'r> FieldReadSettlement<'r> {
    pub fn new(report: &'r InstantiationReport) -> Self {
        Self { report }
    }

    pub fn settle(&self, program: &mut TypedProgram) -> Result<(), Diagnostics> {
        let readers = Self::readers(program);
        if readers.is_empty() {
            return Ok(());
        }
        let mut tally = Tally::default();
        for reader in readers {
            self.count(program, reader, &mut tally);
        }
        if !tally.errors.is_empty() {
            return Err(tally.errors);
        }
        tally.settlements.apply_to(program);
        tally.record_plans(&mut program.declarations);
        Ok(())
    }

    fn count(&self, program: &TypedProgram, reader: FieldReader, tally: &mut Tally) {
        let planned = self.plan(program, &reader).and_then(|plan| match plan {
            Some(plan) => tally.kept(program, &reader, plan),
            None => Ok(()),
        });
        if let Err(refusal) = planned {
            tally
                .errors
                .extend(self.refusals(program, &reader, refusal));
        }
    }

    fn readers(program: &TypedProgram) -> Vec<FieldReader> {
        let readers = FieldReaders::of(program).into_iter();
        let reached_or_written =
            |reader: &FieldReader| !reader.inferred_at.is_inferred() || reader.read.is_concrete();
        readers.filter(reached_or_written).collect()
    }

    fn plan(
        &self,
        program: &TypedProgram,
        reader: &FieldReader,
    ) -> Result<Option<FieldTraitPlan>, WrittenRefusal> {
        let reads = ReachingReads::new(self.report, program).of(reader);
        let unreached = matches!(&reads, Ok(found) if found.is_empty());
        if !reader.read.is_concrete() && !unreached {
            return Err(WrittenRefusal::Read(Self::open_refusal(&reads)));
        }
        let reads = reads?;
        let mut plan: Option<FieldTraitPlan> = None;
        for reached in reads
            .into_iter()
            .filter(|reached| reached.read.read == reader.read)
        {
            let implementor = Type::Named(reached.owner, reached.arguments);
            let found = plan.get_or_insert_with(|| FieldTraitPlan {
                read: reached.read.read,
                kind: reached.read.kind,
                implementors: Vec::new(),
            });
            if !found.implementors.contains(&implementor) {
                found.implementors.push(implementor);
            }
        }
        Ok(plan)
    }

    fn refusals(
        &self,
        program: &TypedProgram,
        reader: &FieldReader,
        refusal: WrittenRefusal,
    ) -> Diagnostics {
        let note = self.reads_note(program, reader, refusal);
        let definitions = (program.modules.iter()).flat_map(|module| &module.definitions);
        let refused = definitions.filter(|definition| definition.binding == reader.binding);
        refused
            .map(|definition| {
                let span = Self::first_read(&definition.value, reader.owner)
                    .map_or(definition.span, |label| label.span);
                match (reader.inferred_at.first(), &note) {
                    (Some(read), _) => InferredBoundProblems::new(&reader.field, read).unkept(
                        refusal,
                        Some(&InferredBoundProblems::shown_bound(reader, program)),
                    ),
                    (None, Some(note)) => {
                        FieldOwnerProblems::unkept(refusal, &reader.field, span).with_note(note)
                    }
                    (None, None) => FieldOwnerProblems::unkept(refusal, &reader.field, span),
                }
            })
            .collect()
    }

    fn open_refusal(reads: &Result<Vec<ReachingRead>, WrittenRefusal>) -> ReadRefusal {
        let found = reads.as_ref().map(Vec::as_slice).unwrap_or_default();
        let function = |reached: &ReachingRead| matches!(reached.read.read, Type::Function(..));
        if found.iter().any(function) {
            ReadRefusal::Function
        } else if found.iter().any(|reached| reached.read != found[0].read) {
            ReadRefusal::Differing
        } else {
            ReadRefusal::Open
        }
    }

    fn reads_note(
        &self,
        program: &TypedProgram,
        reader: &FieldReader,
        refusal: WrittenRefusal,
    ) -> Option<String> {
        let reads = ReachingReads::new(self.report, program).of(reader).ok()?;
        let shown = ReachingReads::shown(program, &reads);
        match refusal {
            WrittenRefusal::Read(ReadRefusal::Open) => Some(RefusalWording::agreed_read(
                &reader.field,
                &shown.first()?.1,
            )),
            WrittenRefusal::Read(ReadRefusal::Differing) => {
                Some(RefusalWording::differing_reads(&reader.field, &shown))
            }
            _ => None,
        }
    }

    fn first_read(expr: &TypedExpr, owner: GenericId) -> Option<&FieldLabel> {
        if let TypedExprKind::Struct(TypedStructExpr::Field(value, label)) = &expr.kind
            && Unborrowed::of(&value.ty) == &Type::Generic(owner)
        {
            return Some(label);
        }
        (expr.children().into_iter()).find_map(|child| Self::first_read(child, owner))
    }
}

impl Tally {
    fn kept(
        &mut self,
        program: &TypedProgram,
        reader: &FieldReader,
        plan: FieldTraitPlan,
    ) -> Result<(), WrittenRefusal> {
        let shape = ReaderShape::new(program);
        if let Some(refusal) = shape.refusal(reader, &plan) {
            return Err(refusal);
        }
        if !reader.inferred_at.is_inferred() && shape.copies_owner(&plan) {
            self.settlements.require_copy(reader.owner);
        }
        self.join(reader.trait_binding, plan)
    }

    fn join(
        &mut self,
        trait_binding: BindingId,
        plan: FieldTraitPlan,
    ) -> Result<(), WrittenRefusal> {
        let Some(known) = self.plans.get_mut(&trait_binding) else {
            self.plans.insert(trait_binding, plan);
            return Ok(());
        };
        if known.read != plan.read {
            return Err(WrittenRefusal::Read(ReadRefusal::Otherwise));
        }
        for implementor in plan.implementors {
            if !known.implementors.contains(&implementor) {
                known.implementors.push(implementor);
            }
        }
        Ok(())
    }

    fn record_plans(self, declarations: &mut Declarations) {
        for (binding, plan) in self.plans {
            declarations.settle_field_trait(binding, plan);
        }
    }
}
