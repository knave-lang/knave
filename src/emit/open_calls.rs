use super::argument_start::ArgumentStart;
use super::literal::Literal;
use super::nesting::Nesting;
use super::open_delimiter::OpenDelimiter;

pub struct OpenCalls<'a, 'm> {
    open: Vec<OpenDelimiter<'a>>,
    marked: MarkedStarts<'a, 'm>,
}

struct MarkedStarts<'a, 'm> {
    marks: &'m [usize],
    starts: Vec<Option<ArgumentStart<'a>>>,
}

impl<'a, 'm> OpenCalls<'a, 'm> {
    pub fn marking(marks: &'m [usize]) -> Self {
        Self {
            open: Vec::new(),
            marked: MarkedStarts {
                marks,
                starts: Vec::with_capacity(marks.len()),
            },
        }
    }

    pub fn into_starts(self) -> Vec<Option<ArgumentStart<'a>>> {
        self.marked.starts
    }

    fn start_at(&self, text: &'a str, mark: usize, literal: Literal) -> Option<ArgumentStart<'a>> {
        if literal != Literal::Outside {
            return None;
        }
        ArgumentStart::after(self.open.last()?, &text[..mark])
    }
}

impl<'a> Nesting<'a> for OpenCalls<'a, '_> {
    fn open(&mut self, called_after: Option<&'a str>) {
        self.open.push(OpenDelimiter::new(called_after));
    }

    fn close(&mut self) {
        self.open.pop();
    }

    fn depth(&self) -> usize {
        self.open.len()
    }

    fn count_argument(&mut self) {
        if let Some(open) = self.open.last_mut() {
            open.count_argument();
        }
    }

    fn hold_text(&mut self, text: &'a str) {
        if let Some(open) = self.open.last_mut() {
            open.hold_text(text);
        }
    }

    fn reach(&mut self, text: &'a str, position: usize, literal: Literal) {
        while let Some(mark) = self.marked.next_up_to(position) {
            let start = self.start_at(text, mark, literal);
            self.marked.starts.push(start);
        }
    }
}

impl MarkedStarts<'_, '_> {
    fn next_up_to(&self, position: usize) -> Option<usize> {
        self.marks
            .get(self.starts.len())
            .copied()
            .filter(|mark| *mark <= position)
    }
}
