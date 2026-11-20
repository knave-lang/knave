use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::copy::CopyRules;
use super::declarations::Declarations;
use super::field_owner_problems::FieldOwnerProblems;
use super::field_traits::{FieldTraitPlan, FieldTraits};
use super::open_read::OpenRead;
use super::pinning::InstantiationReport;
use super::read_kind::ReadKind;
use super::ty::{Bound, GenericId, Type};
use super::typed::{TypedDefinition, TypedExpr, TypedExprKind, TypedProgram, TypedStructExpr};
use crate::core::BindingId;
use crate::diagnostic::Diagnostics;

pub struct FieldReadSettlement<'r> {
    report: &'r InstantiationReport,
}

struct Reader {
    binding: BindingId,
    owner: GenericId,
    read: Type,
    field: Rc<str>,
    trait_binding: BindingId,
}

enum Outcome {
    Unreached,
    Agreed(FieldTraitPlan),
    Disagreed,
}

type Settled = FxHashMap<GenericId, Type>;

#[derive(Default)]
struct Tally {
    settlements: Settlements,
    errors: Diagnostics,
    plans: FxHashMap<BindingId, (FieldTraitPlan, Vec<Reader>)>,
}

#[derive(Default)]
struct Settlements {
    reads: Settled,
    fixed: FxHashMap<(BindingId, GenericId), Type>,
}

impl<'r> FieldReadSettlement<'r> {
    pub fn new(report: &'r InstantiationReport) -> Self {
        Self { report }
    }

    pub fn settle(&self, program: &mut TypedProgram) -> Result<(), Diagnostics> {
        let mut tally = Tally::default();
        for reader in Self::readers(program) {
            self.count(program, reader, &mut tally);
        }
        tally.refuse_disagreeing_readers(program);
        if !tally.errors.is_empty() {
            return Err(tally.errors);
        }
        Self::apply(program, &tally.settlements);
        tally.record_plans(&mut program.declarations);
        Ok(())
    }

    fn count(&self, program: &TypedProgram, reader: Reader, tally: &mut Tally) {
        match (self.outcome(program, &reader), &reader.read) {
            (Outcome::Agreed(plan), _) if Self::escapes(program, &reader, &plan) => {
                tally.refuse(program, &reader);
            }
            (Outcome::Agreed(plan), _) => tally.agree(reader, plan),
            (Outcome::Disagreed, Type::Generic(_)) => tally.refuse(program, &reader),
            (Outcome::Disagreed | Outcome::Unreached, _) => {}
        }
    }

    fn readers(program: &TypedProgram) -> Vec<Reader> {
        let field_traits = &program.declarations.traits().rust_paths.field_traits;
        let definitions = program
            .modules
            .iter()
            .flat_map(|module| &module.definitions);
        let params = definitions.flat_map(|definition| {
            (definition.scheme.params.iter()).map(move |param| (definition.binding, param))
        });
        params
            .flat_map(|(binding, param)| {
                param.bounds.iter().filter_map(move |bound| {
                    let field = field_traits.field_of(bound.binding)?;
                    Some(Reader {
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

    fn outcome(&self, program: &TypedProgram, reader: &Reader) -> Outcome {
        let mut visited = FxHashSet::default();
        let structs = self.reaching_structs(reader.binding, reader.owner, &mut visited);
        let reads = OpenRead::new(CopyRules::new(&program.declarations, &program.generics));
        let mut found: Vec<(Type, ReadKind)> = Vec::new();
        let mut implementors: Vec<Type> = Vec::new();
        for (owner, arguments) in structs {
            let definition = program.declarations.types().structs.get(owner);
            let Some((read, kind)) = definition.and_then(|found| reads.of(found, &reader.field))
            else {
                continue;
            };
            let replacements = (definition.into_iter())
                .flat_map(|found| found.params.iter().map(|param| param.id))
                .zip(arguments.iter().cloned())
                .collect();
            let implementor = Type::Named(owner, arguments.clone());
            let read = (read.substitute(&replacements), kind);
            if !found.contains(&read) {
                found.push(read);
            }
            if !implementors.contains(&implementor) {
                implementors.push(implementor);
            }
        }
        match found.as_slice() {
            [] => Outcome::Unreached,
            [(read, kind)] => {
                Outcome::Agreed(FieldTraitPlan::new(read.clone(), *kind, implementors))
            }
            _ => Outcome::Disagreed,
        }
    }

    fn reaching_structs(
        &self,
        binding: BindingId,
        generic: GenericId,
        visited: &mut FxHashSet<(BindingId, GenericId)>,
    ) -> Vec<(BindingId, Vec<Type>)> {
        let mut structs = Vec::new();
        for (enclosing, used) in self.report.sited_uses_of(binding, generic) {
            match used.map(Self::without_borrows) {
                Some(Type::Named(owner, arguments)) => structs.push((owner, arguments)),
                Some(Type::Generic(caller)) if visited.insert((enclosing, caller)) => {
                    structs.extend(self.reaching_structs(enclosing, caller, visited));
                }
                _ => {}
            }
        }
        structs
    }

    fn without_borrows(mut used: Type) -> Type {
        while let Type::Ref(_, inner) | Type::Mut(_, inner) = used {
            used = *inner;
        }
        used
    }

    fn escapes(program: &TypedProgram, reader: &Reader, plan: &FieldTraitPlan) -> bool {
        if plan.kind == ReadKind::BorrowOfStruct || !Self::holds_borrow(program, &plan.read) {
            return false;
        }
        let definitions = program
            .modules
            .iter()
            .flat_map(|module| &module.definitions);
        let exits = definitions
            .filter(|definition| definition.binding == reader.binding)
            .find_map(|definition| match &definition.scheme.body {
                Type::Function(_, parameters, result) => {
                    let changed = parameters
                        .iter()
                        .filter(|parameter| matches!(parameter, Type::Mut(..)));
                    Some(
                        changed
                            .chain(std::iter::once(result.as_ref()))
                            .collect::<Vec<_>>(),
                    )
                }
                _ => None,
            });
        let settled: Settled = match reader.read {
            Type::Generic(open) => std::iter::once((open, plan.read.clone())).collect(),
            _ => Settled::default(),
        };
        let mut exits = exits.into_iter().flatten();
        exits.any(|exit| Self::holds_borrow(program, &exit.substitute(&settled)))
    }

    fn holds_borrow(program: &TypedProgram, read: &Type) -> bool {
        match read {
            Type::Ref(..) | Type::Mut(..) => true,
            Type::Named(binding, arguments) => {
                program.has_lifetime_parameter(*binding)
                    || arguments
                        .iter()
                        .any(|argument| Self::holds_borrow(program, argument))
            }
            _ => false,
        }
    }

    fn ambiguous_reads(program: &TypedProgram, reader: &Reader) -> Diagnostics {
        let mut found = Diagnostics::default();
        let definitions = program
            .modules
            .iter()
            .flat_map(|module| &module.definitions);
        for definition in definitions.filter(|definition| definition.binding == reader.binding) {
            Self::collect_reads(&definition.value, reader.owner, &mut found);
        }
        found
    }

    fn collect_reads(expr: &TypedExpr, owner: GenericId, found: &mut Diagnostics) {
        if let TypedExprKind::Struct(TypedStructExpr::Field(value, label)) = &expr.kind
            && Self::without_borrows(value.ty.clone()) == Type::Generic(owner)
        {
            found.push(FieldOwnerProblems::ambiguous(&label.name, label.span));
        }
        for child in expr.children() {
            Self::collect_reads(child, owner, found);
        }
    }

    fn apply(program: &mut TypedProgram, settled: &Settlements) {
        let field_traits = &program.declarations.traits().rust_paths.field_traits;
        for (id, param) in &mut program.generics {
            for bound in &mut param.bounds {
                *bound = bound.substituted(&settled.reads);
            }
            Self::fix_reads(field_traits, &mut param.bounds, settled.fixed_for(*id));
        }
        let definitions = program
            .modules
            .iter_mut()
            .flat_map(|module| &mut module.definitions);
        definitions
            .for_each(|definition| Self::settle_definition(field_traits, definition, settled));
    }

    fn fix_reads(field_traits: &FieldTraits, bounds: &mut [Bound], fixed: Option<&Type>) {
        let Some(read) = fixed else {
            return;
        };
        for bound in bounds
            .iter_mut()
            .filter(|bound| field_traits.field_of(bound.binding).is_some())
        {
            bound.argument = Some(read.clone());
        }
    }

    fn settle_definition(
        field_traits: &FieldTraits,
        definition: &mut TypedDefinition,
        settled: &Settlements,
    ) {
        let scheme = &mut definition.scheme;
        scheme
            .params
            .retain(|param| !settled.reads.contains_key(&param.id));
        for param in &mut scheme.params {
            for bound in &mut param.bounds {
                *bound = bound.substituted(&settled.reads);
            }
            let fixed = settled.fixed.get(&(definition.binding, param.id));
            Self::fix_reads(field_traits, &mut param.bounds, fixed);
        }
        scheme.body = scheme.body.substitute(&settled.reads);
        Self::settle_expr(&mut definition.value, &settled.reads);
    }

    fn settle_expr(expr: &mut TypedExpr, settled: &Settled) {
        expr.ty = expr.ty.substitute(settled);
        if let TypedExprKind::Fn(lambda) = &mut expr.kind {
            for param in &mut Rc::make_mut(lambda).params {
                param.ty = param.ty.substitute(settled);
            }
        }
        for child in expr.children_mut() {
            Self::settle_expr(child, settled);
        }
    }
}

impl Settlements {
    fn settle(&mut self, reader: &Reader, read: Type) {
        match reader.read {
            Type::Generic(open) => self.reads.insert(open, read),
            _ => self.fixed.insert((reader.binding, reader.owner), read),
        };
    }

    fn fixed_for(&self, owner: GenericId) -> Option<&Type> {
        (self.fixed.iter())
            .find_map(|((_, fixed_owner), read)| (*fixed_owner == owner).then_some(read))
    }
}

impl Tally {
    fn refuse(&mut self, program: &TypedProgram, reader: &Reader) {
        (self.errors).extend(FieldReadSettlement::ambiguous_reads(program, reader));
    }

    fn agree(&mut self, reader: Reader, plan: FieldTraitPlan) {
        self.settlements.settle(&reader, plan.read.clone());
        let (known, readers) = (self.plans)
            .entry(reader.trait_binding)
            .or_insert_with(|| (plan.clone(), Vec::new()));
        if known.read == plan.read {
            known.merge(plan);
        } else {
            known.disagrees = true;
        }
        readers.push(reader);
    }

    fn refuse_disagreeing_readers(&mut self, program: &TypedProgram) {
        let disagreeing = (self.plans.values()).filter(|(plan, _)| plan.disagrees);
        for reader in disagreeing.flat_map(|(_, readers)| readers) {
            (self.errors).extend(FieldReadSettlement::ambiguous_reads(program, reader));
        }
    }

    fn record_plans(self, declarations: &mut Declarations) {
        for (binding, (plan, _)) in self.plans {
            declarations.settle_field_trait(binding, plan);
        }
    }
}
