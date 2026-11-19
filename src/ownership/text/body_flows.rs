use rustc_hash::FxHashSet;

use super::call_flows::CallFlows;
use super::reaching_text::ReachingText;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::slot_reads::SlotReads;
use super::source::Source;
use super::text_position::TextPositions;
use super::text_value::TextValue;
use super::transfer::Transfer;
use super::type_path::TypePath;
use super::walk_context::WalkContext;
use crate::core::{BindingId, FieldLabel, Literal, ReferenceKind};
use crate::types::{
    FieldOrder, Type, TypedArgument, TypedBlock, TypedBodyItem, TypedControl, TypedExpr,
    TypedExprKind, TypedLambda, TypedLeaf, TypedStructExpr,
};

pub struct BodyFlows<'b, 'w, 'p> {
    walk: &'b mut WalkContext<'w, 'p>,
    function: BindingId,
    parameters: Vec<BindingId>,
    opaque: FxHashSet<BindingId>,
}

impl<'b, 'w, 'p> BodyFlows<'b, 'w, 'p> {
    pub fn new(walk: &'b mut WalkContext<'w, 'p>, function: BindingId) -> Self {
        Self {
            walk,
            function,
            parameters: Vec::new(),
            opaque: FxHashSet::default(),
        }
    }

    pub fn walk(mut self, lambda: &TypedLambda) {
        for parameter in &lambda.params {
            self.parameters.push(parameter.binding);
            self.list(&SlotOwner::Parameter(parameter.binding), &parameter.ty);
        }
        let value = self.block(&lambda.body);
        if let Some(result) = self.result_type() {
            let owner = SlotOwner::Result(self.function);
            self.list(&owner, &result);
            self.walk.graph().receive(&Slot::top(owner), &value);
        }
    }

    pub fn value(&mut self, expr: &TypedExpr) -> TextValue {
        match &expr.kind {
            TypedExprKind::Leaf(leaf) => self.leaf(leaf, expr),
            TypedExprKind::Fn(lambda) => self.closure(lambda),
            TypedExprKind::Call(callee, arguments) => self.call(expr, callee, arguments),
            TypedExprKind::Recur(arguments) => {
                self.recur(arguments.iter().map(|argument| &argument.value))
            }
            TypedExprKind::Borrow(ReferenceKind::Shared, inner) => {
                self.value(inner).transferred(Transfer::Refer)
            }
            TypedExprKind::Borrow(_, inner) | TypedExprKind::Deref(inner) => self.value(inner),
            TypedExprKind::Control(control) => self.control(control),
            TypedExprKind::Struct(structure) => self.structure(structure, expr),
            TypedExprKind::Variant(binding, payloads, order) => {
                self.variant((*binding, order), payloads, expr)
            }
            _ => self.discarded(expr.children()),
        }
    }

    fn call(
        &mut self,
        expr: &TypedExpr,
        callee: &TypedExpr,
        arguments: &[TypedArgument],
    ) -> TextValue {
        let values: Vec<TextValue> = arguments
            .iter()
            .map(|argument| self.value(&argument.value))
            .collect();
        if !matches!(callee.kind, TypedExprKind::Leaf(_)) {
            self.value(callee);
        }
        CallFlows::new(self.walk, expr).value(callee, &values)
    }

    fn result_type(&self) -> Option<Type> {
        let definition = self.walk.program().definition(self.function)?;
        match &definition.value.ty {
            Type::Function(_, _, result) if TextPositions::holds_text(result) => {
                Some(result.as_ref().clone())
            }
            _ => None,
        }
    }

    fn list(&mut self, owner: &SlotOwner, ty: &Type) {
        self.walk.listing().list(owner, ty);
    }

    fn leaf(&mut self, leaf: &TypedLeaf, expr: &TypedExpr) -> TextValue {
        match leaf {
            TypedLeaf::Literal(Literal::String(_)) => {
                TextValue::from(Source::Text(ReachingText::STATIC), expr.span)
            }
            TypedLeaf::Local(binding) => self.local(*binding, expr),
            TypedLeaf::Global(_) if TextPositions::holds_text(&expr.ty) => {
                TextValue::from(Source::Text(ReachingText::STATIC), expr.span)
            }
            _ => TextValue::default(),
        }
    }

    fn local(&mut self, binding: BindingId, expr: &TypedExpr) -> TextValue {
        if !TextPositions::holds_text(&expr.ty) {
            return TextValue::default();
        }
        if self.opaque.contains(&binding) {
            return TextValue::from(Source::Text(ReachingText::BORROWED), expr.span);
        }
        let owner = if self.parameters.contains(&binding) {
            SlotOwner::Parameter(binding)
        } else {
            SlotOwner::Local(binding)
        };
        self.slot_value(&owner, expr, Transfer::Move)
    }

    fn slot_value(&mut self, owner: &SlotOwner, expr: &TypedExpr, transfer: Transfer) -> TextValue {
        SlotReads::new(self.walk.graph()).read(owner, (&expr.ty, expr.span), transfer)
    }

    fn closure(&mut self, lambda: &TypedLambda) -> TextValue {
        self.opaque
            .extend(lambda.params.iter().map(|parameter| parameter.binding));
        self.block(&lambda.body);
        TextValue::default()
    }

    fn recur<'e>(&mut self, arguments: impl Iterator<Item = &'e TypedExpr>) -> TextValue {
        let parameters = self.parameters.clone();
        for (parameter, argument) in parameters.into_iter().zip(arguments) {
            let value = self.value(argument);
            self.walk
                .graph()
                .receive(&Slot::top(SlotOwner::Parameter(parameter)), &value);
        }
        TextValue::default()
    }

    fn block(&mut self, block: &TypedBlock) -> TextValue {
        let mut last = TextValue::default();
        for item in &block.items {
            last = match item {
                TypedBodyItem::Let(binder, init) => self.bind(binder.binding, &binder.ty, init),
                TypedBodyItem::Expr(expr) => self.value(expr),
            };
        }
        last
    }

    fn bind(&mut self, binding: BindingId, ty: &Type, init: &TypedExpr) -> TextValue {
        let value = self.value(init);
        if TextPositions::holds_text(ty) {
            let owner = SlotOwner::Local(binding);
            self.list(&owner, ty);
            self.walk.graph().receive(&Slot::top(owner), &value);
        }
        TextValue::default()
    }

    fn control(&mut self, control: &TypedControl) -> TextValue {
        match control {
            TypedControl::Block(block) => self.block(block),
            TypedControl::If(test, then, otherwise) => {
                self.value(test);
                let then = self.value(then);
                then.merged(self.value(otherwise))
            }
            TypedControl::Match(matched) => {
                self.value(&matched.value);
                matched
                    .arms
                    .iter()
                    .fold(TextValue::default(), |value, arm| {
                        self.opaque
                            .extend(arm.pattern.binders().iter().map(|binder| binder.binding));
                        value.merged(self.block(&arm.body))
                    })
            }
        }
    }

    fn structure(&mut self, structure: &TypedStructExpr, expr: &TypedExpr) -> TextValue {
        match structure {
            TypedStructExpr::New(binding, fields) => self.fields(*binding, fields),
            TypedStructExpr::Update(binding, base, fields) => {
                self.value(base);
                self.fields(*binding, fields)
            }
            TypedStructExpr::Field(owner, label) => {
                self.value(owner);
                match self.walk.program().structure_of(&owner.ty) {
                    Some((binding, _)) => {
                        let field = SlotOwner::Field(binding, label.name.clone());
                        self.slot_value(&field, expr, Transfer::Refer)
                    }
                    None => TextValue::default(),
                }
            }
        }
    }

    fn fields(&mut self, binding: BindingId, fields: &[(FieldLabel, TypedExpr)]) -> TextValue {
        for (label, value) in fields {
            let value = self.value(value);
            let owner = SlotOwner::Field(binding, label.name.clone());
            self.walk.graph().receive(&Slot::top(owner), &value);
        }
        TextValue::default()
    }

    fn variant(
        &mut self,
        (binding, order): (BindingId, &FieldOrder),
        payloads: &[TypedExpr],
        expr: &TypedExpr,
    ) -> TextValue {
        for (written, payload) in payloads.iter().enumerate() {
            let field = order.field_of(written);
            let value = self.value(payload);
            let slot = match self.walk.program().payload_argument(binding, field) {
                Some(argument) => Slot::new(
                    SlotOwner::Value(expr.span),
                    TypePath::default().child(argument),
                ),
                None => Slot::top(SlotOwner::Field(binding, field.to_string().into())),
            };
            self.walk.graph().receive(&slot, &value);
        }
        SlotReads::new(self.walk.graph()).nested(&SlotOwner::Value(expr.span), &expr.ty)
    }

    fn discarded(&mut self, children: Vec<&TypedExpr>) -> TextValue {
        for child in children {
            self.value(child);
        }
        TextValue::default()
    }
}
