use std::collections::BTreeMap;

use crate::generated_main::GeneratedMain;
use crate::helper_template::HelperTemplate;
use crate::local_scopes::LocalScopes;
use crate::seed::Seed;
use crate::seeded_random::SeededRandom;
use crate::shape::Shape;
use crate::value_kind::ValueKind;

pub struct GeneratorState {
    random: SeededRandom,
    locals: LocalScopes,
    helpers: BTreeMap<&'static str, &'static HelperTemplate>,
}

impl From<Seed> for GeneratorState {
    fn from(seed: Seed) -> Self {
        Self {
            random: seed.random(),
            locals: LocalScopes::new(),
            helpers: BTreeMap::new(),
        }
    }
}

impl GeneratorState {
    const DEEPEST_BLOCK: usize = 2;

    pub fn chance(&mut self, percent: u64) -> bool {
        self.random.chance(percent)
    }

    pub fn pick<T: Copy>(&mut self, items: &[T]) -> Option<T> {
        self.random.pick(items)
    }

    pub fn some(&mut self, kind: ValueKind) -> Option<&'static str> {
        let visible = self.locals.visible(kind);
        self.random.pick(&visible)
    }

    pub fn bind_local(&mut self, kind: ValueKind, value: String) -> String {
        let name = self.random.pick(kind.names()).expect("each kind has names");
        self.locals.bind(name, kind);
        format!("(local {name} {value})")
    }

    pub fn in_block<T>(
        &mut self,
        loop_variable: Option<&'static str>,
        write: impl FnOnce(&mut Self) -> T,
    ) -> Option<T> {
        if self.locals.depth() >= Self::DEEPEST_BLOCK {
            return None;
        }
        self.locals.open();
        if let Some(variable) = loop_variable {
            self.locals.bind(variable, ValueKind::NUMBER);
        }
        let written = write(self);
        self.locals.close();
        Some(written)
    }

    pub fn use_helper(&mut self, template: &'static HelperTemplate) {
        if self.helpers.insert(template.name, template).is_some() {
            return;
        }
        if let Some(callee) = template.calls {
            self.use_helper(callee);
        }
    }

    pub fn closing_main(self, mut lines: Vec<String>) -> GeneratedMain {
        for kind in ValueKind::ALL {
            lines.extend(
                self.locals
                    .visible(kind)
                    .into_iter()
                    .filter_map(|name| kind.printed(name))
                    .map(|value| Shape::PRINTED.filled(&value)),
            );
        }
        GeneratedMain::new(lines, self.helpers.into_values().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::GeneratorState;
    use crate::seed::Seed;
    use crate::value_kind::ValueKind;

    #[test]
    fn refuses_a_block_past_the_deepest_and_closes_each_block_it_opens() {
        let mut state = GeneratorState::from(Seed::from(1));
        let nested = state.in_block(Some("i"), |outer| {
            outer.in_block(None, |inner| inner.in_block(None, |_| ()))
        });
        assert_eq!(nested, Some(Some(None)));
        assert_eq!(state.some(ValueKind::NUMBER), None);
    }
}
