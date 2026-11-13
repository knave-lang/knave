use super::point_id::PointId;
use crate::types::TypedBinder;

#[derive(Default)]
pub struct LoopHeads<'w> {
    heads: Vec<LoopHead<'w>>,
}

#[derive(Clone, Copy)]
pub struct LoopHead<'w> {
    pub head: PointId,
    pub parameters: &'w [TypedBinder],
}

impl<'w> LoopHeads<'w> {
    pub fn enter(&mut self, head: LoopHead<'w>) {
        self.heads.push(head);
    }

    pub fn leave(&mut self) {
        self.heads.pop();
    }

    pub fn innermost(&self) -> Option<LoopHead<'w>> {
        self.heads.last().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::{LoopHead, LoopHeads};
    use crate::ownership::loans::point_id::PointId;

    #[test]
    fn the_innermost_loop_is_the_last_one_entered_and_not_yet_left() {
        let mut loops = LoopHeads::default();
        let outer = PointId::new(1);
        let inner = PointId::new(2);
        loops.enter(LoopHead {
            head: outer,
            parameters: &[],
        });
        loops.enter(LoopHead {
            head: inner,
            parameters: &[],
        });
        assert_eq!(loops.innermost().map(|head| head.head), Some(inner));
        loops.leave();
        assert_eq!(loops.innermost().map(|head| head.head), Some(outer));
        loops.leave();
        assert!(loops.innermost().is_none());
    }
}
