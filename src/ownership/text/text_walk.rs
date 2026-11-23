use super::body_flows::BodyFlows;
use super::enum_payloads::EnumPayloads;
use super::flow_graph::FlowGraph;
use super::need::Need;
use super::slot::Slot;
use super::slot_listing::SlotListing;
use super::slot_owner::SlotOwner;
use super::text_position::{TextPosition, TextPositions};
use super::text_program::TextProgram;
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
                if !case.bare_text_fields.contains(&field) {
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
        let context = WalkContext::new(self.program, &mut self.graph);
        BodyFlows::new(context, &mut self.listing, binding).walk_value(value);
    }

    fn walk_function(&mut self, binding: BindingId, lambda: &TypedLambda) {
        let names = self.program.names();
        let heading = match self.program.written().trait_method(binding) {
            Some(_) => Some(format!("method {}", names.name(binding))),
            None => (!names.is_generated(binding)).then(|| format!("defn {}", names.name(binding))),
        };
        self.listing.open(heading);
        let context = WalkContext::new(self.program, &mut self.graph);
        BodyFlows::new(context, &mut self.listing, binding).walk(lambda);
        self.share_trait_method(binding, lambda);
    }

    fn share_trait_method(&mut self, binding: BindingId, lambda: &TypedLambda) {
        let declared = self.program.written().trait_method(binding);
        let definition = self.program.definition(binding);
        if let (Some(declared), Some(definition)) = (declared, definition)
            && let Type::Function(_, _, result) = &definition.value.ty
        {
            let mut context = WalkContext::new(self.program, &mut self.graph);
            TraitMethodSlots::new(&mut context, declared).share(binding, (lambda, result));
        }
    }
}
