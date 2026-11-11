use std::rc::Rc;

use super::name_records::NameRecords;
use super::set_aside_drop::SetAsideDrop;
use crate::diagnostic::Diagnostic;
use crate::expand::{BoundName, InputTerms, SetAsideInput, SetAsideProblems};

pub struct DroppedDefinitions {
    source: String,
    dropped: Vec<BoundName>,
    replacing: Rc<str>,
}

impl DroppedDefinitions {
    pub fn left_behind(source: &str, earlier: &InputTerms, entered: &InputTerms) -> Option<Self> {
        let dropped: Vec<BoundName> = earlier
            .bound()
            .iter()
            .filter(|name| !entered.binds(name))
            .cloned()
            .collect();
        let replacing = entered.bound().iter().find(|bound| earlier.binds(bound))?;
        (!dropped.is_empty()).then(|| Self {
            source: source.to_string(),
            dropped,
            replacing: Rc::clone(&replacing.name),
        })
    }
}

impl SetAsideDrop for DroppedDefinitions {
    fn warning(&self, setter: &str) -> Diagnostic {
        let names: Vec<&str> = self.dropped.iter().map(|name| &*name.name).collect();
        let input = SetAsideInput {
            names: &names,
            source: &self.source,
            setter,
        };
        SetAsideProblems::dropped_definitions(&input, &self.replacing, self.dropped[0].span)
    }

    fn record(&self, records: &mut NameRecords, setter: &str) {
        for name in &self.dropped {
            let reason = SetAsideProblems::set_aside_reason(&self.replacing, &name.name);
            records.set_aside(name, setter, &reason);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::name_records::Tense;
    use crate::core::Namespace;
    use crate::diagnostic::MissingName;
    use crate::source::{SourceId, Span};

    struct DefinitionsFixture;

    impl DefinitionsFixture {
        const SOURCE: &str = "<input 2>";
        const SETTER: &str = "<input 3>";

        fn name(name: &str) -> BoundName {
            BoundName {
                name: name.into(),
                namespace: Namespace::Value,
                span: Span::new(SourceId::new(0), 0..1),
            }
        }

        fn terms(names: &[&str]) -> InputTerms {
            InputTerms::binding(names.iter().map(|name| Self::name(name)).collect())
        }

        fn dropped() -> DroppedDefinitions {
            DroppedDefinitions::left_behind(
                Self::SOURCE,
                &Self::terms(&["a", "b"]),
                &Self::terms(&["b"]),
            )
            .expect("`b` replaces the input that also defined `a`")
        }
    }

    #[test]
    fn a_replacement_that_keeps_every_name_leaves_nothing_behind() {
        let earlier = DefinitionsFixture::terms(&["a"]);
        let left = DroppedDefinitions::left_behind(DefinitionsFixture::SOURCE, &earlier, &earlier);
        assert!(left.is_none());
    }

    #[test]
    fn an_input_that_replaces_nothing_leaves_nothing_behind() {
        let left = DroppedDefinitions::left_behind(
            DefinitionsFixture::SOURCE,
            &DefinitionsFixture::terms(&["a"]),
            &DefinitionsFixture::terms(&["c"]),
        );
        assert!(left.is_none());
    }

    #[test]
    fn the_warning_names_the_definitions_left_behind() {
        let warning = DefinitionsFixture::dropped().warning(DefinitionsFixture::SETTER);
        assert_eq!(warning.title(), "set aside `a` after `<input 3>`");
        assert_eq!(warning.notes(), ["`b` replaced the input that defined `a`"]);
    }

    #[test]
    fn each_definition_left_behind_is_recorded_as_set_aside() {
        let mut records = NameRecords::default();
        DefinitionsFixture::dropped().record(&mut records, DefinitionsFixture::SETTER);
        let missing = MissingName {
            name: "a".into(),
            namespace: Namespace::Value,
        };
        assert_eq!(
            records.note(&missing, Tense::Done).as_deref(),
            Some("`<input 3>` set aside `a`: `b` replaced the input that defined `a`")
        );
    }
}
