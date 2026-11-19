pub(super) struct FormatString<'t> {
    text: &'t str,
}

pub(super) struct PassedArgument<'t> {
    position: usize,
    name: Option<&'t str>,
}

struct FormatPlaceholder<'t> {
    argument: FormatArgument<'t>,
    spec: &'t str,
}

struct FormatPlaceholders<'t> {
    rest: &'t str,
    next_positional: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FormatArgument<'t> {
    Positional(usize),
    Named(&'t str),
}

impl<'t> FormatString<'t> {
    pub(super) fn new(text: &'t str) -> Self {
        Self { text }
    }

    pub(super) fn points_at(&self, argument: &PassedArgument<'_>) -> bool {
        FormatPlaceholders::of(self.text).any(|placeholder| {
            argument.is_formatted_by(placeholder.argument) && placeholder.is_pointer()
        })
    }
}

impl<'t> PassedArgument<'t> {
    pub(super) fn new(position: usize, name: Option<&'t str>) -> Self {
        Self { position, name }
    }

    fn is_formatted_by(&self, formatted: FormatArgument<'_>) -> bool {
        match formatted {
            FormatArgument::Positional(position) => position == self.position,
            FormatArgument::Named(name) => Some(name) == self.name,
        }
    }
}

impl FormatPlaceholder<'_> {
    const POINTER: char = 'p';

    fn is_pointer(&self) -> bool {
        self.spec.trim_end().ends_with(Self::POINTER)
    }
}

impl<'t> FormatPlaceholders<'t> {
    const PRECISION_FROM_ARGUMENT: &'static str = ".*";

    fn of(text: &'t str) -> Self {
        Self {
            rest: text,
            next_positional: 0,
        }
    }

    fn next_written(&mut self) -> Option<&'t str> {
        loop {
            let opened = self.rest.find('{')?;
            let after = &self.rest[opened + 1..];
            if let Some(escaped) = after.strip_prefix('{') {
                self.rest = escaped;
                continue;
            }
            let (inside, rest) = after.split_once('}')?;
            self.rest = rest;
            return Some(inside);
        }
    }

    fn resolve(&mut self, written: &'t str, spec: &str) -> FormatArgument<'t> {
        if spec.contains(Self::PRECISION_FROM_ARGUMENT) {
            self.next_positional += 1;
        }
        match written.parse() {
            Ok(index) => FormatArgument::Positional(index),
            Err(_) if written.is_empty() => self.take_positional(),
            Err(_) => FormatArgument::Named(written),
        }
    }

    fn take_positional(&mut self) -> FormatArgument<'t> {
        let argument = FormatArgument::Positional(self.next_positional);
        self.next_positional += 1;
        argument
    }
}

impl<'t> Iterator for FormatPlaceholders<'t> {
    type Item = FormatPlaceholder<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        let placeholder = self.next_written()?;
        let (written, spec) = placeholder.split_once(':').unwrap_or((placeholder, ""));
        let argument = self.resolve(written.trim(), spec);
        Some(FormatPlaceholder { argument, spec })
    }
}
