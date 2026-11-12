use std::collections::BTreeSet;
use std::ops::Range;

use knave_expand::mangle::NameMangler;
use knave_expand::reader::{Token, TokenKind, TokenScan};

pub struct TopLevelDefinitions {
    definitions: Vec<Definition>,
}

struct Definition {
    name: String,
    range: Range<usize>,
    mentions: BTreeSet<String>,
}

impl TopLevelDefinitions {
    const DEFINITION_PREFIX: &str = "def";
    const ENTRY: &str = "main";

    pub fn of_text(text: &str) -> Self {
        let mut reader = DefinitionReader::default();
        TokenScan::new(text).for_each(|token| reader.read(&token, text));
        Self {
            definitions: reader.definitions,
        }
    }

    pub fn name_at(&self, offset: usize) -> Option<&str> {
        self.definitions
            .iter()
            .find(|definition| definition.range.contains(&offset))
            .map(|definition| definition.name.as_str())
    }

    pub fn named_in_rust(&self, rust_name: &str) -> Option<&str> {
        self.definitions
            .iter()
            .find(|definition| NameMangler::value(&definition.name) == rust_name)
            .map(|definition| definition.name.as_str())
    }

    pub fn reaches(&self, function: &str) -> bool {
        self.definition_named(function).is_none() || self.reached_from_main().contains(function)
    }

    fn reached_from_main(&self) -> BTreeSet<&str> {
        let mut reached = BTreeSet::new();
        let mut waiting = vec![Self::ENTRY];
        while let Some(name) = waiting.pop() {
            if reached.insert(name) {
                waiting.extend(self.mentioned_by(name));
            }
        }
        reached
    }

    fn mentioned_by(&self, name: &str) -> impl Iterator<Item = &str> {
        self.definitions
            .iter()
            .filter(move |definition| definition.name == name)
            .flat_map(|definition| &definition.mentions)
            .filter_map(|mention| self.definition_named(mention))
    }

    fn definition_named(&self, name: &str) -> Option<&str> {
        self.definitions
            .iter()
            .find(|definition| definition.name == name)
            .map(|definition| definition.name.as_str())
    }
}

#[derive(Default)]
struct DefinitionReader {
    depth: usize,
    start: usize,
    words: Vec<String>,
    mentions: BTreeSet<String>,
    definitions: Vec<Definition>,
}

impl DefinitionReader {
    const WORDS_OF_A_HEADER: usize = 2;

    fn read(&mut self, token: &Token, text: &str) {
        match token.kind {
            TokenKind::Open => self.open(token.range.start),
            TokenKind::Close => self.close(token.range.start),
            TokenKind::Symbol => self.symbol(&text[token.range.clone()]),
            _ => {}
        }
    }

    fn open(&mut self, offset: usize) {
        if self.depth == 0 {
            self.start = offset;
            self.words.clear();
            self.mentions.clear();
        }
        self.depth += 1;
    }

    fn close(&mut self, offset: usize) {
        self.depth = self.depth.saturating_sub(1);
        if self.depth == 0 {
            self.finish(offset + 1);
        }
    }

    fn symbol(&mut self, word: &str) {
        if self.depth == 1 && self.words.len() < Self::WORDS_OF_A_HEADER {
            self.words.push(word.to_string());
        } else if self.depth > 0 {
            self.mentions.insert(word.to_string());
        }
    }

    fn finish(&mut self, end: usize) {
        if let [head, name] = self.words.as_slice()
            && head.starts_with(TopLevelDefinitions::DEFINITION_PREFIX)
        {
            self.definitions.push(Definition {
                name: name.clone(),
                range: self.start..end,
                mentions: std::mem::take(&mut self.mentions),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TopLevelDefinitions;

    struct DefinitionsFixture;

    impl DefinitionsFixture {
        const TWO_FUNCTIONS: &str =
            "(defn grow! (v) (push! v 1))\n(defn main ()\n  (grow! (vec 1)))\n";
        const UNREACHED: &str = "(defn helper () 1)\n(defn unused () (helper))\n(defn shown () 2)\n(defn main () (map (iter (vec 1)) shown))\n";
    }

    #[test]
    fn names_the_definition_that_holds_an_offset() {
        let definitions = TopLevelDefinitions::of_text(DefinitionsFixture::TWO_FUNCTIONS);
        assert_eq!(definitions.name_at(3), Some("grow!"));
        assert_eq!(definitions.name_at(40), Some("main"));
    }

    #[test]
    fn names_nothing_between_definitions_or_in_a_form_that_defines_nothing() {
        let definitions = TopLevelDefinitions::of_text("(defn f () 1)\n(use other)\n");
        assert_eq!(definitions.name_at(13), None);
        assert_eq!(definitions.name_at(16), None);
    }

    #[test]
    fn finds_the_knave_name_of_an_emitted_rust_function() {
        let definitions = TopLevelDefinitions::of_text(DefinitionsFixture::TWO_FUNCTIONS);
        assert_eq!(definitions.named_in_rust("grow"), Some("grow!"));
        assert_eq!(definitions.named_in_rust("shrink"), None);
    }

    #[test]
    fn reaches_every_definition_main_calls_or_names_and_no_other() {
        let definitions = TopLevelDefinitions::of_text(DefinitionsFixture::UNREACHED);
        let reached: Vec<&str> = definitions.reached_from_main().into_iter().collect();
        assert_eq!(reached, ["main", "shown"]);
    }

    #[test]
    fn reaches_a_name_it_does_not_define() {
        let definitions = TopLevelDefinitions::of_text(DefinitionsFixture::UNREACHED);
        assert!(definitions.reaches("shown"));
        assert!(definitions.reaches("println"));
        assert!(!definitions.reaches("unused"));
    }
}
