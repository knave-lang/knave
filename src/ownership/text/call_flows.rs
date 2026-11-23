use super::access::Access;
use super::argument_flows::ArgumentFlows;
use super::call_result_reads::CallResultReads;
use super::call_target::CallTarget;
use super::callee::Callee;
use super::callee_scheme::CalleeScheme;
use super::compound_queries::CompoundQueries;
use super::function_value::FunctionValue;
use super::instantiation::Instantiation;
use super::node_id::NodeId;
use super::scheme_position::SchemePosition;
use super::short_lived_text::ShortLivedText;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::source::Source;
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
        if let Type::Function(_, parameters, _) = &callee.ty {
            CompoundQueries::new(self.context, self.call).check(parameters, self.values);
        }
        let Some(target) = Callee::new(self.context.program()).of(callee) else {
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

    pub fn function_value(mut self, callee: &TypedExpr, owner: SlotOwner) -> TextValue {
        let target = FunctionValue::new(CalleeScheme::new(&callee.ty, &[]), Slot::from(owner));
        match &callee.ty {
            Type::Function(_, parameters, result) => self.through(&target, (parameters, result)),
            _ => self.unknown_result(),
        }
    }

    fn unknown_result(&self) -> TextValue {
        ShortLivedText::of(self.call)
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
            ArgumentFlows::new(self.context, self.call, self.values).flow(
                target,
                Instantiation::new(declared, instantiated),
                index,
            );
        }
        self.result(target, Instantiation::new(declared_result, result))
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
                Transfer::referring(access),
            ),
            None => target.result().map_or_else(TextValue::default, |slot| {
                let read = TextRead {
                    ty,
                    span: self.call.span,
                    transfer: Transfer::Move,
                };
                let read = self.context.read(&slot, read);
                CallResultReads::new(self.context, self.call).read(read, (target, self.values))
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
            _ => {
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
        self.context.read(&Slot::from(owner.clone()), read)
    }

    fn pour(&mut self, slot: &Slot, value: &TextValue) {
        self.context.receive(slot, value);
    }
}
