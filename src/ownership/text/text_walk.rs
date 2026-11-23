use super::body_flows::BodyFlows;
use super::enum_payloads::EnumPayloads;
use super::flow_graph::FlowGraph;
use super::flow_origin::FlowOrigin;
use super::need::Need;
use super::reaching_text::ReachingText;
use super::representation::Representation;
use super::slot::Slot;
use super::slot_listing::SlotListing;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_position::{TextPosition, TextPositions};
use super::text_program::TextProgram;
use super::text_value::TextValue;
use super::trait_method_slots::TraitMethodSlots;

use super::walk_context::WalkContext;
use crate::core::BindingId;
use crate::types::{Type, TypedExpr, TypedExprKind, TypedLambda, TypedModule};

pub struct TextWalk<'w, 'p> {
    program: &'w TextProgram<'p>,
    graph: FlowGraph,
    listing: SlotListing,
}

impl<'w, 'p> TextWalk<'w, 'p> {
    pub fn of(program: &'w TextProgram<'p>) -> (FlowGraph, SlotListing) {
        let mut walk = Self {
            program,
            graph: FlowGraph::default(),
            listing: SlotListing::default(),
        };
        for module in program.written_modules() {
            walk.walk_module(module);
        }
        (walk.graph, walk.listing)
    }

    fn walk_module(&mut self, module: &TypedModule) {
        for binding in &module.structs {
            self.list_struct(*binding);
        }
        for binding in &module.enums {
            self.list_enum(*binding);
        }
        for definition in &module.definitions {
            match &definition.value.kind {
                TypedExprKind::Fn(lambda) => self.walk_function(definition.binding, lambda),
                _ if self.program.written().is_value(definition.binding) => {
                    self.walk_value(definition.binding, &definition.value)
                }
                _ => {}
            }
        }
    }

    fn list_struct(&mut self, binding: BindingId) {
        let Some((_, structure)) = self
            .program
            .types()
            .structure_of(&Type::Named(binding, Vec::new()))
        else {
            return;
        };
        let heading = format!("struct {}", self.program.names().name(binding));
        self.listing.open(Some(heading));
        for (name, ty) in &structure.fields {
            let field = SlotOwner::Field(binding, name.clone());
            self.listing.list(&field, ty);
            if !structure.bare_text_fields.contains(name) {
                self.keep_written_view(&field, ty);
            }
        }
    }

    fn list_enum(&mut self, binding: BindingId) {
        let Some(definition) = self.program.types().payloads().enumeration(binding) else {
            return;
        };
        let heading = format!("enum {}", self.program.names().name(binding));
        self.listing.open(Some(heading));
        for case in &definition.variants {
            for (field, ty) in case.payload_types().into_iter().enumerate() {
                let owner = SlotOwner::Field(binding, EnumPayloads::label(case, field));
                self.listing.list(&owner, ty);
                if !case.is_bare_text(field) {
                    self.keep_written_view(&owner, ty);
                }
            }
        }
    }

    fn keep_written_view(&mut self, field: &SlotOwner, ty: &Type) {
        let views = TextPositions::of(ty)
            .into_iter()
            .filter(TextPosition::is_written_reference);
        for position in views {
            self.graph
                .require(Slot::new(field.clone(), position.path), Need::View);
        }
    }

    fn walk_value(&mut self, binding: BindingId, value: &TypedExpr) {
        let names = self.program.names();
        self.listing
            .open(Some(format!("def {}", names.name(binding))));
        let context = WalkContext::new(self.program, &mut self.graph, FlowOrigin::Written);
        BodyFlows::new(context, &mut self.listing, binding).walk_value(value);
    }

    fn walk_function(&mut self, binding: BindingId, lambda: &TypedLambda) {
        let names = self.program.names();
        let origin = self.program.written().origin(binding);
        let heading = match self.program.written().trait_method(binding) {
            Some(_) => Some(format!("method {}", names.name(binding))),
            None => {
                (origin == FlowOrigin::Written).then(|| format!("defn {}", names.name(binding)))
            }
        };
        self.listing.open(heading);
        let context = WalkContext::new(self.program, &mut self.graph, origin);
        BodyFlows::new(context, &mut self.listing, binding).walk(lambda);
        self.share_trait_method(binding, lambda);
        if self.program.written().has_receiver(binding) {
            self.keep_receiver(lambda);
        }
    }

    fn keep_receiver(&mut self, lambda: &TypedLambda) {
        let Some(receiver) = lambda.params.first() else {
            return;
        };
        let top = TextPositions::of(&receiver.ty)
            .into_iter()
            .find(|position| position.path.is_top());
        let (need, given) = match top.map(|position| position.written) {
            Some(Representation::Owned) => (Need::Owned, ReachingText::OWNED),
            Some(Representation::View) => (Need::View, ReachingText::BORROWED),
            Some(Representation::Lent) => (Need::Lent, ReachingText::LENT),
            _ => return,
        };
        let slot = Slot::from(SlotOwner::Parameter(receiver.binding));
        let given = TextValue::from(Source::Text(given), receiver.span);
        self.graph.receive(&slot, &given, FlowOrigin::Written);
        self.graph.require(slot, need);
    }

    fn share_trait_method(&mut self, binding: BindingId, lambda: &TypedLambda) {
        let declared = self.program.written().trait_method(binding);
        let definition = self.program.definition(binding);
        if let (Some(declared), Some(definition)) = (declared, definition)
            && let Type::Function(_, _, result) = &definition.value.ty
        {
            let mut context = WalkContext::new(self.program, &mut self.graph, FlowOrigin::Written);
            TraitMethodSlots::new(&mut context, declared).share(binding, (lambda, result));
        }
    }
}
