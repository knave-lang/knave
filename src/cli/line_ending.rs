#[derive(Clone, Copy)]
pub enum LineEnding {
    Lf,
    Crlf,
}

impl LineEnding {
    const LF: &str = "\n";
    const CRLF: &str = "\r\n";

    pub fn of_first_line(written: &str) -> Self {
        match written.split_once(Self::LF) {
            Some((first, _)) if first.ends_with('\r') => Self::Crlf,
            _ => Self::Lf,
        }
    }

    pub fn with_lf_endings(written: &[u8]) -> String {
        String::from_utf8_lossy(written).replace(Self::CRLF, Self::LF)
    }

    pub fn text(self) -> &'static str {
        match self {
            Self::Lf => Self::LF,
            Self::Crlf => Self::CRLF,
        }
    }
}
