mod aliased_changes;
mod aliases;
mod borrowing_lambdas;
mod callers;
mod changes;
mod checks;
mod copied_reads;
mod definitions;
mod escaping_lambdas;
mod functions;
mod gate;
mod held;
mod kinds;
mod lambda_address;
#[cfg(test)]
mod lambda_source;
mod lent_borrows;
mod local_borrows;
mod modes;
mod moved_values;
mod moves;
mod mutation;
mod names;
mod parameter_uses;
mod place_arguments;
mod problems;
mod promoted_constants;
mod recurrence;
mod refusals;
mod repeats;
mod returned;
mod returned_closure_uses;
mod rewrite;
mod shrunk_parameters;
mod slices;
mod slots;
mod stored;
mod stored_definitions;
mod subtree;
mod tail_calls;
mod trait_slices;
mod uncoerced_parameters;
mod update_bases;
mod update_order;
mod usage;
mod value_kinds;
mod values;
mod whole_value_changes;
mod worklist;

use rustc_hash::FxHashMap;

use crate::core::{BindingArena, BindingId};
use crate::diagnostic::Diagnostics;
use crate::types::{
    CopyRules, InferredCopies, ParameterModes, PendingModeCheck, Pipelines, TypedExprKind,
    TypedProgram,
};
use aliased_changes::AliasedChanges;
use borrowing_lambdas::BorrowingLambdas;
use checks::OwnershipNeeds;
use copied_reads::CopiedMutReads;
use escaping_lambdas::EscapingLambdas;
use functions::{FunctionTable, ParamMode};
use kinds::{ClosureKinds, KindReport, KindSources};
use modes::ModeInference;
use moves::{MoveChecker, MoveReport};
use names::ChangingFunctionNames;
use place_arguments::PlaceArguments;
use problems::OwnershipProblems;
use returned::ReturnedBorrows;
use returned_closure_uses::ReturnedClosureUses;
use rewrite::ModeRewrite;
use slices::ShrinkSelection;
use stored::StoredClosures;
use stored_definitions::StoredDefinitions;
use uncoerced_parameters::UncoercedMethodParameters;
use values::FunctionValueModes;

pub use changes::InPlaceChanges;
pub use gate::{EnforcedOwnership, OwnershipGate, WaivedOwnership};
pub use mutation::ChangedValues;
pub use tail_calls::TailCalls;
pub use update_order::StructUpdateOrder;

pub struct OwnershipChecker;

struct OwnershipFindings {
    inferred_copies: InferredCopies,
    refusals: Diagnostics,
    warnings: Diagnostics,
}

impl OwnershipFindings {
    fn enforced(mut self, program: &mut TypedProgram) -> Result<InferredCopies, Diagnostics> {
        if self.refusals.has_errors() {
            self.refusals.extend(self.warnings);
        } else {
            program.warnings.extend(self.warnings);
        }
        self.refusals.into_result(self.inferred_copies)
    }

    fn waived(self, program: &mut TypedProgram) -> InferredCopies {
        program.warnings.extend(self.warnings);
        self.inferred_copies
    }
}

impl OwnershipChecker {
    pub fn check(program: &mut TypedProgram) -> Result<InferredCopies, Diagnostics> {
        Self::findings(program).enforced(program)
    }

    pub fn waived(program: &mut TypedProgram) -> InferredCopies {
        Self::findings(program).waived(program)
    }

    fn findings(program: &mut TypedProgram) -> OwnershipFindings {
        let functions = FunctionTable::collect(program);
        let modes = ModeInference::new(
            &functions,
            CopyRules::new(&program.declarations, &program.generics),
        )
        .run();
        program.parameter_modes = ParameterModes::new(modes.clone());
        let mut diagnostics = Diagnostics::default();
        Self::check_written_modes(
            &program.pending_mode_checks,
            &functions,
            &modes,
            &program.bindings,
            &mut diagnostics,
        );
        let slices = ShrinkSelection::of(program, &functions, &modes)
            .map(|selection| selection.run())
            .unwrap_or_default();
        let copy = CopyRules::new(&program.declarations, &program.generics);
        let mut rewrite = ModeRewrite::new(
            &functions,
            &modes,
            slices.parameters(),
            &copy,
            UncoercedMethodParameters::of(program),
        );
        let copied_reads = CopiedMutReads::new(&functions, &modes, &copy);
        rewrite.apply(program);
        copied_reads.apply(program, rewrite.function_params());
        let copy = CopyRules::new(&program.declarations, &program.generics);
        diagnostics.extend(
            PlaceArguments::new(program, &functions, rewrite.function_params(), &copy).check(),
        );
        diagnostics.extend(AliasedChanges::new(program, rewrite.function_params()).check());
        diagnostics.extend(ReturnedClosureUses::new(program, &copy).check());
        slices.rewrite_declarations(program);
        FunctionValueModes::apply(program, rewrite.function_params(), &copied_reads)
            .into_iter()
            .for_each(|diagnostic| diagnostics.push(diagnostic));
        program.escaping_lambdas = EscapingLambdas::in_program(program);
        StoredClosures::check(program)
            .into_iter()
            .for_each(|diagnostic| diagnostics.push(diagnostic));
        program.borrowing_lambdas = BorrowingLambdas::in_program(program);
        let kinds = ClosureKinds::new(KindSources {
            declarations: &program.declarations,
            generics: &program.generics,
            bindings: &program.bindings,
            template_borrows: &program.template_borrows,
            function_parameters: ClosureKinds::function_parameters(program),
            rust_parameter_kinds: ClosureKinds::rust_parameter_kinds(program),
            written_parameter_kinds: ClosureKinds::written_parameter_kinds(program),
            borrowing_lambdas: &program.borrowing_lambdas,
        });
        let report = kinds.run(&mut program.modules);
        Self::apply_kinds(program, report, &mut diagnostics);
        let mut inferred_copies = InferredCopies::default();
        let pipelines = Pipelines::new(program.declarations.types().rust_types());
        let stored_definitions = StoredDefinitions::of(program);
        let returned_borrows = ReturnedBorrows::new(program);
        for definition in program
            .modules
            .iter()
            .flat_map(|module| &module.definitions)
        {
            returned_borrows
                .check_definition(definition)
                .into_iter()
                .for_each(|diagnostic| diagnostics.push(diagnostic));
            let report = match &definition.value.kind {
                TypedExprKind::Fn(lambda) => {
                    Self::move_checker(program, pipelines, &stored_definitions)
                        .check(definition.binding, lambda)
                }
                _ if stored_definitions.contains(definition.binding) => {
                    Self::move_checker(program, pipelines, &stored_definitions)
                        .check_value(&definition.value)
                }
                _ => MoveReport::default(),
            };
            inferred_copies.extend(report.inferred_copies);
            report
                .diagnostics
                .into_iter()
                .for_each(|diagnostic| diagnostics.push(diagnostic));
            let needs = OwnershipNeeds::new(program, rewrite.function_params(), pipelines)
                .check(definition.binding, &definition.value);
            needs
                .into_iter()
                .for_each(|diagnostic| diagnostics.push(diagnostic));
        }
        OwnershipFindings {
            inferred_copies,
            refusals: diagnostics,
            warnings: ChangingFunctionNames::check(program),
        }
    }

    fn move_checker<'a>(
        program: &'a TypedProgram,
        pipelines: Pipelines<'a>,
        stored_definitions: &'a StoredDefinitions,
    ) -> MoveChecker<'a> {
        MoveChecker::new(
            CopyRules::new(&program.declarations, &program.generics),
            program.declarations.types().structs(),
            &program.bindings,
            &program.template_borrows,
            &program.generics,
            pipelines,
            stored_definitions,
        )
    }

    fn apply_kinds(program: &mut TypedProgram, report: KindReport, diagnostics: &mut Diagnostics) {
        program.mutable_function_bindings = report.mutable_bindings;
        report
            .diagnostics
            .into_iter()
            .for_each(|diagnostic| diagnostics.push(diagnostic));
    }

    fn check_written_modes(
        checks: &[PendingModeCheck],
        functions: &FunctionTable,
        modes: &FxHashMap<BindingId, ParamMode>,
        bindings: &BindingArena,
        diagnostics: &mut Diagnostics,
    ) {
        for check in checks {
            let Some(info) = functions.get(check.callee) else {
                continue;
            };
            let Some(slot) = info.params.get(check.position) else {
                continue;
            };
            let mode = modes.get(&slot.binding).copied().unwrap_or(ParamMode::Own);
            if mode.accepts(check.kind) {
                continue;
            }
            diagnostics.push(OwnershipProblems::written_mode_mismatch(
                bindings.get(slot.binding).name(),
                check.kind,
                mode,
                check.span,
            ));
        }
    }
}
