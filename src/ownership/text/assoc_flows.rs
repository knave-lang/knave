use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::text_read::TextRead;
use super::text_value::TextValue;
use super::transfer::Transfer;
use super::type_path::TypePath;
use super::walk_context::WalkContext;
use crate::types::TypedExpr;

pub struct AssocFlows<'c, 'w, 'p> {
    context: &'c mut WalkContext<'w, 'p>,
    assoc: &'c TypedExpr,
}

impl<'c, 'w, 'p> AssocFlows<'c, 'w, 'p> {
    const KEY: usize = 0;
    const VALUE: usize = 1;
    const ELEMENT: usize = 0;

    pub fn new(context: &'c mut WalkContext<'w, 'p>, assoc: &'c TypedExpr) -> Self {
        Self { context, assoc }
    }

    pub fn value(self, base: &TextValue, (keys, item): (&[TextValue], &TextValue)) -> TextValue {
        let result = SlotOwner::Value(self.assoc.span);
        self.context.receive(&Slot::from(result.clone()), base);
        for (path, placed) in self.placements(keys, item) {
            self.context
                .receive(&Slot::new(result.clone(), path), placed);
        }
        let read = TextRead {
            ty: &self.assoc.ty,
            span: self.assoc.span,
            transfer: Transfer::Move,
        };
        self.context.read(&result, read)
    }

    fn placements<'v>(
        &self,
        keys: &'v [TextValue],
        item: &'v TextValue,
    ) -> Vec<(TypePath, &'v TextValue)> {
        let top = TypePath::default();
        match keys {
            [key] if self.is_map() => {
                vec![(top.child(Self::KEY), key), (top.child(Self::VALUE), item)]
            }
            _ => {
                let element = keys.iter().fold(top, |path, _| path.child(Self::ELEMENT));
                vec![(element, item)]
            }
        }
    }

    fn is_map(&self) -> bool {
        self.context
            .program()
            .types()
            .collection_of(&self.assoc.ty)
            .is_some_and(|collection| collection.is_map())
    }
}
