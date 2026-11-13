use super::point_id::PointId;
use crate::types::TypedBinder;

#[derive(Default)]
pub struct LoopHeads<'w> {
    heads: Vec<LoopHead<'w>>,
}

#[derive(Clone, Copy)]
struct LoopHead<'w> {
    head: PointId,
    parameters: &'w [TypedBinder],
}

impl<'w> LoopHeads<'w> {
    pub fn enter(&mut self, head: PointId, parameters: &'w [TypedBinder]) {
        self.heads.push(LoopHead { head, parameters });
    }

    pub fn leave(&mut self) {
        self.heads.pop();
    }

    pub fn innermost_head(&self) -> Option<PointId> {
        self.heads.last().map(|innermost| innermost.head)
    }

    pub fn innermost_parameters(&self) -> Option<&'w [TypedBinder]> {
        self.heads.last().map(|innermost| innermost.parameters)
    }
}

#[cfg(test)]
mod tests {
    use super::LoopHeads;
    use crate::ownership::loans::point_id::PointId;

    #[test]
    fn the_innermost_loop_is_the_last_one_entered_and_not_yet_left() {
        let mut loops = LoopHeads::default();
        let outer = PointId::new(1);
        let inner = PointId::new(2);
        loops.enter(outer, &[]);
        loops.enter(inner, &[]);
        assert_eq!(loops.innermost_head(), Some(inner));
        loops.leave();
        assert_eq!(loops.innermost_head(), Some(outer));
        assert_eq!(loops.innermost_parameters().map(<[_]>::len), Some(0));
        loops.leave();
        assert!(loops.innermost_head().is_none());
        assert!(loops.innermost_parameters().is_none());
    }
}
