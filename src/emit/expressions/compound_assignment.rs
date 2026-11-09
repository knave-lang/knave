use super::{ArgumentRole, ExpressionEmitter, TypeContext};
use crate::core::{ReferenceKind, RustTemplate};
use crate::emit::fragment::Code;
use crate::source::Span;
use crate::types::{
    Borrow, Type, TypedArgument, TypedExpr, TypedExprKind, TypedLeaf, TypedStructExpr,
};

pub(super) struct CompoundAssignment {
    pub template: String,
    pub arguments: Vec<TypedArgument>,
}

struct Assignment<'e> {
    target: &'e str,
    places: &'e [TypedArgument],
    operator: &'static str,
}

struct Operands<'e> {
    place: &'e TypedArgument,
    value: &'e TypedArgument,
    commuted: bool,
}

impl ExpressionEmitter<'_> {
    const COMMUTING_OPERATORS: [&'static str; 2] = ["+", "*"];
    const READ_PLACE: &'static str = "{!current}";
    const COMPUTED_VALUE: &'static str = "{!value}";

    pub(super) fn compound_assignment(
        &mut self,
        template: &RustTemplate,
        arguments: &[TypedArgument],
        types: &[(String, Code)],
        span: Span,
    ) -> Option<CompoundAssignment> {
        let (value, places) = arguments.split_last()?;
        let assigned = format!(" = {{{}}}", places.len());
        let target = template.template.strip_suffix(assigned.as_str())?;
        let TypedExprKind::Call(callee, operands) = &value.value.kind else {
            return None;
        };
        let [left, right] = operands.as_slice() else {
            return None;
        };
        let operator = self.compound_operator(callee)?;
        if !places.iter().all(|place| self.is_steady(&place.value)) {
            return None;
        }
        let target_text = self.target_text(template, target, places, types, span);
        let operands =
            self.operands_naming(&target_text, operator, [left, right], &value.value.ty)?;
        self.is_steady(&operands.place.value).then(|| {
            Self::compound(
                &Assignment {
                    target,
                    places,
                    operator,
                },
                &operands,
            )
        })
    }

    fn compound(assignment: &Assignment, operands: &Operands) -> CompoundAssignment {
        let Assignment {
            target,
            places,
            operator,
        } = assignment;
        let value = format!("{{{}}}", places.len());
        let template = if Self::leaves_places_alone(&operands.value.value) {
            format!("{target} {operator}= {value}")
        } else if operands.commuted {
            let computed = Self::COMPUTED_VALUE;
            format!("{{{{ let {computed} = {value}; {target} {operator}= {computed} }}}}")
        } else {
            let current = Self::READ_PLACE;
            format!("{{{{ let {current} = {target}; {target} = {current} {operator} {value} }}}}")
        };
        let mut arguments = places.to_vec();
        arguments.push(operands.value.clone());
        CompoundAssignment {
            template,
            arguments,
        }
    }

    fn target_text(
        &mut self,
        template: &RustTemplate,
        target: &str,
        places: &[TypedArgument],
        types: &[(String, Code)],
        span: Span,
    ) -> String {
        let target_template = RustTemplate {
            template: target.into(),
            ..template.clone()
        };
        self.template_call(&target_template, places, types.to_vec(), span)
            .text()
    }

    fn operands_naming<'e>(
        &mut self,
        target_text: &str,
        operator: &str,
        [left, right]: [&'e TypedArgument; 2],
        ty: &Type,
    ) -> Option<Operands<'e>> {
        if self.operand_text(left) == target_text {
            return Some(Operands {
                place: left,
                value: right,
                commuted: false,
            });
        }
        let commutes =
            Self::COMMUTING_OPERATORS.contains(&operator) && matches!(ty, Type::Primitive(_));
        (commutes && self.operand_text(right) == target_text).then_some(Operands {
            place: right,
            value: left,
            commuted: true,
        })
    }

    fn operand_text(&mut self, operand: &TypedArgument) -> String {
        self.argument(operand, TypeContext::Known, ArgumentRole::Ordinary)
            .text()
    }

    fn is_steady(&self, expr: &TypedExpr) -> bool {
        match &expr.kind {
            TypedExprKind::Leaf(_) => true,
            TypedExprKind::Deref(value)
            | TypedExprKind::Borrow(ReferenceKind::Shared, value)
            | TypedExprKind::Struct(TypedStructExpr::Field(value, _)) => self.is_steady(value),
            TypedExprKind::Call(callee, arguments) => {
                self.called_template(callee).is_some()
                    && arguments.iter().all(|argument| {
                        Self::passes_unchanged(argument) && self.is_steady(&argument.value)
                    })
            }
            _ => false,
        }
    }

    fn leaves_places_alone(expr: &TypedExpr) -> bool {
        let changes = match &expr.kind {
            TypedExprKind::Call(callee, arguments) => {
                !matches!(
                    callee.kind,
                    TypedExprKind::Leaf(TypedLeaf::Global(_) | TypedLeaf::Rust(_))
                ) || !arguments.iter().all(Self::passes_unchanged)
            }
            _ => false,
        };
        !changes && expr.children().into_iter().all(Self::leaves_places_alone)
    }

    fn passes_unchanged(argument: &TypedArgument) -> bool {
        !matches!(argument.borrow, Borrow::Mutable | Borrow::Reborrow)
            && !matches!(argument.value.ty, Type::Mut(..))
    }
}
