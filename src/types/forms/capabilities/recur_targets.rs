use crate::types::recur_target::RecurTarget;

pub(in crate::types) trait RecurTargets {
    fn recur_target(&self) -> RecurTarget;
}
