use super::access::Access;
use super::conversion::Conversion;
use super::template_braces::TemplateBraces;
use super::template_operation::TemplateOperation;

#[derive(Debug, PartialEq, Eq)]
pub struct MalformedPlaceholder;

struct WrittenPlaceholder;

impl WrittenPlaceholder {
    const BINDER: char = '!';

    fn read(
        characters: &mut std::iter::Peekable<std::str::Chars>,
    ) -> Result<Piece, MalformedPlaceholder> {
        let borrow_marker = characters.next_if_eq(&Access::BORROW);
        let written = Self::placeholder_name(characters)?;
        let (conversion, unconverted) = Conversion::split(&written);
        let (access, name) = Self::access(borrow_marker, unconverted);
        let marks = PlaceholderMarks { access, conversion };
        if marks.combines_access_and_conversion() {
            return Err(MalformedPlaceholder);
        }
        match name.strip_prefix(Self::BINDER) {
            Some(binder) => Self::binder_placeholder(binder, &marks),
            None => Self::argument_placeholder(name, marks),
        }
    }

    fn binder_placeholder(
        binder: &str,
        marks: &PlaceholderMarks,
    ) -> Result<Piece, MalformedPlaceholder> {
        if marks.is_marked() || binder.is_empty() {
            return Err(MalformedPlaceholder);
        }
        Ok(Piece::BinderPlaceholder {
            name: binder.to_string(),
        })
    }

    fn argument_placeholder(
        name: &str,
        marks: PlaceholderMarks,
    ) -> Result<Piece, MalformedPlaceholder> {
        match name.parse::<usize>() {
            Ok(index) => Ok(Piece::Placeholder {
                index,
                access: marks.access,
                conversion: marks.conversion,
            }),
            Err(_) if marks.is_marked() => Err(MalformedPlaceholder),
            Err(_) => Ok(Piece::TypePlaceholder {
                name: name.to_string(),
            }),
        }
    }

    fn access(borrow_marker: Option<char>, name: &str) -> (Access, &str) {
        if borrow_marker.is_some() {
            return (Access::Borrowed, name);
        }
        match name.strip_prefix(Access::OWNED) {
            Some(index) => (Access::Owned, index),
            None => (Access::Passed, name),
        }
    }

    fn placeholder_name(
        characters: &mut impl Iterator<Item = char>,
    ) -> Result<String, MalformedPlaceholder> {
        let mut name = String::new();
        for next in characters {
            if next == TemplateBraces::CLOSE {
                return if name.is_empty() {
                    Err(MalformedPlaceholder)
                } else {
                    Ok(name)
                };
            }
            name.push(next);
        }
        Err(MalformedPlaceholder)
    }
}

struct PlaceholderMarks {
    access: Access,
    conversion: Option<Conversion>,
}

impl PlaceholderMarks {
    fn is_marked(&self) -> bool {
        self.access != Access::Passed || self.conversion.is_some()
    }

    fn combines_access_and_conversion(&self) -> bool {
        self.access != Access::Passed && self.conversion.is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Piece {
    Literal(String),
    Placeholder {
        index: usize,
        access: Access,
        conversion: Option<Conversion>,
    },
    TypePlaceholder {
        name: String,
    },
    BinderPlaceholder {
        name: String,
    },
}

impl Piece {
    pub fn conversions_in(pieces: &[Self]) -> impl Iterator<Item = (usize, Conversion)> + '_ {
        pieces.iter().filter_map(Self::conversion)
    }

    pub fn conversion(&self) -> Option<(usize, Conversion)> {
        match self {
            Self::Placeholder {
                index,
                conversion: Some(conversion),
                ..
            } => Some((*index, *conversion)),
            _ => None,
        }
    }

    pub fn unconverted(&self) -> Self {
        match self {
            Self::Placeholder { index, access, .. } => Self::Placeholder {
                index: *index,
                access: *access,
                conversion: None,
            },
            other => other.clone(),
        }
    }

    pub(super) fn access_for(&self, argument: usize) -> Option<Access> {
        match self {
            Self::Placeholder { index, access, .. } if *index == argument => Some(*access),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassingDifference {
    Argument(usize, PassingAspect),
    CallOnlyInClause,
    ImpureClause,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassingAspect {
    Use,
    Borrow,
    Receiver,
    Index,
    Dereference,
    Call,
    Operand,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ParsedTemplate {
    pieces: Vec<Piece>,
    text: String,
}

impl ParsedTemplate {
    const DEREFERENCE: &'static str = "*";

    pub fn parse(text: &str) -> Result<Self, MalformedPlaceholder> {
        let mut pieces = Vec::new();
        let mut literal = String::new();
        let mut characters = text.chars().peekable();
        while let Some(character) = characters.next() {
            match (character, characters.peek()) {
                (TemplateBraces::OPEN, Some(&TemplateBraces::OPEN))
                | (TemplateBraces::CLOSE, Some(&TemplateBraces::CLOSE)) => {
                    characters.next();
                    literal.push(character);
                }
                (TemplateBraces::OPEN, _) => {
                    let placeholder = WrittenPlaceholder::read(&mut characters)?;
                    pieces.push(Piece::Literal(std::mem::take(&mut literal)));
                    pieces.push(placeholder);
                }
                (TemplateBraces::CLOSE, _) => return Err(MalformedPlaceholder),
                _ => literal.push(character),
            }
        }
        pieces.push(Piece::Literal(literal));
        pieces.retain(|piece| *piece != Piece::Literal(String::new()));
        Ok(Self {
            pieces,
            text: text.to_string(),
        })
    }

    pub fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn placeholders(&self) -> impl Iterator<Item = usize> + '_ {
        self.pieces.iter().filter_map(|piece| match piece {
            Piece::Placeholder { index, .. } => Some(*index),
            Piece::Literal(_) | Piece::TypePlaceholder { .. } | Piece::BinderPlaceholder { .. } => {
                None
            }
        })
    }

    pub fn type_placeholders(&self) -> impl Iterator<Item = &str> + '_ {
        self.pieces.iter().filter_map(|piece| match piece {
            Piece::TypePlaceholder { name } => Some(name.as_str()),
            Piece::Literal(_) | Piece::Placeholder { .. } | Piece::BinderPlaceholder { .. } => None,
        })
    }

    pub fn binder_names(&self) -> Vec<&str> {
        let mut names = Vec::new();
        for piece in &self.pieces {
            if let Piece::BinderPlaceholder { name } = piece
                && !names.contains(&name.as_str())
            {
                names.push(name.as_str());
            }
        }
        names
    }

    pub fn with_binders(&self, renames: &[(String, String)]) -> Self {
        let pieces = self
            .pieces
            .iter()
            .map(|piece| match piece {
                Piece::BinderPlaceholder { name } => {
                    let fresh = renames
                        .iter()
                        .find(|(binder, _)| binder == name)
                        .map_or(name.as_str(), |(_, fresh)| fresh.as_str());
                    Piece::Literal(fresh.to_string())
                }
                other => other.clone(),
            })
            .collect();
        Self::from_pieces(pieces)
    }

    pub(super) fn without_written_borrow(&self, argument: usize) -> Self {
        let mut pieces = self.pieces.clone();
        for position in 1..pieces.len() {
            let borrowed = matches!(&pieces[position],
                Piece::Placeholder { index, access: Access::Borrowed, .. } if *index == argument);
            if borrowed
                && let Piece::Literal(text) = &mut pieces[position - 1]
                && text.ends_with(Access::BORROW)
            {
                text.pop();
                pieces[position] = Piece::Placeholder {
                    index: argument,
                    access: Access::Passed,
                    conversion: None,
                };
            }
        }
        Self::from_pieces(pieces)
    }

    fn from_pieces(pieces: Vec<Piece>) -> Self {
        let mut merged: Vec<Piece> = Vec::new();
        for piece in pieces {
            if let (Piece::Literal(text), Some(Piece::Literal(previous))) =
                (&piece, merged.last_mut())
            {
                previous.push_str(text);
            } else {
                merged.push(piece);
            }
        }
        let text = merged.iter().map(Self::rendered_piece).collect();
        Self {
            pieces: merged,
            text,
        }
    }

    fn rendered_piece(piece: &Piece) -> String {
        match piece {
            Piece::Literal(literal) => literal.replace('{', "{{").replace('}', "}}"),
            Piece::Placeholder {
                index,
                conversion: Some(conversion),
                ..
            } => conversion.placeholder(*index),
            Piece::Placeholder {
                index,
                access,
                conversion: None,
            } => format!("{{{}{index}}}", access.marker()),
            Piece::TypePlaceholder { name } => format!("{{{name}}}"),
            Piece::BinderPlaceholder { name } => format!("{{!{name}}}"),
        }
    }

    pub fn borrows(&self, argument: usize) -> bool {
        let uses = self.accesses(argument);
        !uses.is_empty() && uses.iter().all(|access| *access == Access::Borrowed)
    }

    fn accesses(&self, argument: usize) -> Vec<Access> {
        self.pieces
            .iter()
            .filter_map(|piece| piece.access_for(argument))
            .collect()
    }

    pub fn receives(&self, argument: usize) -> bool {
        let pieces = match self.pieces.as_slice() {
            [Piece::Literal(dereference), rest @ ..] if dereference == Self::DEREFERENCE => rest,
            pieces => pieces,
        };
        matches!(
            pieces,
            [Piece::Placeholder { index, access, .. }, Piece::Literal(method), ..]
                if *index == argument && *access != Access::Borrowed && method.starts_with('.')
        )
    }

    pub fn borrows_any(&self) -> bool {
        self.pieces.iter().any(
            |piece| matches!(piece, Piece::Placeholder { access, .. } if *access == Access::Borrowed),
        )
    }

    pub fn called_arguments(&self, argument: usize) -> Option<Vec<&str>> {
        let literal = self.call_literal(argument)?;
        Self::arguments_after_open_parenthesis(literal)
    }

    fn call_literal(&self, argument: usize) -> Option<&str> {
        let mut uses = self
            .pieces
            .iter()
            .enumerate()
            .filter_map(|(position, piece)| {
                matches!(piece, Piece::Placeholder { index, .. } if *index == argument)
                    .then_some(position)
            });
        let position = uses.next()?;
        if uses.next().is_some() {
            return None;
        }
        match self.pieces.get(position + 1)? {
            Piece::Literal(literal) => Some(literal),
            Piece::Placeholder { .. }
            | Piece::TypePlaceholder { .. }
            | Piece::BinderPlaceholder { .. } => None,
        }
    }

    fn arguments_after_open_parenthesis(literal: &str) -> Option<Vec<&str>> {
        let inner = literal.trim_start().strip_prefix('(')?;
        let close = inner.find(')')?;
        let arguments: Vec<&str> = inner[..close].split(',').map(str::trim).collect();
        (!arguments.iter().any(|name| name.is_empty())).then_some(arguments)
    }

    pub fn indexes(&self, argument: usize) -> bool {
        self.pieces.iter().enumerate().any(|(position, piece)| {
            matches!(piece, Piece::Placeholder { index, .. } if *index == argument)
                && self.pieces.get(position + 1).is_some_and(|next| {
                    matches!(next, Piece::Literal(literal) if literal.trim_start().starts_with('['))
                })
        })
    }

    pub fn is_single_index_place(&self) -> bool {
        matches!(
            self.pieces.as_slice(),
            [
                Piece::Placeholder { index: 0, .. },
                Piece::Literal(open),
                Piece::Placeholder { .. },
                Piece::Literal(close),
            ] if open.trim() == "[" && close.trim() == "]"
        )
    }

    pub fn dereferences(&self, argument: usize) -> bool {
        let mut found = false;
        for (position, piece) in self.pieces.iter().enumerate() {
            if !matches!(piece, Piece::Placeholder { index, .. } if *index == argument) {
                continue;
            }
            found = true;
            let dereferenced = position
                .checked_sub(1)
                .and_then(|previous| match &self.pieces[previous] {
                    Piece::Literal(literal) => Some(Self::unary_deref(literal)),
                    Piece::Placeholder { .. }
                    | Piece::TypePlaceholder { .. }
                    | Piece::BinderPlaceholder { .. } => None,
                })
                .unwrap_or(false);
            if !dereferenced {
                return false;
            }
        }
        found
    }

    fn unary_deref(literal: &str) -> bool {
        let Some(remainder) = literal.strip_suffix(Self::DEREFERENCE) else {
            return false;
        };
        remainder
            .chars()
            .rev()
            .find(|character| !character.is_whitespace())
            .is_none_or(|character| {
                !character.is_alphanumeric() && character != '_' && !")]}".contains(character)
            })
    }

    pub fn is_path(&self) -> bool {
        match self.pieces.as_slice() {
            [Piece::Literal(literal)] => literal
                .chars()
                .all(|character| character.is_alphanumeric() || "_:".contains(character)),
            _ => false,
        }
    }

    pub fn operation(&self) -> TemplateOperation<'_> {
        TemplateOperation::of(&self.pieces)
    }
}

#[cfg(test)]
mod tests {
    use super::{Access, Conversion, MalformedPlaceholder, ParsedTemplate, Piece};

    #[test]
    fn splits_literals_from_placeholders() {
        let parsed = ParsedTemplate::parse("Vec<{0}>").expect("a valid template");
        assert_eq!(
            parsed.pieces(),
            [
                Piece::Literal("Vec<".to_string()),
                Piece::Placeholder {
                    index: 0,
                    access: Access::Passed,
                    conversion: None
                },
                Piece::Literal(">".to_string())
            ]
        );
    }

    #[test]
    fn marks_a_placeholder_that_rust_borrows() {
        let parsed = ParsedTemplate::parse("println!(\"{{}}\", {&0})").expect("a valid template");
        assert!(parsed.borrows(0));
        assert!(parsed.borrows_any());
    }

    #[test]
    fn receives_an_argument_before_a_method_call() {
        for text in ["{0}.len()", "*{0}.len()"] {
            let parsed = ParsedTemplate::parse(text).expect("a valid template");
            assert!(parsed.receives(0), "{text}");
            assert!(!parsed.receives(1), "{text}");
        }
    }

    #[test]
    fn does_not_receive_an_argument_outside_a_leading_method_call() {
        for text in [
            "{&0}.len()",
            "{1} + {0}.len()",
            "{0} .len()",
            "{0}[1]",
            "{0}",
        ] {
            let parsed = ParsedTemplate::parse(text).expect("a valid template");
            assert!(!parsed.receives(0), "{text}");
        }
    }

    #[test]
    fn reports_a_placeholder_before_a_bracket() {
        let parsed = ParsedTemplate::parse("{0} [{1}]").expect("a valid template");
        assert!(parsed.indexes(0));
        assert!(!parsed.indexes(1));
    }

    #[test]
    fn accepts_an_element_place_with_one_index_argument() {
        let parsed = ParsedTemplate::parse("{0}[{1}]").expect("a valid template");
        assert!(parsed.is_single_index_place());
    }

    #[test]
    fn accepts_an_element_place_with_a_converted_index() {
        let parsed = ParsedTemplate::parse("{0}[{index 1}]").expect("a valid template");
        assert!(parsed.is_single_index_place());
    }

    #[test]
    fn rejects_a_range_index_as_an_element_place() {
        let parsed = ParsedTemplate::parse("{0}[{1}..{2}]").expect("a valid template");
        assert!(!parsed.is_single_index_place());
    }

    #[test]
    fn ignores_a_placeholder_before_other_text() {
        let parsed = ParsedTemplate::parse("{0}({1})").expect("a valid template");
        assert!(!parsed.indexes(0));
    }

    #[test]
    fn borrows_only_when_every_use_is_marked() {
        let parsed = ParsedTemplate::parse("{0} + {&0}").expect("a valid template");
        assert!(!parsed.borrows(0));
        assert!(!parsed.borrows(1));
    }

    #[test]
    fn marks_a_placeholder_that_the_call_owns() {
        let parsed = ParsedTemplate::parse("{owned 0}.any({1})").expect("a valid template");
        assert_eq!(
            parsed.pieces()[0],
            Piece::Placeholder {
                index: 0,
                access: Access::Owned,
                conversion: None
            }
        );
        assert!(!parsed.borrows(0));
        assert!(!parsed.borrows_any());
        assert_eq!(parsed.text(), "{owned 0}.any({1})");
    }

    #[test]
    fn marks_each_conversion_of_a_placeholder() {
        let parsed =
            ParsedTemplate::parse("{0}.get({index? 1})[{index 2}]").expect("a valid template");
        let conversions: Vec<(usize, Conversion)> = parsed
            .pieces()
            .iter()
            .filter_map(Piece::conversion)
            .collect();
        assert_eq!(
            conversions,
            [(1, Conversion::OptionalIndex), (2, Conversion::Index)]
        );
        assert_eq!(parsed.placeholders().collect::<Vec<_>>(), [0, 1, 2]);
    }

    #[test]
    fn a_converted_placeholder_passes_its_argument() {
        let parsed = ParsedTemplate::parse("&{0}[{index 1}]").expect("a valid template");
        assert_eq!(
            parsed.pieces()[3],
            Piece::Placeholder {
                index: 1,
                access: Access::Passed,
                conversion: Some(Conversion::Index)
            }
        );
        assert!(!parsed.borrows(1));
    }

    #[test]
    fn keeps_the_text_of_a_conversion_through_a_rewrite() {
        let text = "{0}.get({index? 1}).map(|{!item}| {!item}[{index 2}])";
        let renamed = ParsedTemplate::parse(text)
            .expect("a valid template")
            .with_binders(&[("item".to_string(), "item".to_string())]);
        assert_eq!(
            renamed.text(),
            "{0}.get({index? 1}).map(|item| item[{index 2}])"
        );
    }

    #[test]
    fn keeps_the_access_of_a_placeholder_through_a_rewrite() {
        let text = "{&0}.iter().map(|{!item}| {owned 1}.push({!item}))";
        let renamed = ParsedTemplate::parse(text)
            .expect("a valid template")
            .with_binders(&[("item".to_string(), "item".to_string())]);
        assert_eq!(
            renamed.text(),
            "{&0}.iter().map(|item| {owned 1}.push(item))"
        );
    }

    #[test]
    fn drops_the_conversion_of_a_placeholder() {
        let parsed = ParsedTemplate::parse("{owned 0}.nth({index 1})").expect("a valid template");
        let unconverted: Vec<Piece> = parsed.pieces().iter().map(Piece::unconverted).collect();
        let plain = ParsedTemplate::parse("{owned 0}.nth({1})").expect("a valid template");
        assert_eq!(unconverted, plain.pieces());
    }

    #[test]
    fn reports_a_conversion_beside_an_access_or_a_name() {
        for text in [
            "{&index 0}",
            "{index &0}",
            "{index owned 0}",
            "{owned index 0}",
            "{index? !item}",
            "{index target}",
            "{index? target}",
            "{index }",
        ] {
            assert_eq!(
                ParsedTemplate::parse(text).err(),
                Some(MalformedPlaceholder),
                "{text}"
            );
        }
    }

    #[test]
    fn reports_an_owned_placeholder_without_a_number() {
        for text in ["{owned target}", "{&owned 0}", "{owned !left}"] {
            assert_eq!(
                ParsedTemplate::parse(text).err(),
                Some(MalformedPlaceholder),
                "{text}"
            );
        }
    }

    #[test]
    fn reports_a_borrowed_placeholder_without_a_number() {
        assert_eq!(
            ParsedTemplate::parse("{&x}").err(),
            Some(MalformedPlaceholder)
        );
    }

    #[test]
    fn reports_an_unclosed_placeholder() {
        assert_eq!(
            ParsedTemplate::parse("{0").err(),
            Some(MalformedPlaceholder)
        );
    }

    #[test]
    fn reports_an_empty_placeholder() {
        assert_eq!(
            ParsedTemplate::parse("{}").err(),
            Some(MalformedPlaceholder)
        );
    }

    #[test]
    fn reads_the_arguments_of_a_called_placeholder() {
        let parsed = ParsedTemplate::parse("{0}.sort_by(|left, right| {1}(left, right).cmp(&0))")
            .expect("a valid template");
        assert_eq!(parsed.called_arguments(1), Some(vec!["left", "right"]));
    }

    #[test]
    fn rejects_a_placeholder_that_is_not_called() {
        let parsed = ParsedTemplate::parse("{1}.len()").expect("a valid template");
        assert_eq!(parsed.called_arguments(1), None);
    }

    #[test]
    fn rejects_a_repeated_called_placeholder() {
        let parsed = ParsedTemplate::parse("{1}(a) + {1}(b)").expect("a valid template");
        assert_eq!(parsed.called_arguments(1), None);
    }

    #[test]
    fn rejects_a_call_with_no_arguments() {
        let parsed = ParsedTemplate::parse("{1}()").expect("a valid template");
        assert_eq!(parsed.called_arguments(1), None);
    }

    #[test]
    fn rejects_a_placeholder_followed_by_another_placeholder() {
        let parsed = ParsedTemplate::parse("{1}{0}").expect("a valid template");
        assert_eq!(parsed.called_arguments(1), None);
    }

    #[test]
    fn rejects_a_trailing_placeholder() {
        let parsed = ParsedTemplate::parse("{1}").expect("a valid template");
        assert_eq!(parsed.called_arguments(1), None);
    }

    #[test]
    fn rejects_an_uncalled_argument_zero() {
        let parsed = ParsedTemplate::parse("{0}.sort_by(|left, right| {1}(left, right))")
            .expect("a valid template");
        assert_eq!(parsed.called_arguments(0), None);
    }

    #[test]
    fn names_a_type_placeholder() {
        let parsed = ParsedTemplate::parse("{target}").expect("a named placeholder");
        assert_eq!(parsed.type_placeholders().collect::<Vec<_>>(), ["target"]);
        assert_eq!(
            parsed.placeholders().collect::<Vec<_>>(),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn reports_a_borrowed_type_placeholder() {
        assert_eq!(
            ParsedTemplate::parse("{&target}").err(),
            Some(MalformedPlaceholder)
        );
    }

    #[test]
    fn names_each_binder_once() {
        let parsed =
            ParsedTemplate::parse("|{!left}, {!right}| {!left}").expect("a template with binders");
        assert_eq!(parsed.binder_names(), ["left", "right"]);
        assert_eq!(
            parsed.placeholders().collect::<Vec<_>>(),
            Vec::<usize>::new()
        );
        assert_eq!(
            parsed.type_placeholders().collect::<Vec<_>>(),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn reports_a_borrowed_binder_placeholder() {
        assert_eq!(
            ParsedTemplate::parse("{&!left}").err(),
            Some(MalformedPlaceholder)
        );
    }

    #[test]
    fn reports_an_empty_binder_placeholder() {
        assert_eq!(
            ParsedTemplate::parse("{!}").err(),
            Some(MalformedPlaceholder)
        );
    }
}
