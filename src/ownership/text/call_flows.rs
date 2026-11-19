use super::access::Access;
use super::call_target::CallTarget;
use super::callee::Callee;
use super::instantiation::Instantiation;
use super::need::Need;
use super::node_id::NodeId;
use super::reaching_text::ReachingText;
use super::representation::Representation;
use super::scheme_position::SchemePosition;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_position::TextPositions;
use super::text_read::TextRead;
use super::text_value::TextValue;
use super::transfer::Transfer;
use super::type_path::TypePath;
use super::walk_context::WalkContext;
use crate::types::{GenericId, Type, TypedExpr};

pub struct CallFlows<'c, 'w, 'p> {
    context: &'c mut WalkContext<'w, 'p>,
    call: &'c TypedExpr,
    values: &'c [TextValue],
}

impl<'c, 'w, 'p> CallFlows<'c, 'w, 'p> {
    pub fn new(
        context: &'c mut WalkContext<'w, 'p>,
        call: &'c TypedExpr,
        values: &'c [TextValue],
    ) -> Self {
        Self {
            context,
            call,
            values,
        }
    }

    pub fn value(mut self, callee: &TypedExpr) -> TextValue {
        let Some(target) = Callee::of(callee, self.context.program()) else {
            return self.unknown_result();
        };
        if let Some(value) = target.inline_value(self.context, self.call, self.values) {
            return value;
        }
        match &callee.ty {
            Type::Function(_, parameters, result) => {
                self.through(target.as_ref(), (parameters, result))
            }
            _ => self.unknown_result(),
        }
    }

    fn unknown_result(&self) -> TextValue {
        if TextPositions::holds_text(&self.call.ty) {
            TextValue::from(Source::Text(ReachingText::BORROWED), self.call.span)
        } else {
            TextValue::default()
        }
    }

    fn through(
        &mut self,
        target: &dyn CallTarget<'_>,
        (parameters, result): (&[Type], &Type),
    ) -> TextValue {
        let Some((declared_parameters, declared_result)) = target.scheme().signature() else {
            return TextValue::default();
        };
        if let Some(converted) = target.converted(self.values) {
            return converted;
        }
        let arguments = declared_parameters.iter().zip(parameters).zip(self.values);
        for (index, ((declared, instantiated), _)) in arguments.enumerate() {
            self.argument(target, Instantiation::new(declared, instantiated), index);
        }
        self.result(target, Instantiation::new(declared_result, result))
    }

    fn argument(
        &mut self,
        target: &dyn CallTarget<'_>,
        instantiation: Instantiation<'_>,
        index: usize,
    ) {
        let values = self.values;
        let value = &values[index];
        if let Some(whole) = instantiation.whole_generic() {
            self.whole_generic_argument(target, whole, value);
        } else if let Some(parameter) = target.parameter(index) {
            self.pour(&Slot::from(parameter), value);
        }
        for position in instantiation.positions() {
            self.argument_position(target, &position, value);
        }
    }

    fn whole_generic_argument(
        &mut self,
        target: &dyn CallTarget<'_>,
        (generic, access): (GenericId, Access),
        value: &TextValue,
    ) {
        let variable = Slot::from(SlotOwner::Variable(self.call.span, generic));
        let program = self.context.program();
        if !value.sources().is_empty() && target.scheme().needs_owned_text(generic, program) {
            self.context.require(variable.clone(), Need::OwnedOrMixed);
        }
        self.pour(
            &variable,
            &value.clone().transferred(Transfer::through(access)),
        );
    }

    fn argument_position(
        &mut self,
        target: &dyn CallTarget<'_>,
        position: &SchemePosition,
        value: &TextValue,
    ) {
        match position {
            SchemePosition::Generic { outer, .. } if outer.is_top() => {}
            SchemePosition::Generic { generic, inner, .. } => {
                if let Some(argument) = Self::nested_at(value, &position.path()) {
                    self.unite_with_variable(*generic, inner, argument);
                }
            }
            SchemePosition::Written {
                written: Representation::Owned,
                path,
            } if path.is_top() => self.pour_owned_argument(target, value),
            SchemePosition::Written { .. } => {}
        }
    }

    fn pour_owned_argument(&mut self, target: &dyn CallTarget<'_>, value: &TextValue) {
        if let Some(owner) = target.owned_argument(self.call.span) {
            let argument = Slot::from(owner);
            self.pour(&argument, value);
            self.context.require(argument, Need::Owned);
        }
    }

    fn nested_at(value: &TextValue, path: &TypePath) -> Option<NodeId> {
        value
            .nested()
            .iter()
            .find(|(nested, _)| nested == path)
            .map(|(_, node)| *node)
    }

    fn unite_with_variable(&mut self, generic: GenericId, inner: &TypePath, argument: NodeId) {
        let slot = Slot::new(SlotOwner::Variable(self.call.span, generic), inner.clone());
        let variable = self.context.node(slot);
        self.context.unite(variable, argument);
    }

    fn result(
        &mut self,
        target: &dyn CallTarget<'_>,
        instantiation: Instantiation<'_>,
    ) -> TextValue {
        let ty = instantiation.instantiated();
        let value = match instantiation.whole_generic() {
            Some((generic, access)) => self.read(
                &SlotOwner::Variable(self.call.span, generic),
                ty,
                Transfer::through(access),
            ),
            None => target.result().map_or_else(TextValue::default, |owner| {
                self.read(&owner, ty, Transfer::Move)
            }),
        };
        instantiation
            .positions()
            .iter()
            .fold(value, |value, position| {
                self.result_position(target, position, value)
            })
    }

    fn result_position(
        &mut self,
        target: &dyn CallTarget<'_>,
        position: &SchemePosition,
        value: TextValue,
    ) -> TextValue {
        match position {
            SchemePosition::Generic { outer, .. } if outer.is_top() => value,
            SchemePosition::Generic {
                generic,
                inner,
                access,
                ..
            } => {
                let nested = self.nested_result(*generic, inner, (&position.path(), *access));
                value.nesting(position.path(), nested)
            }
            SchemePosition::Written { written, path } => {
                match target.made(*written, self.values, self.call.span) {
                    Some(made) => self.placed_result(path, made, value),
                    None => value,
                }
            }
        }
    }

    fn nested_result(
        &mut self,
        generic: GenericId,
        inner: &TypePath,
        (path, access): (&TypePath, Access),
    ) -> NodeId {
        let variable = Slot::new(SlotOwner::Variable(self.call.span, generic), inner.clone());
        let node = self.context.node(variable);
        match access {
            Access::Direct => node,
            Access::Referenced => {
                let read = Slot::new(SlotOwner::Value(self.call.span), path.clone());
                let reference = TextValue::from(Source::Slot(node), self.call.span)
                    .transferred(Transfer::Refer);
                self.context.receive(&read, &reference);
                self.context.node(read)
            }
        }
    }

    fn placed_result(&mut self, path: &TypePath, made: TextValue, value: TextValue) -> TextValue {
        if path.is_top() {
            return value.merged(made);
        }
        let slot = Slot::new(SlotOwner::Value(self.call.span), path.clone());
        self.pour(&slot, &made);
        let node = self.context.node(slot);
        value.nesting(path.clone(), node)
    }

    fn read(&mut self, owner: &SlotOwner, ty: &Type, transfer: Transfer) -> TextValue {
        let read = TextRead {
            ty,
            span: self.call.span,
            transfer,
        };
        self.context.read(owner, read)
    }

    fn pour(&mut self, slot: &Slot, value: &TextValue) {
        self.context.receive(slot, value);
    }
}
