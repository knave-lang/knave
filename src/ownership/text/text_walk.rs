use super::body_flows::BodyFlows;
use super::flow_graph::FlowGraph;
use super::need::Need;
use super::slot::Slot;
use super::slot_listing::SlotListing;
use super::slot_owner::SlotOwner;
use super::text_position::{TextPosition, TextPositions};
use super::text_program::TextProgram;
use super::walk_context::WalkContext;
use crate::core::BindingId;
use crate::types::{Type, TypedExprKind, TypedLambda, TypedModule};

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
        for definition in &module.definitions {
            if let TypedExprKind::Fn(lambda) = &definition.value.kind {
                self.walk_function(definition.binding, lambda);
            }
        }
    }

    fn list_struct(&mut self, binding: BindingId) {
        let Some((_, structure)) = self.program.structure_of(&Type::Named(binding, Vec::new()))
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

    fn keep_written_view(&mut self, field: &SlotOwner, ty: &Type) {
        let views = TextPositions::of(ty)
            .into_iter()
            .filter(TextPosition::is_written_reference);
        for position in views {
            self.graph
                .require(Slot::new(field.clone(), position.path), Need::View);
        }
    }

    fn walk_function(&mut self, binding: BindingId, lambda: &TypedLambda) {
        let names = self.program.names();
        let heading =
            (!names.is_generated(binding)).then(|| format!("defn {}", names.name(binding)));
        self.listing.open(heading);
        let context = WalkContext::new(self.program, &mut self.graph);
        BodyFlows::new(context, &mut self.listing, binding).walk(lambda);
    }
}
