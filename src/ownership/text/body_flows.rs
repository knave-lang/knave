use super::assoc_flows::AssocFlows;
use super::binding_owners::BindingOwners;
use super::call_flows::CallFlows;
use super::callee::Callee;
use super::closure_slots::{ClosureSlots, ClosureUse};
use super::construction_flows::ConstructionFlows;
use super::global_reads::GlobalReads;
use super::need::Need;
use super::pattern_flows::PatternFlows;
use super::place_writes::PlaceWrites;
use super::reaching_text::ReachingText;
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
    owners: BindingOwners,
    closure_results: Vec<(SlotOwner, Type)>,
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
            owners: BindingOwners::default(),
            closure_results: Vec::new(),
        }
    }

    pub fn walk(mut self, lambda: &TypedLambda) {
        for parameter in &lambda.params {
            self.owners.add_parameter(parameter.binding);
            let owner = SlotOwner::Parameter(parameter.binding);
            self.listing.list(&owner, &parameter.ty);
            if parameter.written_view {
                self.context.require(Slot::from(owner), Need::View);
            }
        }
        let value = self.block(&lambda.body);
        if let Some(result) = self.result_type().filter(TextPositions::holds_text) {
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
            TypedExprKind::Fn(lambda) => self.closure(lambda, expr, ClosureUse::Free),
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
        let fixed = Callee::fixes_closures(callee, self.context.program());
        let values: Vec<TextValue> = arguments
            .iter()
            .map(|argument| {
                let value = match &argument.value.kind {
                    TypedExprKind::Fn(lambda) => self.closure(lambda, &argument.value, fixed),
                    _ => self.value(&argument.value),
                };
                value.transferred(Transfer::passing(&argument.borrow))
            })
            .collect();
        let function_value = self.function_value(callee);
        for (argument, value) in arguments.iter().zip(&values) {
            if self.owners.writes_a_parameter(&argument.value) {
                PlaceWrites::new(&mut self.context, expr).into_parameter(value);
            }
        }
        let flows = CallFlows::new(&mut self.context, expr, &values);
        match function_value {
            Some(owner) => flows.function_value(callee, owner),
            None => flows.value(callee),
        }
    }

    fn function_value(&mut self, callee: &TypedExpr) -> Option<SlotOwner> {
        match &callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => Some(self.owners.owner(*binding)),
            TypedExprKind::Leaf(_) => None,
            _ => {
                let value = self.value(callee);
                let owner = SlotOwner::Value(callee.span);
                self.context.receive(&Slot::from(owner.clone()), &value);
                Some(owner)
            }
        }
    }

    fn result_type(&self) -> Option<Type> {
        let definition = self.context.program().definition(self.function)?;
        match &definition.value.ty {
            Type::Function(_, _, result) => Some(result.as_ref().clone()),
            _ => None,
        }
    }

    fn leaf(&mut self, leaf: &TypedLeaf, expr: &TypedExpr) -> TextValue {
        match leaf {
            TypedLeaf::Literal(Literal::String(_)) => {
                TextValue::from(Source::Text(ReachingText::STATIC), expr.span)
            }
            TypedLeaf::Local(binding) if TextPositions::holds_text(&expr.ty) => {
                let owner = Slot::from(self.owners.owner(*binding));
                self.context
                    .read(&owner, TextRead::of(expr, Transfer::Move))
            }
            TypedLeaf::Global(binding) => GlobalReads::new(&mut self.context, expr).value(*binding),
            _ => TextValue::default(),
        }
    }

    fn closure(
        &mut self,
        lambda: &TypedLambda,
        expr: &TypedExpr,
        closure_use: ClosureUse,
    ) -> TextValue {
        let slots = ClosureSlots::new(lambda, expr, closure_use);
        slots.list(self.listing);
        self.owners.add_closure_parameters(slots.free_parameters());
        self.closure_results
            .push((slots.result(), slots.result_type().clone()));
        let body = self.block(&lambda.body);
        self.closure_results.pop();
        slots.close(&mut self.context, &body)
    }

    fn recur<'e>(&mut self, arguments: impl Iterator<Item = &'e TypedExpr>) -> TextValue {
        let parameters = self.owners.parameters().to_vec();
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
        let (returned, returned_type) = match self.closure_results.last() {
            Some(closure) => closure.clone(),
            None => (
                SlotOwner::Result(self.function),
                self.result_type().unwrap_or_else(Type::unit),
            ),
        };
        let mut flows = TryFlows::new(&mut self.context, expr.span);
        match conversion {
            TryErrorConversion::Identity => flows.propagate(&tried, &returned),
            TryErrorConversion::Converted => flows.convert(&tried, (&value.ty, &returned_type)),
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
