use super::access::Access;
use super::callee::Callee;
use super::inline_flows::InlineFlows;
use super::need::Need;
use super::node_id::NodeId;
use super::reaching_text::ReachingText;
use super::representation::Representation;
use super::scheme_position::SchemePosition;
use super::scheme_positions::SchemePositions;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_conversion::TextConversion;
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
}

impl<'c, 'w, 'p> CallFlows<'c, 'w, 'p> {
    pub fn new(context: &'c mut WalkContext<'w, 'p>, call: &'c TypedExpr) -> Self {
        Self { context, call }
    }

    pub fn value(mut self, callee: &TypedExpr, values: &[TextValue]) -> TextValue {
        let program = self.context.program();
        match (Callee::of(callee, program), &callee.ty) {
            (Some(signature), _) if signature.is_inline() && self.shares_nested_text(values) => {
                InlineFlows::new(self.context, self.call).value(values)
            }
            (Some(signature), Type::Function(_, parameters, result)) => {
                self.through(&signature, (parameters, result), values)
            }
            _ => self.unknown_result(),
        }
    }

    fn shares_nested_text(&self, values: &[TextValue]) -> bool {
        values.iter().any(|value| !value.nested().is_empty())
            || TextPositions::of(&self.call.ty)
                .iter()
                .any(|position| !position.path.is_top())
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
        callee: &Callee<'_>,
        (parameters, result): (&[Type], &Type),
        values: &[TextValue],
    ) -> TextValue {
        let Some((declared_parameters, declared_result)) = callee.signature() else {
            return TextValue::default();
        };
        if callee.function().is_none()
            && TextConversion::is_conversion(declared_parameters, declared_result)
        {
            return TextConversion::converted(&values[0]);
        }
        let pairs = declared_parameters.iter().zip(parameters).zip(values);
        for (index, ((declared, instantiated), value)) in pairs.enumerate() {
            self.argument((callee, index), (declared, instantiated), value);
        }
        self.result(callee, (declared_result, result), values)
    }

    fn argument(
        &mut self,
        (callee, index): (&Callee<'_>, usize),
        (declared, instantiated): (&Type, &Type),
        value: &TextValue,
    ) {
        if let Some((generic, access)) = Callee::whole_generic(declared) {
            self.whole_generic_argument((callee, generic), access, value);
        } else if let Some(parameter) = callee.parameter(index) {
            self.pour(&Slot::from(SlotOwner::Parameter(parameter)), value);
        }
        for position in SchemePositions::of(declared, instantiated) {
            self.argument_position(callee, &position, value);
        }
    }

    fn whole_generic_argument(
        &mut self,
        (callee, generic): (&Callee<'_>, GenericId),
        access: Access,
        value: &TextValue,
    ) {
        let variable = Slot::from(SlotOwner::Variable(self.call.span, generic));
        let program = self.context.program();
        if !value.sources().is_empty() && callee.needs_owned_text(generic, program) {
            self.context.require(variable.clone(), Need::OwnedOrMixed);
        }
        self.pour(
            &variable,
            &value.clone().transferred(Transfer::through(access)),
        );
    }

    fn argument_position(
        &mut self,
        callee: &Callee<'_>,
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
            } if path.is_top() && callee.function().is_none() => self.pour_owned_argument(value),
            SchemePosition::Written { .. } => {}
        }
    }

    fn pour_owned_argument(&mut self, value: &TextValue) {
        let argument = Slot::from(SlotOwner::Argument(self.call.span));
        self.pour(&argument, value);
        self.context.require(argument, Need::Owned);
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
        callee: &Callee<'_>,
        (declared, instantiated): (&Type, &Type),
        arguments: &[TextValue],
    ) -> TextValue {
        let mut value = match (Callee::whole_generic(declared), callee.function()) {
            (Some((generic, access)), _) => self.read(
                &SlotOwner::Variable(self.call.span, generic),
                instantiated,
                Transfer::through(access),
            ),
            (None, Some(function)) => {
                self.read(&SlotOwner::Result(function), instantiated, Transfer::Move)
            }
            (None, None) => TextValue::default(),
        };
        for position in SchemePositions::of(declared, instantiated) {
            value = self.result_position(callee, &position, (arguments, value));
        }
        value
    }

    fn result_position(
        &mut self,
        callee: &Callee<'_>,
        position: &SchemePosition,
        (arguments, value): (&[TextValue], TextValue),
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
            SchemePosition::Written { written, path } if callee.function().is_none() => {
                let made = self.made(*written, arguments);
                self.placed_result(path, made, value)
            }
            SchemePosition::Written { .. } => value,
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

    fn made(&self, written: Representation, arguments: &[TextValue]) -> TextValue {
        match written {
            Representation::Owned => {
                TextValue::from(Source::Text(ReachingText::OWNED), self.call.span)
            }
            _ => arguments
                .iter()
                .fold(TextValue::default(), |value, argument| {
                    value.merged(argument.clone().transferred(Transfer::View))
                }),
        }
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
