use super::access::Access;
use super::call_target::CallTarget;
use super::instantiation::Instantiation;
use super::need::Need;
use super::node_id::NodeId;
use super::place_writes::PlaceWrites;
use super::representation::Representation;
use super::scheme_position::SchemePosition;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_position::TextPositions;
use super::text_value::TextValue;
use super::transfer::Transfer;
use super::type_path::TypePath;
use super::value_parts::ValueParts;
use super::walk_context::WalkContext;
use crate::types::{GenericId, Type, TypedExpr, TypedExprKind};

pub struct ArgumentFlows<'c, 'w, 'p> {
    context: &'c mut WalkContext<'w, 'p>,
    call: &'c TypedExpr,
    values: &'c [TextValue],
}

impl<'c, 'w, 'p> ArgumentFlows<'c, 'w, 'p> {
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

    pub fn flow(
        mut self,
        target: &dyn CallTarget<'_>,
        instantiation: Instantiation<'_>,
        index: usize,
    ) {
        let values = self.values;
        let value = &values[index];
        if let Some(whole) = instantiation.whole_generic() {
            self.whole_generic_argument(target, whole, value);
        } else if let Some(parameter) = target.parameter(index) {
            self.hand_to_parameter(&parameter, (instantiation.instantiated(), index));
        }
        for position in instantiation.positions() {
            self.argument_position(target, &position, value);
        }
    }

    fn hand_to_parameter(&mut self, parameter: &Slot, (ty, index): (&Type, usize)) {
        let values = self.values;
        let value = &values[index];
        if matches!(ty, Type::Mut(..)) || self.passes_mutably(index) {
            self.share_place(parameter, value);
            return;
        }
        let elements = TextPositions::elements(ty);
        let mut shared = ValueParts::of(value).top();
        for (path, node) in value.nested() {
            if elements.contains(path) {
                let element = TextValue::from(Source::Slot(*node), self.call.span);
                self.pour(&parameter.nested(path), &element);
            } else {
                shared = shared.nesting(path.clone(), *node);
            }
        }
        self.pour(parameter, &shared);
    }

    fn share_place(&mut self, parameter: &Slot, value: &TextValue) {
        self.pour(parameter, value);
        let parameter = self.context.node(parameter.clone());
        for (source, _, _) in value.sources() {
            if let Source::Slot(place) = source {
                self.context.unite(parameter, *place);
            }
        }
    }

    fn passes_mutably(&self, index: usize) -> bool {
        let TypedExprKind::Call(_, arguments) = &self.call.kind else {
            return false;
        };
        arguments
            .get(index)
            .is_some_and(|argument| matches!(argument.value.ty, Type::Mut(..)))
    }

    fn whole_generic_argument(
        &mut self,
        target: &dyn CallTarget<'_>,
        (generic, access): (GenericId, Access),
        value: &TextValue,
    ) {
        let variable = Slot::from(SlotOwner::Variable(self.call.span, generic));
        let traits = self.context.program().traits();
        if !value.sources().is_empty() && target.scheme().needs_owned_text(generic, traits) {
            self.context.require(variable.clone(), Need::OwnedOrMixed);
        }
        match access {
            Access::Mutable => {
                PlaceWrites::new(self.context, self.call).through_variable(variable, value)
            }
            Access::Direct | Access::Referenced => self.pour(
                &variable,
                &value.clone().transferred(Transfer::lending(access)),
            ),
        }
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
                    let variable = self.variable_node(*generic, inner);
                    self.context.unite(variable, argument);
                }
            }
            SchemePosition::Written { written, path } if path.is_top() => {
                self.top_written_argument(target, *written, value)
            }
            SchemePosition::Written { .. } => {}
        }
    }

    fn top_written_argument(
        &mut self,
        target: &dyn CallTarget<'_>,
        written: Representation,
        value: &TextValue,
    ) {
        match written {
            Representation::Owned => self.pour_owned_argument(target, value),
            Representation::Mutable if target.changes_in_place() => {
                PlaceWrites::new(self.context, self.call).in_place(value)
            }
            _ => {}
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

    fn variable_node(&mut self, generic: GenericId, inner: &TypePath) -> NodeId {
        let slot = Slot::new(SlotOwner::Variable(self.call.span, generic), inner.clone());
        self.context.node(slot)
    }

    fn pour(&mut self, slot: &Slot, value: &TextValue) {
        self.context.receive(slot, value);
    }
}
