use super::open_delimiter::OpenDelimiter;

pub struct ArgumentStart<'a> {
    open: OpenDelimiter<'a>,
    name: Option<&'a str>,
}

impl<'a> ArgumentStart<'a> {
    pub(super) fn new(open: OpenDelimiter<'a>, name: Option<&'a str>) -> Self {
        Self { open, name }
    }

    pub fn open(&self) -> &OpenDelimiter<'a> {
        &self.open
    }

    pub fn name(&self) -> Option<&'a str> {
        self.name
    }
}
