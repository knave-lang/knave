use super::identifier::Identifier;
use super::open_delimiter::OpenDelimiter;

pub struct ArgumentStart<'a> {
    open: OpenDelimiter<'a>,
    name: Option<&'a str>,
}

impl<'a> ArgumentStart<'a> {
    pub(super) fn after(open: &OpenDelimiter<'a>, written: &'a str) -> Option<Self> {
        let written = written.trim_end();
        let name = if written.ends_with(',') {
            None
        } else {
            Some(Self::argument_name(written)?)
        };
        Some(Self {
            open: open.clone(),
            name,
        })
    }

    pub fn open(&self) -> &OpenDelimiter<'a> {
        &self.open
    }

    pub fn name(&self) -> Option<&'a str> {
        self.name
    }

    fn argument_name(written: &'a str) -> Option<&'a str> {
        let named = written.strip_suffix('=')?.trim_end();
        let name = Identifier::ending(named).text();
        let before_name = &named[..named.len() - name.len()];
        (!name.is_empty() && before_name.trim_end().ends_with(',')).then_some(name)
    }
}
