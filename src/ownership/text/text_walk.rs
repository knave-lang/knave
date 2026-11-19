use super::body_flows::BodyFlows;
use super::flow_graph::FlowGraph;
use super::representation::Representation;
use super::slot::{Slot, SlotOwner};
use super::slot_listing::SlotListing;
use super::text_position::TextPositions;
use super::text_program::TextProgram;
use super::walk_context::WalkContext;
use crate::core::BindingId;
use crate::types::{Type, TypedExprKind, TypedLambda, TypedModule};

pub struct TextWalk<'w, 'p> {
    context: WalkContext<'w, 'p>,
}

impl<'w, 'p> TextWalk<'w, 'p> {
    pub fn of(program: &'w TextProgram<'p>) -> (FlowGraph, SlotListing) {
        let mut walk = Self {
            context: WalkContext::new(program),
        };
        for module in program.written_modules() {
            walk.walk_module(module);
        }
        walk.context.finished()
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
        let Some((_, structure)) = self
            .context
            .program()
            .structure_of(&Type::Named(binding, Vec::new()))
        else {
            return;
        };
        let heading = format!("struct {}", self.context.program().names().name(binding));
        self.context.listing().open(Some(heading));
        for (name, ty) in &structure.fields {
            let field = SlotOwner::Field(binding, name.clone());
            self.context.listing().list(&field, ty);
            if !structure.bare_text_fields.contains(name) {
                self.keep_written_view(&field, ty);
            }
        }
    }

    fn keep_written_view(&mut self, field: &SlotOwner, ty: &Type) {
        let SlotOwner::Field(binding, name) = field else {
            return;
        };
        let views = TextPositions::of(ty)
            .into_iter()
            .filter(|position| position.written == Representation::View);
        for position in views {
            let slot = self
                .context
                .graph()
                .node(Slot::new(field.clone(), position.path.clone()));
            let view = SlotOwner::WrittenView(*binding, name.clone());
            let marker = self.context.graph().node(Slot::new(view, position.path));
            self.context.graph().unite(slot, marker);
        }
    }

    fn walk_function(&mut self, binding: BindingId, lambda: &TypedLambda) {
        let heading = (!self.context.program().names().is_generated(binding))
            .then(|| format!("defn {}", self.context.program().names().name(binding)));
        self.context.listing().open(heading);
        BodyFlows::new(&mut self.context, binding).walk(lambda);
    }
}
