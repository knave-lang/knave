use crate::generated_main::GeneratedMain;
use crate::generator_state::GeneratorState;
use crate::grammar::Grammar;
use crate::seed::Seed;

pub struct MainBody {
    statements: usize,
}

impl MainBody {
    pub fn of_statements(statements: usize) -> Self {
        Self { statements }
    }

    pub fn write(&self, seed: Seed) -> GeneratedMain {
        let grammar = Grammar::of_statements();
        let mut state = GeneratorState::from(seed);
        let mut lines = vec![grammar.first_vector(&mut state)];
        lines.extend((0..self.statements).map(|_| grammar.statement(&mut state)));
        state.closing_main(lines)
    }
}
