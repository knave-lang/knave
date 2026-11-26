use super::signature_owner::SignatureOwner;
use super::text_place::TextPlace;
use super::unheld_binder::UnheldBinder;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub(super) struct BoundOnlyProblems;

pub(super) struct NamedUnheld<'a> {
    pub(super) unheld: &'a UnheldBinder,
    pub(super) trait_name: Option<&'a str>,
    pub(super) form: Span,
    pub(super) text: Option<TextPlace>,
}

impl BoundOnlyProblems {
    pub(super) fn unheld_binders(
        unheld: &[NamedUnheld<'_>],
        span: Span,
        owner: &SignatureOwner,
    ) -> Diagnostic {
        let binders: Vec<&str> = unheld
            .iter()
            .map(|named| &**named.unheld.binder())
            .collect();
        let diagnostic = Diagnostic::error(
            DiagnosticCode::CANNOT_INFER,
            Self::title(&binders),
            Label::new(
                span,
                format!(
                    "{} can fix {}",
                    Self::user(owner),
                    Self::listed(&binders, "or")
                ),
            ),
        );
        (unheld.iter())
            .fold(diagnostic, |diagnostic, named| {
                diagnostic.with_label(Label::new(named.form, Self::naming(named)))
            })
            .with_help(Self::help(&binders, owner))
    }

    fn user(owner: &SignatureOwner) -> &'static str {
        match owner {
            SignatureOwner::Function => "no call of this function",
            SignatureOwner::Template => "no use of this template",
        }
    }

    fn title(binders: &[&str]) -> String {
        format!(
            "no parameter and no result holds {}",
            Self::listed(binders, "or")
        )
    }

    fn naming(named: &NamedUnheld<'_>) -> String {
        let trait_name = named.trait_name.unwrap_or_default();
        let binder = named.unheld.binder();
        match (named.unheld, &named.text) {
            (_, Some(TextPlace::Template)) => format!("the template's text names `{binder}`"),
            (_, Some(TextPlace::Clause)) => format!("a clause's text names `{binder}`"),
            (UnheldBinder::InArgument { holder, .. }, None) => {
                format!("the `{trait_name}` bound of `{holder}` names `{binder}`")
            }
            (UnheldBinder::Bare { .. }, None) => format!("`{binder}` is declared here"),
            (UnheldBinder::Implementor { .. }, None) => {
                format!("`{binder}` takes the `{trait_name}` bound here")
            }
        }
    }

    fn help(binders: &[&str], owner: &SignatureOwner) -> String {
        let removal = match (owner, binders) {
            (SignatureOwner::Template, _) => "",
            (SignatureOwner::Function, [_]) => ", or remove it from the `where`",
            (SignatureOwner::Function, _) => ", or remove them from the `where`",
        };
        match binders {
            [binder] => format!(
                "the signature must change: name `{binder}` in a parameter or the result{removal}"
            ),
            _ => format!(
                "the signature must change: name {} in parameters or the result{removal}",
                Self::listed(binders, "and")
            ),
        }
    }

    fn listed(binders: &[&str], last_joiner: &str) -> String {
        let quoted: Vec<String> = binders.iter().map(|binder| format!("`{binder}`")).collect();
        match quoted.split_last() {
            Some((last, [])) => last.clone(),
            Some((last, rest)) => format!("{} {last_joiner} {last}", rest.join(", ")),
            None => String::new(),
        }
    }
}
