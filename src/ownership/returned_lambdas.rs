use std::rc::Rc;

use crate::types::{
    TypedBlock, TypedBodyItem, TypedControl, TypedExpr, TypedExprKind, TypedProgram,
};

pub struct ReturnedLambdas;

impl ReturnedLambdas {
    pub fn mark(program: &mut TypedProgram) {
        for definition in program
            .modules
            .iter_mut()
            .flat_map(|module| &mut module.definitions)
        {
            if let TypedExprKind::Fn(function) = &mut definition.value.kind {
                Self::mark_block(&mut Rc::make_mut(function).body);
            }
        }
    }

    fn mark_block(block: &mut TypedBlock) {
        if let Some(TypedBodyItem::Expr(tail)) = block.items.last_mut() {
            Self::mark_value(tail);
        }
    }

    fn mark_value(value: &mut TypedExpr) {
        match &mut value.kind {
            TypedExprKind::Fn(lambda) => Rc::make_mut(lambda).facts.returned = true,
            TypedExprKind::Control(TypedControl::Block(block)) => Self::mark_block(block),
            TypedExprKind::Control(TypedControl::If(_, then, otherwise)) => {
                Self::mark_value(then);
                Self::mark_value(otherwise);
            }
            TypedExprKind::Control(TypedControl::Match(matched)) => {
                for arm in &mut matched.arms {
                    Self::mark_block(&mut arm.body);
                }
            }
            TypedExprKind::Call(callee, _) => {
                if let TypedExprKind::Fn(inlined) = &mut callee.kind {
                    Self::mark_block(&mut Rc::make_mut(inlined).body);
                }
            }
            _ => {}
        }
    }
}
