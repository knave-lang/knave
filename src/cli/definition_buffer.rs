use super::dropped_definitions::DroppedDefinitions;
use super::dropped_documents::DroppedDocuments;
use super::set_aside_drop::SetAsideDrop;
use super::set_aside_source::SetAsideSource;
use crate::expand::{BoundName, InputTerms};
use crate::source::{SourceId, Span};

#[derive(Clone)]
pub struct BufferedInput {
    name: String,
    span: Span,
    terms: InputTerms,
}

#[derive(Clone, Default)]
pub struct DefinitionBuffer {
    inputs: Vec<BufferedInput>,
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Replacement {
    replaced: Vec<usize>,
}

impl BufferedInput {
    pub fn new(name: String, span: Span, terms: InputTerms) -> Self {
        Self { name, span, terms }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn names(&self) -> &[BoundName] {
        self.terms.bound()
    }

    pub fn source(&self) -> SourceId {
        self.span.source()
    }

    pub fn binds(&self, name: &BoundName) -> bool {
        self.terms.binds(name)
    }
}

impl DefinitionBuffer {
    pub fn inputs(&self) -> &[BufferedInput] {
        &self.inputs
    }

    pub fn sources(&self) -> Vec<SourceId> {
        self.sources_kept_by(&Replacement::default())
    }

    pub fn spans(&self) -> Vec<Span> {
        self.inputs.iter().map(|input| input.span).collect()
    }

    pub fn bound_names(&self) -> impl Iterator<Item = &BoundName> {
        self.inputs.iter().flat_map(|input| input.names())
    }

    pub fn sources_kept_by(&self, replacement: &Replacement) -> Vec<SourceId> {
        self.inputs
            .iter()
            .enumerate()
            .filter(|(position, _)| !replacement.replaces(*position))
            .map(|(_, input)| input.source())
            .collect()
    }

    pub fn widened(&self, entered: &InputTerms, replacement: &Replacement) -> Option<Replacement> {
        let sharing: Vec<usize> = self
            .inputs
            .iter()
            .enumerate()
            .filter(|(position, input)| {
                !replacement.replaces(*position) && input.terms.shares_a_term_with(entered)
            })
            .map(|(position, _)| position)
            .collect();
        if sharing.is_empty() {
            return None;
        }
        let mut replaced = replacement.replaced.clone();
        replaced.extend(sharing);
        Some(Replacement { replaced })
    }

    pub fn dropped_by(
        &self,
        replacement: &Replacement,
        entered: &BufferedInput,
    ) -> Vec<Box<dyn SetAsideDrop>> {
        replacement
            .replaced
            .iter()
            .filter_map(|position| Self::drop_of(&self.inputs[*position], entered))
            .collect()
    }

    pub fn replaced(&self, replacement: &Replacement, input: BufferedInput) -> Self {
        let place = replacement.first();
        let mut entered = Some(input);
        let mut inputs = Vec::with_capacity(self.inputs.len() + 1);
        for (position, earlier) in self.inputs.iter().enumerate() {
            if Some(position) == place {
                inputs.extend(entered.take());
            }
            if !replacement.replaces(position) {
                inputs.push(earlier.clone());
            }
        }
        inputs.extend(entered);
        Self { inputs }
    }

    pub fn renamed(&self, names: Vec<Vec<BoundName>>) -> Self {
        let inputs = self
            .inputs
            .iter()
            .zip(names)
            .map(|(input, names)| BufferedInput {
                terms: input.terms.rebound(names),
                ..input.clone()
            })
            .collect();
        Self { inputs }
    }

    pub fn without(&self, set_aside: &[SourceId]) -> Self {
        let inputs = self
            .inputs
            .iter()
            .filter(|input| !set_aside.contains(&input.source()))
            .cloned()
            .collect();
        Self { inputs }
    }

    pub fn input(&self, source: SourceId) -> Option<&BufferedInput> {
        self.inputs.iter().find(|input| input.source() == source)
    }

    fn drop_of(earlier: &BufferedInput, entered: &BufferedInput) -> Option<Box<dyn SetAsideDrop>> {
        let source = SetAsideSource {
            name: earlier.name().to_string(),
            span: earlier.span,
        };
        let documents =
            DroppedDocuments::left_behind(source.clone(), &earlier.terms, &entered.terms);
        match documents {
            Some(drop) => Some(Box::new(drop) as Box<dyn SetAsideDrop>),
            None => DroppedDefinitions::left_behind(source, &earlier.terms, &entered.terms)
                .map(|drop| Box::new(drop) as Box<dyn SetAsideDrop>),
        }
    }
}

impl Replacement {
    fn replaces(&self, position: usize) -> bool {
        self.replaced.contains(&position)
    }

    fn first(&self) -> Option<usize> {
        self.replaced.iter().min().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Namespace;

    struct BufferFixture;

    impl BufferFixture {
        const SETTER: &str = "<input 1>";

        fn input(source: usize, names: &[&str]) -> BufferedInput {
            let source = SourceId::new(source);
            let names = names
                .iter()
                .map(|name| BoundName {
                    name: (*name).into(),
                    namespace: Namespace::Value,
                    span: Span::new(source, 0..0),
                })
                .collect();
            Self::entered(source, InputTerms::binding(names))
        }

        fn document(source: usize, term: &str) -> BufferedInput {
            Self::entered(SourceId::new(source), Self::documenting(term))
        }

        fn entered(source: SourceId, terms: InputTerms) -> BufferedInput {
            BufferedInput::new(
                format!("<input {}>", source.index()),
                Span::new(source, 0..1),
                terms,
            )
        }

        fn documenting(term: &str) -> InputTerms {
            InputTerms::documenting(Vec::new(), vec![term.into()])
        }

        fn terms(names: &[BoundName]) -> InputTerms {
            InputTerms::binding(names.to_vec())
        }

        fn documenting_sources(buffer: &DefinitionBuffer, term: &str) -> Vec<usize> {
            let documenting = Self::documenting(term);
            buffer
                .inputs
                .iter()
                .filter(|input| input.terms.shares_a_term_with(&documenting))
                .map(|input| input.source().index())
                .collect()
        }

        fn buffer(inputs: &[(usize, &[&str])]) -> DefinitionBuffer {
            DefinitionBuffer {
                inputs: inputs
                    .iter()
                    .map(|(source, names)| Self::input(*source, names))
                    .collect(),
            }
        }

        fn sources(buffer: &DefinitionBuffer) -> Vec<usize> {
            buffer.sources().into_iter().map(SourceId::index).collect()
        }
    }

    #[test]
    fn a_replacing_input_takes_the_place_of_the_first_input_it_replaces() {
        let buffer = BufferFixture::buffer(&[(0, &["a"]), (1, &["b"]), (2, &["c"])]);
        let entered = BufferFixture::input(3, &["b"]);
        let replacement = buffer
            .widened(
                &BufferFixture::terms(entered.names()),
                &Replacement::default(),
            )
            .expect("`b` is defined earlier");
        let replaced = buffer.replaced(&replacement, entered);
        assert_eq!(BufferFixture::sources(&replaced), [0, 3, 2]);
    }

    #[test]
    fn an_input_that_replaces_two_inputs_takes_the_place_of_the_earlier() {
        let buffer = BufferFixture::buffer(&[(0, &["a"]), (1, &["b"]), (2, &["c"])]);
        let entered = BufferFixture::input(3, &["a", "c"]);
        let first = buffer
            .widened(
                &BufferFixture::terms(&entered.names()[..1]),
                &Replacement::default(),
            )
            .expect("`a` is defined earlier");
        let both = buffer
            .widened(&BufferFixture::terms(entered.names()), &first)
            .expect("`c` is defined earlier");
        let replaced = buffer.replaced(&both, entered);
        assert_eq!(BufferFixture::sources(&replaced), [3, 1]);
    }

    #[test]
    fn a_new_input_joins_the_end() {
        let buffer = BufferFixture::buffer(&[(0, &["a"]), (1, &["b"])]);
        let replaced = buffer.replaced(&Replacement::default(), BufferFixture::input(2, &["c"]));
        assert_eq!(BufferFixture::sources(&replaced), [0, 1, 2]);
    }

    #[test]
    fn a_replacement_widens_only_by_inputs_it_does_not_hold() {
        let buffer = BufferFixture::buffer(&[(0, &["a"]), (1, &["b"])]);
        let names = BufferFixture::input(2, &["a", "b"]).names().to_vec();
        let first = buffer
            .widened(&BufferFixture::terms(&names[..1]), &Replacement::default())
            .expect("`a` is defined earlier");
        let second = buffer
            .widened(&BufferFixture::terms(&names), &first)
            .expect("`b` is defined earlier");
        assert_eq!(buffer.widened(&BufferFixture::terms(&names), &second), None);
        assert_eq!(buffer.sources_kept_by(&second), []);
    }

    #[test]
    fn a_replacement_that_keeps_every_name_drops_nothing() {
        let buffer = BufferFixture::buffer(&[(0, &["a", "b"])]);
        let entered = BufferFixture::input(1, &["b", "a"]);
        let replacement = buffer
            .widened(
                &BufferFixture::terms(entered.names()),
                &Replacement::default(),
            )
            .expect("`a` is defined earlier");
        assert!(buffer.dropped_by(&replacement, &entered).is_empty());
    }

    #[test]
    fn a_replacement_that_loses_a_name_drops_it() {
        let buffer = BufferFixture::buffer(&[(0, &["a", "b"])]);
        let entered = BufferFixture::input(1, &["b"]);
        let replacement = buffer
            .widened(
                &BufferFixture::terms(entered.names()),
                &Replacement::default(),
            )
            .expect("`b` is defined earlier");
        let dropped = buffer.dropped_by(&replacement, &entered);
        let warning = dropped[0].warning(BufferFixture::SETTER);
        assert_eq!(warning.title(), "set aside `a` after `<input 1>`");
        assert_eq!(warning.notes(), ["`b` replaced the input that defined `a`"]);
    }

    #[test]
    fn a_document_that_leaves_a_term_behind_drops_it() {
        let earlier = BufferFixture::entered(
            SourceId::new(0),
            InputTerms::documenting(Vec::new(), vec!["g".into(), "k".into()]),
        );
        let buffer = DefinitionBuffer {
            inputs: vec![earlier],
        };
        let entered = BufferFixture::document(1, "g");
        let replacement = buffer
            .widened(&BufferFixture::documenting("g"), &Replacement::default())
            .expect("`g` is documented earlier");
        let dropped = buffer.dropped_by(&replacement, &entered);
        let warning = dropped[0].warning(BufferFixture::SETTER);
        assert_eq!(warning.title(), "set aside `k` after `<input 1>`");
    }

    #[test]
    fn a_renamed_buffer_takes_each_inputs_new_names() {
        let buffer = BufferFixture::buffer(&[(0, &["a"]), (1, &["b"])]);
        let renamed = buffer.renamed(vec![
            Vec::new(),
            BufferFixture::input(1, &["c"]).names().to_vec(),
        ]);
        let names: Vec<&str> = renamed.bound_names().map(|name| &*name.name).collect();
        assert_eq!(names, ["c"]);
    }

    #[test]
    fn a_buffer_without_an_input_keeps_the_others_in_order() {
        let buffer = BufferFixture::buffer(&[(0, &["a"]), (1, &["b"]), (2, &["c"])]);
        let kept = buffer.without(&[SourceId::new(1)]);
        assert_eq!(BufferFixture::sources(&kept), [0, 2]);
        assert!(kept.input(SourceId::new(1)).is_none());
        assert!(kept.input(SourceId::new(2)).is_some());
    }

    #[test]
    fn a_document_replaces_an_earlier_document_of_the_same_term() {
        let buffer = DefinitionBuffer {
            inputs: vec![
                BufferFixture::input(0, &["g"]),
                BufferFixture::document(1, "g"),
            ],
        };
        let later = BufferFixture::document(2, "g");
        let replacement = buffer
            .widened(&BufferFixture::documenting("g"), &Replacement::default())
            .expect("`g` is documented earlier");
        let replaced = buffer.replaced(&replacement, later);
        assert_eq!(BufferFixture::sources(&replaced), [0, 2]);
        assert_eq!(BufferFixture::documenting_sources(&replaced, "g"), [2]);
    }

    #[test]
    fn a_document_never_replaces_the_input_that_defines_its_term() {
        let buffer = BufferFixture::buffer(&[(0, &["g"])]);
        let terms = BufferFixture::documenting("g");
        assert_eq!(buffer.widened(&terms, &Replacement::default()), None);
    }
}
