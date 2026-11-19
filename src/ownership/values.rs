use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::copied_reads::{ChangedCopy, CopiedMutReads};
use super::function_values::FunctionValues;
use super::parameter_adjustments::{Adjustment, ParameterAdjustments};
use super::problems::{ChangingFunctionValue, OwnershipProblems};
use crate::core::{BindingArena, BindingId, FieldLabel};
use crate::diagnostic::Diagnostic;
use crate::types::{
    CopyRules, FunctionValueWrapper, GenericId, GenericParam, StructTable, Type, TypeDisplay,
    TypedArgument, TypedBlock, TypedBodyItem, TypedControl, TypedDefinition, TypedExpr,
    TypedExprKind, TypedLeaf, TypedProgram, TypedStructExpr,
};

struct WrappedType<'a> {
    parameters: &'a [Type],
    expected_result: &'a Type,
    actual_result: &'a Type,
    actual: &'a Type,
}

#[derive(Clone)]
enum ValueSite {
    Argument,
    Field(Rc<str>),
    Local(BindingId),
}

struct LostChange<'t> {
    function: BindingId,
    actual: &'t Type,
    expected_parameters: &'t [Type],
    position: usize,
    changed: ChangedCopy,
    site: ValueSite,
}

pub struct FunctionValueModes<'a> {
    bindings: &'a mut BindingArena,
    generics: &'a FxHashMap<GenericId, GenericParam>,
    structs: &'a StructTable,
    copied_reads: &'a CopiedMutReads,
    constructor_fields: FxHashMap<BindingId, Vec<Option<Rc<str>>>>,
    function_values: FunctionValues<'a>,
    adjustments: ParameterAdjustments<'a>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> FunctionValueModes<'a> {
    pub fn apply(
        program: &'a mut TypedProgram,
        function_params: &'a FxHashMap<BindingId, Vec<Type>>,
        copied_reads: &'a CopiedMutReads,
    ) -> Vec<Diagnostic> {
        let adjustments = ParameterAdjustments::new(
            CopyRules::new(&program.declarations, &program.generics),
            program.declarations.types().rust_types(),
        );
        let constructor_fields = Self::constructor_fields(program);
        let mut values = Self {
            bindings: &mut program.bindings,
            generics: &program.generics,
            structs: program.declarations.types().structs(),
            copied_reads,
            constructor_fields,
            function_values: FunctionValues::new(function_params, &program.template_borrows),
            adjustments,
            diagnostics: Vec::new(),
        };
        for definition in program
            .modules
            .iter_mut()
            .flat_map(|module| module.definitions.iter_mut())
        {
            values.definition(definition);
        }
        values.diagnostics
    }

    fn constructor_fields(program: &TypedProgram) -> FxHashMap<BindingId, Vec<Option<Rc<str>>>> {
        program
            .modules
            .iter()
            .flat_map(|module| &module.definitions)
            .filter_map(|definition| {
                let TypedExprKind::Fn(lambda) = &definition.value.kind else {
                    return None;
                };
                let [TypedBodyItem::Expr(built)] = lambda.body.items.as_slice() else {
                    return None;
                };
                let TypedExprKind::Struct(TypedStructExpr::New(_, fields)) = &built.kind else {
                    return None;
                };
                let names = lambda
                    .params
                    .iter()
                    .map(|param| Self::field_holding(fields, param.binding))
                    .collect();
                Some((definition.binding, names))
            })
            .collect()
    }

    fn field_holding(fields: &[(FieldLabel, TypedExpr)], param: BindingId) -> Option<Rc<str>> {
        fields
            .iter()
            .find(|(_, value)| {
                matches!(value.kind, TypedExprKind::Leaf(TypedLeaf::Local(binding)) if binding == param)
            })
            .map(|(label, _)| Rc::clone(&label.name))
    }

    fn definition(&mut self, definition: &mut TypedDefinition) {
        let current = self
            .function_values
            .declared_params(definition.binding)
            .map(<[Type]>::to_vec);
        self.expr(&mut definition.value, current.as_deref());
    }

    fn expr(&mut self, expr: &mut TypedExpr, current: Option<&[Type]>) {
        let ty = expr.ty.clone();
        match &mut expr.kind {
            TypedExprKind::Call(callee, arguments) => {
                let expected = self.expected_params(callee);
                let sites = self.argument_sites(callee, arguments.len());
                self.arguments(arguments, expected.as_deref(), &sites, current);
                self.expr(callee, current);
            }
            TypedExprKind::Control(control) => self.control(control, current),
            other => self.nested_expr(other, &ty, current),
        }
    }

    fn nested_expr(&mut self, kind: &mut TypedExprKind, ty: &Type, current: Option<&[Type]>) {
        match kind {
            TypedExprKind::Recur(arguments) => {
                let sites = vec![ValueSite::Argument; arguments.len()];
                self.arguments(arguments, current, &sites, current)
            }
            TypedExprKind::Struct(structure) => self.structure(structure, ty, current),
            TypedExprKind::Variant(_, arguments, _) => self.expressions(arguments, current),
            TypedExprKind::Assoc(value, keys, item) => self.assoc(value, keys, item, current),
            TypedExprKind::TraitObject(value, _) => self.expr(value, current),
            _ => self.value_kind(kind, current),
        }
    }

    fn value_kind(&mut self, kind: &mut TypedExprKind, current: Option<&[Type]>) {
        match kind {
            TypedExprKind::Try { value, .. }
            | TypedExprKind::Borrow(_, value)
            | TypedExprKind::Deref(value) => self.expr(value, current),
            TypedExprKind::Fn(lambda) => {
                let lambda = Rc::make_mut(lambda);
                let params: Vec<Type> =
                    lambda.params.iter().map(|param| param.ty.clone()).collect();
                self.block(&mut lambda.body, Some(&params));
            }
            _ => {}
        }
    }

    fn expressions(&mut self, arguments: &mut [TypedExpr], current: Option<&[Type]>) {
        for argument in arguments {
            self.expr(argument, current);
        }
    }

    fn assoc(
        &mut self,
        value: &mut TypedExpr,
        keys: &mut [TypedExpr],
        item: &mut TypedExpr,
        current: Option<&[Type]>,
    ) {
        self.expr(value, current);
        self.expressions(keys, current);
        self.expr(item, current);
    }

    fn control(&mut self, control: &mut TypedControl, current: Option<&[Type]>) {
        match control {
            TypedControl::Block(block) => self.block(block, current),
            TypedControl::If(test, then, otherwise) => {
                self.expr(test, current);
                self.expr(then, current);
                self.expr(otherwise, current);
            }
            TypedControl::Match(matched) => {
                self.expr(&mut matched.value, current);
                for arm in &mut matched.arms {
                    self.block(&mut arm.body, current);
                }
            }
        }
    }

    fn structure(&mut self, structure: &mut TypedStructExpr, ty: &Type, current: Option<&[Type]>) {
        match structure {
            TypedStructExpr::Field(value, _) => self.expr(value, current),
            TypedStructExpr::New(binding, fields) => {
                self.fields(*binding, fields, ty, current);
            }
            TypedStructExpr::Update(binding, base, fields) => {
                self.expr(base, current);
                self.fields(*binding, fields, ty, current);
            }
        }
    }

    fn fields(
        &mut self,
        binding: BindingId,
        fields: &mut [(FieldLabel, TypedExpr)],
        ty: &Type,
        current: Option<&[Type]>,
    ) {
        let field_types = self.field_types(binding, ty);
        for (label, value) in fields {
            self.field(&field_types, label, value, current);
        }
    }

    fn field(
        &mut self,
        field_types: &[(std::rc::Rc<str>, Type)],
        label: &FieldLabel,
        value: &mut TypedExpr,
        current: Option<&[Type]>,
    ) {
        let (_, expected) = field_types
            .iter()
            .find(|(name, _)| name.as_ref() == label.name.as_ref())
            .expect("the type checker checked every field");
        let expected = expected.clone();
        if !self.compare(value, &expected, ValueSite::Field(Rc::clone(&label.name))) {
            self.expr(value, current);
        }
    }

    fn block(&mut self, block: &mut TypedBlock, current: Option<&[Type]>) {
        for item in &mut block.items {
            match item {
                TypedBodyItem::Let(binder, value) => {
                    let expected = binder.ty.clone();
                    if !self.compare(value, &expected, ValueSite::Local(binder.binding)) {
                        self.expr(value, current);
                    }
                }
                TypedBodyItem::Expr(expr) => self.expr(expr, current),
            }
        }
    }

    fn argument_sites(&self, callee: &TypedExpr, count: usize) -> Vec<ValueSite> {
        let fields = match callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(function)) => {
                self.constructor_fields.get(&function)
            }
            _ => None,
        };
        (0..count)
            .map(
                |position| match fields.and_then(|names| names.get(position).cloned().flatten()) {
                    Some(field) => ValueSite::Field(field),
                    None => ValueSite::Argument,
                },
            )
            .collect()
    }

    fn arguments(
        &mut self,
        arguments: &mut [TypedArgument],
        expected: Option<&[Type]>,
        sites: &[ValueSite],
        current: Option<&[Type]>,
    ) {
        for (position, argument) in arguments.iter_mut().enumerate() {
            let handled = expected
                .and_then(|parameters| parameters.get(position))
                .is_some_and(|expected| {
                    self.compare(&mut argument.value, expected, sites[position].clone())
                });
            if !handled {
                self.expr(&mut argument.value, current);
            }
        }
    }

    fn expected_params(&self, callee: &TypedExpr) -> Option<Vec<Type>> {
        let Type::Function(_, parameters, _) = &callee.ty else {
            return None;
        };
        let used = match &callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => {
                self.function_values.used_params(*binding, parameters)
            }
            _ => None,
        };
        Some(used.unwrap_or_else(|| parameters.clone()))
    }

    fn field_types(&self, binding: BindingId, ty: &Type) -> Vec<(std::rc::Rc<str>, Type)> {
        let definition = self
            .structs
            .get(binding)
            .expect("the type checker checked every struct");
        let empty: &[Type] = &[];
        let arguments = match ty {
            Type::Named(_, arguments) => arguments.as_slice(),
            _ => empty,
        };
        definition.instantiated_fields(arguments)
    }

    fn compare(&mut self, value: &mut TypedExpr, expected: &Type, site: ValueSite) -> bool {
        let Type::Function(_, expected_parameters, expected_result) = expected else {
            return false;
        };
        let Some(function) = self.function_values.recognize(value) else {
            return false;
        };
        let (binding, actual) = (function.binding(), function.ty());
        let Type::Function(_, actual_parameters, actual_result) = actual else {
            return false;
        };
        let adjustments = match self
            .adjustments
            .of_each(expected_parameters, actual_parameters)
        {
            Ok(adjustments) => adjustments,
            Err(parameter) => {
                return function.reports_mismatch()
                    && self.mismatch(value, binding, actual, expected, parameter);
            }
        };
        if let Some((position, changed)) = self.lost_change(binding, &adjustments) {
            let lost = LostChange {
                function: binding,
                actual,
                expected_parameters,
                position,
                changed,
                site,
            };
            return self.changing_value(value, lost);
        }
        if !function.needs_wrapper(expected, &adjustments) {
            if matches!(value.kind, TypedExprKind::Fn(_)) {
                self.unwrap(value, binding, actual);
            }
            return false;
        }
        self.wrap_value(
            value,
            binding,
            WrappedType {
                parameters: expected_parameters,
                expected_result,
                actual_result,
                actual,
            },
            adjustments,
        );
        true
    }

    fn wrap_value(
        &mut self,
        value: &mut TypedExpr,
        binding: BindingId,
        wrapped: WrappedType<'_>,
        adjustments: Vec<Adjustment>,
    ) {
        let original = self.global(binding, wrapped.actual, value.span);
        let mut prepared =
            FunctionValueWrapper::new(self.bindings).prepare(wrapped.parameters, original, binding);
        for (argument, adjustment) in prepared.arguments.iter_mut().zip(adjustments) {
            argument.borrow = adjustment.borrow();
        }
        *value = prepared.finish(wrapped.expected_result, wrapped.actual_result);
    }

    fn mismatch(
        &mut self,
        value: &mut TypedExpr,
        binding: BindingId,
        actual: &Type,
        expected: &Type,
        parameter: usize,
    ) -> bool {
        let (expected, found) = {
            let display = TypeDisplay::new(self.bindings, self.generics);
            (display.show(expected), display.show(actual))
        };
        self.diagnostics
            .push(OwnershipProblems::function_value_mismatch(
                &expected, &found, value.span, parameter,
            ));
        self.unwrap(value, binding, actual);
        true
    }

    fn lost_change(
        &self,
        binding: BindingId,
        adjustments: &[Adjustment],
    ) -> Option<(usize, ChangedCopy)> {
        adjustments
            .iter()
            .enumerate()
            .filter(|(_, adjustment)| **adjustment == Adjustment::Mutable)
            .find_map(|(position, _)| {
                self.copied_reads
                    .changed_at(binding, position)
                    .map(|changed| (position, changed))
            })
    }

    fn changing_value(&mut self, value: &mut TypedExpr, lost: LostChange) -> bool {
        let by_value = Self::with_parameter_by_value(lost.actual, lost.position);
        let display = TypeDisplay::new(self.bindings, self.generics);
        let Type::Function(_, actual_parameters, _) = lost.actual else {
            unreachable!("a function value has a function type")
        };
        let problem = ChangingFunctionValue {
            function: self.bindings.get(lost.function).name(),
            param: self.bindings.get(lost.changed.param).name(),
            expected: display.show(&lost.expected_parameters[lost.position]),
            found: display.show(&actual_parameters[lost.position]),
            by_value: display.show(&by_value),
        };
        let diagnostic = match lost.site {
            ValueSite::Argument => OwnershipProblems::changing_function_argument(
                value.span,
                lost.changed.change,
                &problem,
            ),
            ValueSite::Field(field) => OwnershipProblems::changing_function_field(
                value.span,
                lost.changed.change,
                &problem,
                &field,
            ),
            ValueSite::Local(local) => OwnershipProblems::changing_function_local(
                value.span,
                lost.changed.change,
                &problem,
                self.bindings.get(local).name(),
            ),
        };
        self.diagnostics.push(diagnostic);
        self.unwrap(value, lost.function, lost.actual);
        true
    }

    fn with_parameter_by_value(function: &Type, position: usize) -> Type {
        let Type::Function(kind, parameters, result) = function else {
            unreachable!("a function value has a function type")
        };
        let mut parameters = parameters.clone();
        parameters[position] = parameters[position].referent().clone();
        Type::Function(*kind, parameters, result.clone())
    }

    fn unwrap(&self, value: &mut TypedExpr, binding: BindingId, actual: &Type) {
        *value = self.global(binding, actual, value.span);
    }

    fn global(&self, binding: BindingId, actual: &Type, span: crate::source::Span) -> TypedExpr {
        TypedExpr {
            kind: TypedExprKind::Leaf(TypedLeaf::Global(binding)),
            ty: actual.clone(),
            span,
        }
    }
}
