use std::fmt;

use super::built_value::BuiltValue;
use super::builtin_site::BuiltinSite;
use crate::diagnostic::Diagnostic;

#[derive(Default)]
pub struct LimitedText {
    text: String,
    wanted: usize,
}

impl LimitedText {
    pub fn into_text(self) -> String {
        self.text
    }
}

impl fmt::Write for LimitedText {
    fn write_str(&mut self, part: &str) -> fmt::Result {
        self.wanted = self.wanted.saturating_add(part.len());
        if self.wanted <= <Self as BuiltValue>::LIMIT {
            self.text.push_str(part);
            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}

impl BuiltValue for LimitedText {
    const LIMIT: usize = <String as BuiltValue>::LIMIT;

    fn built_size(&self) -> usize {
        self.wanted
    }

    fn too_large(site: BuiltinSite, bytes: usize) -> Diagnostic {
        String::too_large(site, bytes)
    }
}
