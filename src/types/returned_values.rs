use rustc_hash::FxHashMap;

use super::{
    Pipelines, Type, TypedBlock, TypedBodyItem, TypedControl, TypedExpr, TypedExprKind,
    TypedLambda, TypedLeaf, TypedProgram,
};
use crate::core::BindingId;

type Lets<'e> = FxHashMap<BindingId, &'e TypedExpr>;

pub struct ReturnedValues<'p> {
    program: &'p TypedProgram,
}

impl<'p> ReturnedValues<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub fn closures<'e>(&self, lambda: &'e TypedLambda) -> Vec<&'e TypedExpr> {
        let mut closures = Vec::new();
        Self::visit_block(&lambda.body, Lets::default(), &mut |value, lets| {
            self.collect_closures(value, lets, &mut closures);
        });
        closures
    }

    pub fn keeps_borrow(lambda: &TypedLambda, param: BindingId) -> bool {
        let mut keeps = false;
        Self::visit_block(&lambda.body, Lets::default(), &mut |value, _| {
            keeps |= Self::captures(value, param);
        });
        keeps
    }

    fn visit_block<'e>(
        block: &'e TypedBlock,
        mut lets: Lets<'e>,
        visit: &mut impl FnMut(&'e TypedExpr, &Lets<'e>),
    ) {
        for item in &block.items {
            if let TypedBodyItem::Let(binder, value) = item {
                lets.insert(binder.binding, value);
            }
        }
        if let Some(tail) = block.tail() {
            Self::visit_value(tail, &lets, visit);
        }
    }

    fn visit_value<'e>(
        value: &'e TypedExpr,
        lets: &Lets<'e>,
        visit: &mut impl FnMut(&'e TypedExpr, &Lets<'e>),
    ) {
        match &value.kind {
            TypedExprKind::Control(TypedControl::Block(block)) => {
                Self::visit_block(block, lets.clone(), visit);
            }
            TypedExprKind::Control(TypedControl::If(_, then, otherwise)) => {
                Self::visit_value(then, lets, visit);
                Self::visit_value(otherwise, lets, visit);
            }
            TypedExprKind::Control(TypedControl::Match(matched)) => {
                for arm in &matched.arms {
                    Self::visit_block(&arm.body, lets.clone(), visit);
                }
            }
            _ => visit(value, lets),
        }
    }

    fn collect_closures<'e>(
        &self,
        value: &'e TypedExpr,
        lets: &Lets<'e>,
        closures: &mut Vec<&'e TypedExpr>,
    ) {
        match &value.kind {
            TypedExprKind::Fn(_) => closures.push(value),
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) if lets.contains_key(binding) => {
                self.collect_closures(lets[binding], lets, closures);
            }
            _ if self.may_hold_closure(&value.ty) => {
                for child in value.children() {
                    self.collect_closures(child, lets, closures);
                }
            }
            _ => {}
        }
    }

    fn captures(value: &TypedExpr, param: BindingId) -> bool {
        match &value.kind {
            TypedExprKind::Fn(_) => value.mentions_local(param),
            _ => value
                .children()
                .into_iter()
                .any(|child| Self::captures(child, param)),
        }
    }

    fn may_hold_closure(&self, ty: &Type) -> bool {
        match ty {
            Type::Function(..) => true,
            Type::Ref(_, inner) | Type::Mut(_, inner) => self.may_hold_closure(inner),
            Type::Named(_, arguments) => {
                Pipelines::new(&self.program.rust_types).is_pipeline(ty)
                    || arguments
                        .iter()
                        .any(|argument| self.may_hold_closure(argument))
            }
            Type::Var(_) | Type::Generic(_) | Type::Primitive(_) | Type::TraitObject(_) => false,
        }
    }
}
