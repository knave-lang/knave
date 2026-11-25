use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::field_read_use::FieldReadUse;
use super::needs_ownership_problems::{MacroFieldRead, NeedsOwnershipProblems};
use super::place_text::PlaceText;
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::{
    Borrow, PassedCall, Pipelines, Type, TypedArgument, TypedAssignment, TypedExpr, TypedExprKind,
    TypedLeaf, TypedProgram, TypedStructExpr,
};

#[derive(Clone, Copy)]
enum PassingSite<'e> {
    Call(&'e TypedExpr),
    Assignment(&'e TypedAssignment),
    Recur,
}

impl PassingSite<'_> {
    fn span(&self) -> Option<Span> {
        match self {
            PassingSite::Call(callee) => Some(callee.span),
            PassingSite::Assignment(assignment) => Some(assignment.template.span),
            PassingSite::Recur => None,
        }
    }
}

pub struct OwnershipNeeds<'a> {
    program: &'a TypedProgram,
    function_params: &'a FxHashMap<BindingId, Vec<Type>>,
    pipelines: Pipelines<'a>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> OwnershipNeeds<'a> {
    pub fn new(
        program: &'a TypedProgram,
        function_params: &'a FxHashMap<BindingId, Vec<Type>>,
        pipelines: Pipelines<'a>,
    ) -> Self {
        Self {
            program,
            function_params,
            pipelines,
            diagnostics: Vec::new(),
        }
    }

    pub fn check(mut self, binding: BindingId, value: &TypedExpr) -> Vec<Diagnostic> {
        let current = self.function_params.get(&binding).cloned();
        match &value.kind {
            TypedExprKind::Fn(_) => value
                .children()
                .into_iter()
                .for_each(|child| self.value(child, current.as_deref())),
            _ => self.value(value, current.as_deref()),
        }
        self.diagnostics
    }

    fn value(&mut self, expr: &TypedExpr, current: Option<&[Type]>) {
        match &expr.kind {
            TypedExprKind::Assign(assignment) => self.check_assignment(assignment),
            _ if let Some(PassedCall { callee, arguments }) = PassedCall::of(expr) => {
                self.check_arguments(callee, arguments)
            }
            TypedExprKind::Recur(arguments) => self.check_recur(arguments, current),
            _ => {}
        }
        let child_current = match expr.kind {
            TypedExprKind::Fn(_) => None,
            _ => current,
        };
        for child in expr.children() {
            self.value(child, child_current);
        }
    }

    fn check_arguments(&mut self, callee: &TypedExpr, arguments: &[TypedArgument]) {
        let parameters = self.parameters(callee);
        self.check_with_parameters(PassingSite::Call(callee), arguments, &parameters);
    }

    fn check_assignment(&mut self, assignment: &TypedAssignment) {
        let parameters = self.parameters(&assignment.template);
        let site = PassingSite::Assignment(assignment);
        self.check_with_parameters(site, &assignment.arguments, &parameters);
    }

    fn check_recur(&mut self, arguments: &[TypedArgument], parameters: Option<&[Type]>) {
        if let Some(parameters) = parameters {
            self.check_with_parameters(PassingSite::Recur, arguments, parameters);
        }
    }

    fn check_with_parameters(
        &mut self,
        site: PassingSite<'_>,
        arguments: &[TypedArgument],
        parameters: &[Type],
    ) {
        for (position, argument) in arguments.iter().enumerate() {
            let owns = parameters
                .get(position)
                .is_some_and(|parameter| !parameter.is_reference());
            if argument.borrow == Borrow::Passthrough && owns {
                let diagnostic = self.needs_ownership(site, &argument.value);
                self.diagnostics.push(diagnostic);
            }
        }
    }

    fn needs_ownership(&self, site: PassingSite<'_>, value: &TypedExpr) -> Diagnostic {
        if self.pipelines.is_pipeline(value.ty.referent()) {
            return NeedsOwnershipProblems::needs_pipeline_ownership(value.span);
        }
        match self.macro_field_read(site, value) {
            Some(read) => NeedsOwnershipProblems::macro_field_needs_ownership(&read),
            None => NeedsOwnershipProblems::needs_ownership(value.span),
        }
    }

    fn macro_field_read(
        &self,
        site: PassingSite<'_>,
        value: &TypedExpr,
    ) -> Option<MacroFieldRead<'a>> {
        let TypedExprKind::Struct(TypedStructExpr::Field(_, label)) = &value.kind else {
            return None;
        };
        site.span().filter(|span| *span != value.span)?;
        let macro_name = self.program.macro_field_reads.get(&value.span)?;
        Some(MacroFieldRead {
            macro_name: Rc::clone(macro_name),
            field: Rc::clone(&label.name),
            field_span: label.span,
            field_type: self.program.show_type(value.ty.referent()),
            used: self.field_read_use(site),
        })
    }

    fn field_read_use(&self, site: PassingSite<'_>) -> FieldReadUse<'a> {
        let named =
            |binding: Option<BindingId>| binding.map(|it| self.program.bindings.get(it).name());
        match site {
            PassingSite::Call(callee) => {
                named(Self::function_of(callee)).map_or(FieldReadUse::Unnamed, FieldReadUse::Call)
            }
            PassingSite::Assignment(assignment) => (PlaceText::new(&self.program.bindings))
                .assigned(assignment)
                .map_or(FieldReadUse::UnwrittenAssignment, FieldReadUse::Assignment),
            PassingSite::Recur => FieldReadUse::Unnamed,
        }
    }

    fn function_of(callee: &TypedExpr) -> Option<BindingId> {
        match callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(function) | TypedLeaf::Local(function)) => {
                Some(function)
            }
            _ => None,
        }
    }

    fn parameters(&self, callee: &TypedExpr) -> Vec<Type> {
        if let TypedExprKind::Leaf(TypedLeaf::Global(function) | TypedLeaf::Local(function)) =
            callee.kind
            && let Some(parameters) = self.function_params.get(&function)
        {
            return parameters.clone();
        }
        match &callee.ty {
            Type::Function(_, parameters, _) => parameters.clone(),
            _ => Vec::new(),
        }
    }
}
