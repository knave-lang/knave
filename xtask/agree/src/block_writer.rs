use crate::generator_state::GeneratorState;

pub trait BlockWriter {
    fn block(
        &self,
        state: &mut GeneratorState,
        loop_variable: Option<&'static str>,
    ) -> Option<Vec<String>>;
}
