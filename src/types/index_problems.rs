use super::placeholder_site::PlaceholderSite;
use crate::core::Conversion;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct IndexProblems;

impl IndexProblems {
    pub fn negative_index(value: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::NEGATIVE_INDEX,
            format!("the value `{value}` is negative"),
            Label::new(span, "an index or a size is at least 0"),
        )
        .with_help("count the position from the front, starting at 0")
    }

    pub fn unconverted_index_parameter(site: &PlaceholderSite) -> Diagnostic {
        let converted = Self::converted();
        Self::malformed_index_placeholder(
            site,
            format!(
                "`{}` converts an `{converted}`, and this parameter is not `{converted}`",
                Self::written(site)
            ),
        )
        .with_help(format!("write the parameter's type as `{converted}`"))
    }

    pub fn missing_index_parameter(site: &PlaceholderSite) -> Diagnostic {
        let converted = Self::converted();
        Self::malformed_index_placeholder(
            site,
            format!(
                "`{}` converts an `{converted}`, and the template takes no such parameter",
                Self::written(site)
            ),
        )
        .with_help(format!(
            "write an `{converted}` parameter for it in the template's type"
        ))
    }

    pub fn optional_index_without_option(site: &PlaceholderSite) -> Diagnostic {
        Self::malformed_index_placeholder(
            site,
            format!(
                "`{}` turns a negative index into one past the end, so the result must be an option",
                Self::written(site)
            ),
        )
        .with_help(format!(
            "return an option, or write `{}`, which panics on a negative index",
            Conversion::Index.placeholder(site.position)
        ))
    }

    fn malformed_index_placeholder(site: &PlaceholderSite, message: String) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::MALFORMED_TEMPLATE,
            "this Rust template has a malformed placeholder",
            Label::new(site.span, message),
        )
    }

    fn written(site: &PlaceholderSite) -> String {
        site.conversion.placeholder(site.position)
    }

    fn converted() -> &'static str {
        Conversion::ARGUMENT.names().shown()
    }
}
