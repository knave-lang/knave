#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PrintFormat {
    #[default]
    Display,
    Debug,
}

impl PrintFormat {
    pub const CLAUSE: &'static str = "format";
    const DEBUG_WORD: &'static str = "debug";
    pub const PLACEHOLDER_WORD: &'static str = "format ";

    pub fn written(index: usize) -> String {
        format!("{{{}{index}}}", Self::PLACEHOLDER_WORD)
    }

    pub fn named(word: &str) -> Option<Self> {
        (word == Self::DEBUG_WORD).then_some(Self::Debug)
    }

    pub fn placeholder(self) -> &'static str {
        match self {
            Self::Display => "{}",
            Self::Debug => "{:?}",
        }
    }

    pub fn clause(self) -> Option<String> {
        match self {
            Self::Display => None,
            Self::Debug => Some(format!("({} {})", Self::CLAUSE, Self::DEBUG_WORD)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PrintFormat;

    #[test]
    fn debug_is_the_only_written_format() {
        assert_eq!(PrintFormat::named("debug"), Some(PrintFormat::Debug));
        assert_eq!(PrintFormat::named("display"), None);
    }

    #[test]
    fn each_format_writes_its_rust_placeholder() {
        assert_eq!(PrintFormat::Display.placeholder(), "{}");
        assert_eq!(PrintFormat::Debug.placeholder(), "{:?}");
    }

    #[test]
    fn a_template_writes_the_placeholder_it_names() {
        assert_eq!(PrintFormat::written(2), "{format 2}");
    }

    #[test]
    fn only_debug_is_written_back() {
        assert_eq!(PrintFormat::Display.clause(), None);
        assert_eq!(
            PrintFormat::Debug.clause().as_deref(),
            Some("(format debug)")
        );
    }
}
