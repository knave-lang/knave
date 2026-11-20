use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::copy::CopyRules;
use super::declarations::Declarations;
use super::field_owner_problems::FieldOwnerProblems;
use super::field_read::FieldRead;
use super::field_reader::FieldReader;
use super::field_traits::FieldTraitPlan;
use super::open_read::OpenRead;
use super::pinning::InstantiationReport;
use super::reaching_structs::ReachingStructs;
use super::read_settlements::ReadSettlements;
use super::read_view::ReadView;
use super::reader_shape::ReaderShape;
use super::ty::{GenericId, Type};
use super::typed::{TypedExpr, TypedExprKind, TypedProgram, TypedStructExpr};
use super::unborrowed::Unborrowed;
use crate::core::{BindingId, FieldLabel};
use crate::diagnostic::Diagnostics;

pub struct FieldReadSettlement<'r> {
    report: &'r InstantiationReport,
}

enum Outcome {
    Unreached,
    Agreed(FieldTraitPlan),
    Disagreed,
    Unsupported,
}

#[derive(Default)]
struct Tally {
    settlements: ReadSettlements,
    errors: Diagnostics,
    agreements: FxHashMap<BindingId, TraitAgreement>,
}

struct TraitAgreement {
    plan: FieldTraitPlan,
    readers: Vec<FieldReader>,
    disagrees: bool,
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
        tally.refuse_disagreeing_readers(program);
        if !tally.errors.is_empty() {
            return Err(tally.errors);
        }
        tally.settlements.apply_to(program);
        tally.record_plans(&mut program.declarations);
        Ok(())
    }

    fn count(&self, program: &TypedProgram, reader: FieldReader, tally: &mut Tally) {
        let shape = ReaderShape::new(program);
        match self.outcome(program, &reader) {
            Outcome::Agreed(plan)
                if shape.emits(&reader, &plan)
                    && !tally.settlements.settles_otherwise(&reader, &plan.read) =>
            {
                if shape.copies_owner(&plan) {
                    tally.settlements.require_copy(&reader);
                }
                tally.agree(reader, plan);
            }
            Outcome::Agreed(_) | Outcome::Disagreed | Outcome::Unsupported => {
                tally.refuse(program, &reader)
            }
            Outcome::Unreached => {}
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

    fn outcome(&self, program: &TypedProgram, reader: &FieldReader) -> Outcome {
        let Some(structs) = ReachingStructs::new(self.report).of(reader.binding, reader.owner)
        else {
            return Outcome::Unsupported;
        };
        let reads = OpenRead::new(CopyRules::new(&program.declarations, &program.generics));
        let mut found: Vec<FieldRead> = Vec::new();
        let mut implementors: Vec<Type> = Vec::new();
        for (owner, arguments) in structs {
            let concrete = arguments.iter().all(Type::is_concrete);
            let definition = (program.declarations.types().structs.get(owner)).filter(|_| concrete);
            let read =
                definition.and_then(|found| reads.instantiated(found, &reader.field, &arguments));
            let Some(read) = read else {
                return Outcome::Unsupported;
            };
            let implementor = Type::Named(owner, arguments);
            if !found.contains(&read) {
                found.push(read);
            }
            if !implementors.contains(&implementor) {
                implementors.push(implementor);
            }
        }
        match found.as_slice() {
            [] => Outcome::Unreached,
            [FieldRead { read, kind }]
                if Self::meets_reader(reader, read) && self.calls_pass(reader, read) =>
            {
                Outcome::Agreed(FieldTraitPlan {
                    read: read.clone(),
                    kind: *kind,
                    implementors,
                })
            }
            _ => Outcome::Disagreed,
        }
    }

    fn meets_reader(reader: &FieldReader, read: &Type) -> bool {
        ReadView::of(read).is_used_as(&reader.read)
    }

    fn calls_pass(&self, reader: &FieldReader, read: &Type) -> bool {
        let Type::Generic(open) = reader.read else {
            return true;
        };
        let uses = self.report.sited_uses_of(reader.binding, open);
        let mut passed = uses.iter().filter_map(|(_, passed)| passed.as_ref());
        passed.all(|passed| ReadView::of(read).is_used_as(passed))
    }

    fn ambiguous_reads(program: &TypedProgram, reader: &FieldReader) -> Diagnostics {
        let definitions = (program.modules.iter()).flat_map(|module| &module.definitions);
        let refused = definitions.filter(|definition| definition.binding == reader.binding);
        refused
            .map(
                |definition| match Self::first_read(&definition.value, reader.owner) {
                    Some(label) => FieldOwnerProblems::ambiguous(&label.name, label.span),
                    None => FieldOwnerProblems::ambiguous(&reader.field, definition.span),
                },
            )
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
    fn refuse(&mut self, program: &TypedProgram, reader: &FieldReader) {
        (self.errors).extend(FieldReadSettlement::ambiguous_reads(program, reader));
    }

    fn agree(&mut self, reader: FieldReader, plan: FieldTraitPlan) {
        self.settlements.settle(&reader, plan.read.clone());
        match self.agreements.get_mut(&reader.trait_binding) {
            Some(agreement) => agreement.join(reader, plan),
            None => {
                let binding = reader.trait_binding;
                self.agreements
                    .insert(binding, TraitAgreement::new(reader, plan));
            }
        }
    }

    fn refuse_disagreeing_readers(&mut self, program: &TypedProgram) {
        for agreement in self.agreements.values() {
            self.errors.extend(agreement.refusals(program));
        }
    }

    fn record_plans(self, declarations: &mut Declarations) {
        for (binding, agreement) in self.agreements {
            declarations.settle_field_trait(binding, agreement.into_plan());
        }
    }
}

impl TraitAgreement {
    fn new(reader: FieldReader, plan: FieldTraitPlan) -> Self {
        Self {
            plan,
            readers: vec![reader],
            disagrees: false,
        }
    }

    fn join(&mut self, reader: FieldReader, plan: FieldTraitPlan) {
        if self.plan.read != plan.read {
            self.disagrees = true;
        }
        for implementor in plan.implementors {
            if !self.plan.implementors.contains(&implementor) {
                self.plan.implementors.push(implementor);
            }
        }
        self.readers.push(reader);
    }

    fn refusals(&self, program: &TypedProgram) -> Diagnostics {
        let mut refusals = Diagnostics::default();
        let refused = self.readers.iter().filter(|_| self.disagrees);
        for reader in refused {
            refusals.extend(FieldReadSettlement::ambiguous_reads(program, reader));
        }
        refusals
    }

    fn into_plan(self) -> FieldTraitPlan {
        self.plan
    }
}
