use super::{ArgumentRole, ExpressionEmitter, TypeContext};
use crate::core::RustTemplate;
use crate::emit::fragment::Code;
use crate::source::Span;
use crate::types::{
    Borrow, Type, TypedArgument, TypedExpr, TypedExprKind, TypedLeaf, TypedStructExpr,
};

pub(super) struct CompoundAssignment {
    pub template: String,
    pub arguments: Vec<TypedArgument>,
}

struct Operands<'e> {
    value: &'e TypedArgument,
    place_first: bool,
}

impl ExpressionEmitter<'_> {
    const COMMUTING_OPERATORS: [&'static str; 2] = ["+", "*"];
    const ELEMENT_READ: &'static str = "&{0}[{1}]";
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
        let operator = self.compound_operator(callee, &value.value.ty)?;
        if !places.iter().all(|place| self.is_steady(&place.value)) {
            return None;
        }
        let target_text = self.target_text(template, target, places, types, span);
        let operands = self.operands_beside(&target_text, operator, [left, right])?;
        let value = format!("{{{}}}", places.len());
        let template = if Self::leaves_places_alone(&operands.value.value) {
            format!("{target} {operator}= {value}")
        } else {
            let assigned = Self::in_written_order(&operands, &target_text, operator, &value);
            format!("{target} = {assigned}")
        };
        Some(CompoundAssignment {
            template,
            arguments: places.iter().chain([operands.value]).cloned().collect(),
        })
    }

    fn in_written_order(
        operands: &Operands,
        target_text: &str,
        operator: &str,
        value: &str,
    ) -> String {
        let place = target_text.replace('{', "{{").replace('}', "}}");
        let (current, computed) = (Self::READ_PLACE, Self::COMPUTED_VALUE);
        let statements = if operands.place_first {
            format!(
                "let {current} = {place}; let {computed} = {value}; {current} {operator} {computed}"
            )
        } else {
            format!("let {computed} = {value}; {computed} {operator} {place}")
        };
        format!("{{{{ {statements} }}}}")
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

    fn operands_beside<'e>(
        &mut self,
        target_text: &str,
        operator: &str,
        [left, right]: [&'e TypedArgument; 2],
    ) -> Option<Operands<'e>> {
        if self.names_place(left, target_text) {
            return Some(Operands {
                value: right,
                place_first: true,
            });
        }
        let commutes = Self::COMMUTING_OPERATORS.contains(&operator);
        (commutes && self.names_place(right, target_text)).then_some(Operands {
            value: left,
            place_first: false,
        })
    }

    fn names_place(&mut self, operand: &TypedArgument, target_text: &str) -> bool {
        self.argument(operand, TypeContext::Known, ArgumentRole::Ordinary)
            .text()
            == target_text
    }

    fn is_steady(&self, expr: &TypedExpr) -> bool {
        match &expr.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(_) | TypedLeaf::Literal(_)) => true,
            TypedExprKind::Deref(value)
            | TypedExprKind::Struct(TypedStructExpr::Field(value, _)) => self.is_steady(value),
            TypedExprKind::Call(callee, arguments) => {
                self.reads_element(callee)
                    && arguments
                        .iter()
                        .all(|argument| self.is_steady(&argument.value))
            }
            _ => false,
        }
    }

    fn reads_element(&self, callee: &TypedExpr) -> bool {
        let TypedExprKind::Leaf(TypedLeaf::Global(binding)) = callee.kind else {
            return false;
        };
        self.definition_template(binding, &callee.ty)
            .is_some_and(|template| *template.template == *Self::ELEMENT_READ)
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
