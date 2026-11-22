use rustc_hash::FxHashSet;

use super::assoc_flows::AssocFlows;
use super::call_flows::CallFlows;
use super::construction_flows::ConstructionFlows;
use super::pattern_flows::PatternFlows;
use super::reaching_text::ReachingText;
use super::short_lived_text::ShortLivedText;
use super::slot::Slot;
use super::slot_listing::SlotListing;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_position::TextPositions;
use super::text_read::TextRead;
use super::text_value::TextValue;
use super::transfer::Transfer;
use super::try_flows::TryFlows;
use super::walk_context::WalkContext;
use crate::core::{BindingId, FieldLabel, Literal, ReferenceKind};
use crate::types::{
    FieldOrder, TryErrorConversion, Type, TypedArgument, TypedArm, TypedBlock, TypedBodyItem,
    TypedControl, TypedExpr, TypedExprKind, TypedLambda, TypedLeaf, TypedStructExpr,
};

pub struct BodyFlows<'b, 'p> {
    context: WalkContext<'b, 'p>,
    listing: &'b mut SlotListing,
    function: BindingId,
    parameters: Vec<BindingId>,
    opaque: FxHashSet<BindingId>,
    closures: usize,
}

impl<'b, 'p> BodyFlows<'b, 'p> {
    pub fn new(
        context: WalkContext<'b, 'p>,
        listing: &'b mut SlotListing,
        function: BindingId,
    ) -> Self {
        Self {
            context,
            listing,
            function,
            parameters: Vec::new(),
            opaque: FxHashSet::default(),
            closures: 0,
        }
    }

    pub fn walk(mut self, lambda: &TypedLambda) {
        for parameter in &lambda.params {
            self.parameters.push(parameter.binding);
            self.listing
                .list(&SlotOwner::Parameter(parameter.binding), &parameter.ty);
        }
        let value = self.block(&lambda.body);
        if let Some(result) = self.result_type() {
            let owner = SlotOwner::Result(self.function);
            self.listing.list(&owner, &result);
            self.context.receive(&Slot::from(owner), &value);
        }
    }

    pub fn walk_value(mut self, expr: &TypedExpr) {
        let owner = SlotOwner::Global(self.function);
        self.listing.list(&owner, &expr.ty);
        let value = self.value(expr);
        self.context.receive(&Slot::from(owner), &value);
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
            _ => self.built(expr),
        }
    }

    fn built(&mut self, expr: &TypedExpr) -> TextValue {
        match &expr.kind {
            TypedExprKind::Control(control) => self.control(control),
            TypedExprKind::Struct(structure) => self.structure(structure, expr),
            TypedExprKind::Variant(binding, payloads, order) => {
                self.variant((*binding, order), payloads, expr)
            }
            TypedExprKind::Try { value, conversion } => self.tried(value, *conversion, expr),
            TypedExprKind::Assoc(base, keys, item) => self.associated(base, (keys, item), expr),
            _ => {
                expr.children().into_iter().for_each(|child| {
                    self.value(child);
                });
                TextValue::default()
            }
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
            .map(|argument| {
                self.value(&argument.value)
                    .transferred(Transfer::passing(&argument.borrow))
            })
            .collect();
        if !matches!(callee.kind, TypedExprKind::Leaf(_)) {
            self.value(callee);
        }
        CallFlows::new(&mut self.context, expr, &values).value(callee)
    }

    fn result_type(&self) -> Option<Type> {
        let definition = self.context.program().definition(self.function)?;
        match &definition.value.ty {
            Type::Function(_, _, result) if TextPositions::holds_text(result) => {
                Some(result.as_ref().clone())
            }
            _ => None,
        }
    }

    fn leaf(&mut self, leaf: &TypedLeaf, expr: &TypedExpr) -> TextValue {
        match leaf {
            TypedLeaf::Literal(Literal::String(_)) => {
                TextValue::from(Source::Text(ReachingText::STATIC), expr.span)
            }
            TypedLeaf::Local(binding) => self.local(*binding, expr),
            TypedLeaf::Global(binding) if self.context.program().is_written_value(*binding) => {
                let read = TextRead::of(expr, Transfer::Refer);
                self.context.read(&SlotOwner::Global(*binding), read)
            }
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
            return ShortLivedText::of(expr);
        }
        let owner = if self.parameters.contains(&binding) {
            SlotOwner::Parameter(binding)
        } else {
            SlotOwner::Local(binding)
        };
        self.context
            .read(&owner, TextRead::of(expr, Transfer::Move))
    }

    fn closure(&mut self, lambda: &TypedLambda) -> TextValue {
        self.opaque
            .extend(lambda.params.iter().map(|parameter| parameter.binding));
        self.closures += 1;
        self.block(&lambda.body);
        self.closures -= 1;
        TextValue::default()
    }

    fn recur<'e>(&mut self, arguments: impl Iterator<Item = &'e TypedExpr>) -> TextValue {
        let parameters = self.parameters.clone();
        for (parameter, argument) in parameters.into_iter().zip(arguments) {
            let value = self.value(argument);
            self.context
                .receive(&Slot::from(SlotOwner::Parameter(parameter)), &value);
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
            self.listing.list(&owner, ty);
            self.context.receive(&Slot::from(owner), &value);
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
                let scrutinee = self.value(&matched.value);
                matched
                    .arms
                    .iter()
                    .fold(TextValue::default(), |value, arm| {
                        value.merged(self.arm(arm, (&scrutinee, &matched.value.ty)))
                    })
            }
        }
    }

    fn arm(&mut self, arm: &TypedArm, (scrutinee, matched): (&TextValue, &Type)) -> TextValue {
        for binder in arm.pattern.binders() {
            self.listing
                .list(&SlotOwner::Local(binder.binding), &binder.ty);
        }
        PatternFlows::new(&mut self.context, matched).bind(&arm.pattern, scrutinee);
        self.block(&arm.body)
    }

    fn structure(&mut self, structure: &TypedStructExpr, expr: &TypedExpr) -> TextValue {
        match structure {
            TypedStructExpr::New(binding, fields) => self.fields(*binding, fields, expr),
            TypedStructExpr::Update(binding, base, fields) => {
                self.value(base);
                self.fields(*binding, fields, expr)
            }
            TypedStructExpr::Field(owner, label) => {
                self.value(owner);
                ConstructionFlows::new(&mut self.context, expr).field_read(&owner.ty, label)
            }
        }
    }

    fn fields(
        &mut self,
        binding: BindingId,
        fields: &[(FieldLabel, TypedExpr)],
        expr: &TypedExpr,
    ) -> TextValue {
        let values: Vec<(&FieldLabel, TextValue)> = fields
            .iter()
            .map(|(label, value)| (label, self.value(value)))
            .collect();
        ConstructionFlows::new(&mut self.context, expr).fields(binding, &values)
    }

    fn variant(
        &mut self,
        (binding, order): (BindingId, &FieldOrder),
        payloads: &[TypedExpr],
        expr: &TypedExpr,
    ) -> TextValue {
        let values: Vec<(usize, TextValue)> = payloads
            .iter()
            .enumerate()
            .map(|(written, payload)| (order.field_of(written), self.value(payload)))
            .collect();
        ConstructionFlows::new(&mut self.context, expr).variant(binding, &values)
    }

    fn tried(
        &mut self,
        value: &TypedExpr,
        conversion: TryErrorConversion,
        expr: &TypedExpr,
    ) -> TextValue {
        let tried = self.value(value);
        let mut flows = TryFlows::new(&mut self.context, expr.span);
        if conversion == TryErrorConversion::Identity && self.closures == 0 {
            flows.propagate(&tried, &SlotOwner::Result(self.function));
        }
        flows.payload(&tried)
    }

    fn associated(
        &mut self,
        base: &TypedExpr,
        (keys, item): (&[TypedExpr], &TypedExpr),
        expr: &TypedExpr,
    ) -> TextValue {
        let base = self.value(base);
        let keys: Vec<TextValue> = keys.iter().map(|key| self.value(key)).collect();
        let item = self.value(item);
        AssocFlows::new(&mut self.context, expr).value(&base, (&keys, &item))
    }
}
