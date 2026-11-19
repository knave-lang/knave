#[derive(Clone)]
pub struct OpenDelimiter<'a> {
    callee: Option<&'a str>,
    arguments: usize,
    texts: Vec<TextArgument<'a>>,
}

#[derive(Clone, Copy)]
struct TextArgument<'a> {
    position: usize,
    text: &'a str,
}

impl<'a> OpenDelimiter<'a> {
    pub(super) fn new(callee: Option<&'a str>) -> Self {
        Self {
            callee,
            arguments: 0,
            texts: Vec::new(),
        }
    }

    pub(super) fn count_argument(&mut self) {
        self.arguments += 1;
    }

    pub(super) fn hold_text(&mut self, text: &'a str) {
        self.texts.push(TextArgument {
            position: self.arguments,
            text,
        });
    }

    pub fn callee(&self) -> Option<&'a str> {
        self.callee
    }

    pub fn passed_arguments(&self) -> usize {
        self.arguments
    }

    pub fn text_at(&self, argument: usize) -> Option<&'a str> {
        self.texts
            .iter()
            .find(|held| held.position == argument)
            .map(|held| held.text)
    }
}
