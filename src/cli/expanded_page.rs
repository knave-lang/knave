use super::exit_status::ExitStatus;
use super::standard_error::StandardError;
use super::token_colors::TokenColors;
use crate::core::{Expr, ExprKind, Leaf, Literal};
use crate::diagnostic::Diagnostics;
use crate::expand::ExpansionSession;
use crate::reader::{Form, FormKind, Symbol};
use crate::source::{SourceId, Span};

pub struct PageText {
    header: String,
    body: String,
}

impl PageText {
    pub fn new(header: String, body: String) -> Self {
        Self { header, body }
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn painted(&self, colors: TokenColors) -> String {
        format!("{}{}", colors.paint(&self.header), self.body)
    }
}

pub struct ExpandedPage {
    header: Expr,
    body: Expr,
}

impl ExpandedPage {
    pub const HEADER_MACRO: &'static str = "doc-header";
    pub const BODY_MACRO: &'static str = "doc-body";

    pub fn new(header: Expr, body: Expr) -> Self {
        Self { header, body }
    }

    pub fn expand(
        session: &mut ExpansionSession<'_>,
        term: &Form,
        source: SourceId,
    ) -> Result<Self, Diagnostics> {
        let header = session.expression(Self::lookup_call(Self::HEADER_MACRO, term, source))?;
        let body = session.expression(Self::lookup_call(Self::BODY_MACRO, term, source))?;
        Ok(Self { header, body })
    }

    pub fn text(&self) -> Result<PageText, ExitStatus> {
        Ok(PageText::new(
            Self::string(&self.header, Self::HEADER_MACRO)?,
            Self::string(&self.body, Self::BODY_MACRO)?,
        ))
    }

    pub fn same_text(&self, other: &Self) -> bool {
        self.literals() == other.literals()
    }

    fn literals(&self) -> [Option<&str>; 2] {
        [Self::literal(&self.header), Self::literal(&self.body)]
    }

    pub fn string(expanded: &Expr, lookup: &str) -> Result<String, ExitStatus> {
        match Self::literal(expanded) {
            Some(text) => Ok(text.to_string()),
            None => {
                StandardError::write_line(format_args!(
                    "error: `{lookup}` did not expand to a string"
                ));
                Err(ExitStatus::ProgramErrors)
            }
        }
    }

    pub fn literal(expanded: &Expr) -> Option<&str> {
        match expanded.kind() {
            ExprKind::Leaf(Leaf::Literal(Literal::String(text))) => Some(text),
            _ => None,
        }
    }

    fn lookup_call(lookup: &str, term: &Form, source: SourceId) -> Form {
        let span = Span::new(source, term.span().range());
        let head = Form::new(FormKind::Symbol(Symbol::new(lookup)), span);
        Form::new(
            FormKind::List(vec![head, Self::relocated(term, source)]),
            span,
        )
    }

    fn relocated(form: &Form, source: SourceId) -> Form {
        let kind = match form.kind() {
            FormKind::List(items) => FormKind::List(
                items
                    .iter()
                    .map(|item| Self::relocated(item, source))
                    .collect(),
            ),
            atom => atom.clone(),
        };
        Form::new(kind, Span::new(source, form.span().range()))
    }
}
