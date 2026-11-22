use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::copy::CopyRules;
use super::declarations::Declarations;
use super::field_owner_problems::FieldOwnerProblems;
use super::field_reader::FieldReader;
use super::field_traits::FieldTraitPlan;
use super::open_read::OpenRead;
use super::pinning::InstantiationReport;
use super::reaching_structs::ReachingStructs;
use super::read_settlements::ReadSettlements;
use super::reader_shape::ReaderShape;
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
                .extend(Self::refusals(program, &reader, refusal));
        }
    }

    fn readers(program: &TypedProgram) -> Vec<FieldReader> {
        let field_traits = program.declarations.traits().rust_paths().field_traits();
        let definitions = (program.modules.iter()).flat_map(|module| &module.definitions);
        let params = definitions.flat_map(|definition| {
            (definition.scheme.params.iter()).map(move |param| (definition.binding, param))
        });
        params
            .flat_map(|(binding, param)| {
                param.bounds.iter().filter_map(move |bound| {
                    let field = field_traits.bindings().field_of(bound.binding)?;
                    Some(FieldReader {
                        binding,
                        owner: param.id,
                        read: bound.argument.clone()?,
                        field: Rc::clone(field),
                        trait_binding: bound.binding,
                    })
                })
            })
            .collect()
    }

    fn plan(
        &self,
        program: &TypedProgram,
        reader: &FieldReader,
    ) -> Result<Option<FieldTraitPlan>, WrittenRefusal> {
        if !reader.read.is_concrete() {
            return Err(WrittenRefusal::Read(ReadRefusal::Open));
        }
        let structs = ReachingStructs::new(self.report)
            .of(reader.binding, reader.owner)
            .ok_or(WrittenRefusal::Read(ReadRefusal::OpenStruct))?;
        let reads = OpenRead::new(CopyRules::new(&program.declarations, &program.generics));
        let mut plan: Option<FieldTraitPlan> = None;
        for (owner, arguments) in structs {
            if !arguments.iter().all(Type::is_concrete) {
                return Err(WrittenRefusal::Read(ReadRefusal::OpenStruct));
            }
            let definition = program.declarations.types().structs.get(owner);
            let read =
                definition.and_then(|found| reads.instantiated(found, &reader.field, &arguments));
            let Some(read) = read.filter(|read| read.read == reader.read) else {
                continue;
            };
            let implementor = Type::Named(owner, arguments);
            let found = plan.get_or_insert_with(|| FieldTraitPlan {
                read: read.read,
                kind: read.kind,
                implementors: Vec::new(),
            });
            if !found.implementors.contains(&implementor) {
                found.implementors.push(implementor);
            }
        }
        Ok(plan)
    }

    fn refusals(
        program: &TypedProgram,
        reader: &FieldReader,
        refusal: WrittenRefusal,
    ) -> Diagnostics {
        let definitions = (program.modules.iter()).flat_map(|module| &module.definitions);
        let refused = definitions.filter(|definition| definition.binding == reader.binding);
        refused
            .map(|definition| {
                let span = Self::first_read(&definition.value, reader.owner)
                    .map_or(definition.span, |label| label.span);
                FieldOwnerProblems::unkept(refusal, &reader.field, span)
            })
            .collect()
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
        if shape.copies_owner(&plan) {
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
