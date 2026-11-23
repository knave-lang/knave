use super::map_queries::MapQueries;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_value::TextValue;
use super::transfer::Transfer;
use super::type_path::TypePath;
use super::walk_context::WalkContext;
use crate::types::{Type, TypedExpr, TypedExprKind};

pub struct CompoundQueries<'c, 'w, 'p> {
    context: &'c mut WalkContext<'w, 'p>,
    call: &'c TypedExpr,
}

impl<'c, 'w, 'p> CompoundQueries<'c, 'w, 'p> {
    const KEY: usize = 0;

    pub fn new(context: &'c mut WalkContext<'w, 'p>, call: &'c TypedExpr) -> Self {
        Self { context, call }
    }

    pub fn check(self, parameters: &[Type], values: &[TextValue]) {
        let types = self.context.program().types();
        let Some((map, query)) = MapQueries::new(types).pair(parameters) else {
            return;
        };
        let span = match &self.call.kind {
            TypedExprKind::Call(_, arguments) => arguments
                .get(query)
                .map_or(self.call.span, |argument| argument.value.span),
            _ => self.call.span,
        };
        let (Some(map), Some(query)) = (values.get(map), values.get(query)) else {
            return;
        };
        for (path, queried) in query.nested() {
            let key_path = TypePath::default().child(Self::KEY).joined(path);
            let key = map
                .nested()
                .iter()
                .find(|(nested, _)| *nested == key_path)
                .map(|(_, node)| *node);
            if let Some(key) = key {
                let probe = Slot::new(SlotOwner::Query(self.call.span), path.clone());
                let probe_node = self.context.node(probe.clone());
                self.context.unite(probe_node, key);
                let asked =
                    TextValue::from(Source::Slot(*queried), span).transferred(Transfer::Query);
                self.context.receive(&probe, &asked);
            }
        }
    }
}
