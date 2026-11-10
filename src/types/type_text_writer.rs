use std::rc::Rc;

use rustc_hash::FxHashSet;

use super::checker::TypeChecker;
use super::ty::{Bound, GenericParam, Scheme, Type, TypeVar};
use super::typed::TypedExpr;
use crate::core::BindingId;
use crate::source::Span;

pub struct TypeTextWriter {
    definition: BindingId,
    enclosing_names: FxHashSet<String>,
}

impl TypeTextWriter {
    pub fn new(definition: BindingId, enclosing: &Scheme) -> Self {
        let enclosing_names = enclosing
            .params
            .iter()
            .map(|param| param.name.to_string())
            .collect();
        Self {
            definition,
            enclosing_names,
        }
    }

    pub fn write(&self, checker: &mut TypeChecker, operand: &TypedExpr) -> Rc<str> {
        let scheme = self.generalize(checker, &operand.ty, operand.span);
        let text: Rc<str> = checker.display().scheme(&scheme).into();
        checker.type_texts.record(self.definition, Rc::clone(&text));
        text
    }

    fn generalize(&self, checker: &mut TypeChecker, operand: &Type, span: Span) -> Scheme {
        let open: Vec<OpenVariable> = Self::open_vars(checker, operand, span)
            .into_iter()
            .map(|var| OpenVariable::of(checker, var))
            .collect();
        let mut names = self.free_names();
        let params: Vec<GenericParam> = open
            .into_iter()
            .map(|variable| {
                let name = names.next().expect("parameter names never run out");
                variable.generalize(checker, &name)
            })
            .collect();
        Scheme {
            lifetimes: Vec::new(),
            params: params
                .into_iter()
                .map(|param| Self::with_named_bounds(checker, param))
                .collect(),
            body: checker.unifier.resolve(operand),
        }
    }

    fn open_vars(checker: &mut TypeChecker, operand: &Type, span: Span) -> Vec<TypeVar> {
        let mut open_vars = Vec::new();
        for var in Self::vars_with_their_bounds(checker, operand) {
            match checker.unifier.literal_kind(var) {
                Some(literal) => checker.default_literal(var, literal, span),
                None => open_vars.push(var),
            }
        }
        open_vars
    }

    fn vars_with_their_bounds(checker: &TypeChecker, operand: &Type) -> Vec<TypeVar> {
        let mut vars = checker.free_vars(operand);
        let mut position = 0;
        while let Some(var) = vars.get(position).copied() {
            for named in Self::vars_in_bounds(checker, var) {
                if !vars.contains(&named) {
                    vars.push(named);
                }
            }
            position += 1;
        }
        vars
    }

    fn vars_in_bounds(checker: &TypeChecker, var: TypeVar) -> Vec<TypeVar> {
        checker
            .self_plain_bounds(var)
            .iter()
            .filter_map(|bound| bound.argument.as_ref())
            .flat_map(|argument| checker.free_vars(argument))
            .collect()
    }

    fn free_names(&self) -> impl Iterator<Item = String> + use<'_> {
        (0..)
            .map(TypeChecker::parameter_name)
            .filter(|name| !self.enclosing_names.contains(name))
    }

    fn with_named_bounds(checker: &TypeChecker, mut param: GenericParam) -> GenericParam {
        for bound in &mut param.bounds {
            bound.argument = bound
                .argument
                .as_ref()
                .map(|argument| checker.unifier.resolve(argument));
        }
        param
    }
}

struct OpenVariable {
    var: TypeVar,
    bounds: Vec<Bound>,
}

impl OpenVariable {
    fn of(checker: &TypeChecker, var: TypeVar) -> Self {
        Self {
            var,
            bounds: checker.self_plain_bounds(var),
        }
    }

    fn generalize(self, checker: &mut TypeChecker, name: &str) -> GenericParam {
        let param = checker.new_generic(name, self.bounds);
        checker.unifier.bind(self.var, Type::Generic(param.id));
        param
    }
}
