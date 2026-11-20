use super::checker::TypeChecker;
use super::inferred_span::InferredSpan;

impl AsMut<InferredSpan> for TypeChecker {
    fn as_mut(&mut self) -> &mut InferredSpan {
        &mut self.inferring
    }
}
