mod argument_role;
mod argument_start;
mod assigned_in_place;
mod bound_before;
mod call_order;
mod callee_definitions;
mod constant_definitions;
mod converted_argument;
mod destructured_wrapper;
mod double_negation;
mod emitted_code;
mod emitted_types;
mod emitted_value;
mod entry_point;
mod expressions;
mod field_getter;
mod field_trait_items;
mod format_arguments;
mod format_string;
mod fragment;
mod fresh_lifetimes;
mod function_borrow;
mod generic_callees;
mod generic_positions;
mod identifier;
mod item_visibilities;
mod items;
mod lending_expression;
mod lent_temporaries;
mod let_annotation;
mod let_statement;
mod lift_placement;
mod lifted_temporary;
mod lifted_value;
mod literal;
mod literal_generics;
mod loan;
mod local_closures;
mod locals;
mod macro_call;
mod method_elisions;
mod module_tree;
mod mutable_locals;
mod nesting;
mod number_literal;
mod opaque_borrows;
mod open_calls;
mod open_delimiter;
mod open_literals;
mod operator;
mod plan;
mod preceded_code;
mod print_formats;
mod problems;
mod reused_locals;
mod runtime;
mod runtime_part;
mod rust_fallback;
mod rust_leaf;
mod scanned_part;
mod short_form;
mod signature_borrows;
mod stored_functions;
mod structural_impls;
mod substitution;
mod template;
mod template_arguments;
mod temporary_scope;
#[cfg(test)]
mod test_program;
mod trait_use_search;
mod tree_position;
mod type_anchor;
mod type_context;
mod types;
mod unlent_parameters;
mod unsized_parameters;
mod untied_borrows;
mod visibility;
mod writer;

pub use runtime::RuntimeModule;
pub use writer::SpanMap;

use crate::diagnostic::Diagnostics;
use crate::source::Span;
use crate::types::TypedProgram;
use emitted_types::EmittedTypes;
use items::ItemEmitter;
use plan::EmitPlan;
use unsized_parameters::UnsizedParameters;
use writer::RustWriter;

pub struct EmittedCrate {
    pub code: String,
    pub span_map: SpanMap,
    pub names: Vec<(String, String)>,
    pub emission: EmissionWork,
    pub formatting: Option<FormattingWork>,
}

#[derive(Clone, Copy, Default)]
pub struct EmissionWork {
    pub emitted_expressions: usize,
}

#[derive(Clone, Copy)]
pub struct FormattingWork {
    pub scanned_tokens: usize,
}

pub struct RustEmitter;

impl RustEmitter {
    #[inline(never)]
    pub fn emit(program: &TypedProgram, entry: Span) -> Result<EmittedCrate, Diagnostics> {
        let plan = EmitPlan::build(program, entry)?;
        let unsized_parameters = UnsizedParameters::of(program, &plan);
        let types = EmittedTypes::new(program.declarations.types(), &unsized_parameters);
        let mut items = ItemEmitter::new(program, &plan, types);
        let fragments = items.crate_fragments();
        let emission = items.emission();
        let mut problems = Diagnostics::default();
        items
            .problems
            .into_iter()
            .for_each(|problem| problems.push(problem));
        let mut writer = RustWriter::default();
        writer.write(&fragments);
        let (code, span_map) = writer.finish();
        let mut names = items.names;
        names.extend(
            plan.rust_names()
                .map(|(rust, user)| (rust.to_string(), user.to_string())),
        );
        problems.into_result(EmittedCrate {
            code,
            span_map,
            names,
            emission,
            formatting: None,
        })
    }
}
