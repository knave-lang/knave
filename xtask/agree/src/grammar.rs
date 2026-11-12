use crate::block_writer::BlockWriter;
use crate::generator_state::GeneratorState;
use crate::production::Production;
use crate::productions::{
    Block, Branch, CallChanger, CallHelper, DerivedBinding, FreshCar, FreshNumber, FreshVector,
    Print, Push, Repeat, SetYear,
};

pub struct Grammar {
    productions: &'static [&'static dyn Production],
}

impl Grammar {
    const STATEMENTS: [&'static dyn Production; 18] = [
        &FreshVector,
        &FreshNumber,
        &FreshCar,
        &DerivedBinding::BORROW_ELEMENT,
        &DerivedBinding::BORROW_MAKE,
        &DerivedBinding::MOVE_NUMBERS,
        &DerivedBinding::BIND_TAG,
        &DerivedBinding::HOLD_BORROWS,
        &DerivedBinding::BIND_READER,
        &DerivedBinding::BIND_CHANGER,
        &Push,
        &SetYear,
        &Print,
        &CallChanger,
        &CallHelper,
        &Branch,
        &Repeat,
        &Block,
    ];
    const BLOCK_SIZES: [usize; 3] = [1, 2, 3];

    pub fn of_statements() -> Self {
        Self {
            productions: &Self::STATEMENTS,
        }
    }

    pub fn statement(&self, state: &mut GeneratorState) -> String {
        loop {
            let production = state
                .pick(self.productions)
                .expect("the grammar has productions");
            if let Some(line) = production.write(self, state) {
                return line;
            }
        }
    }

    pub fn first_vector(&self, state: &mut GeneratorState) -> String {
        FreshVector
            .write(self, state)
            .expect("a fresh vector needs nothing visible")
    }
}

impl BlockWriter for Grammar {
    fn block(
        &self,
        state: &mut GeneratorState,
        loop_variable: Option<&'static str>,
    ) -> Option<Vec<String>> {
        state.in_block(loop_variable, |inner| {
            let size = inner.pick(&Self::BLOCK_SIZES).expect("block sizes exist");
            (0..size).map(|_| self.statement(inner)).collect()
        })
    }
}
