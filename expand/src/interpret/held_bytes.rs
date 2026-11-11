use crate::reader::{Form, FormKind};

pub struct HeldBytes;

impl HeldBytes {
    pub const PER_FORM: usize = 64;

    pub fn of_form(form: &Form) -> usize {
        let mut held = 0;
        let mut waiting = vec![form];
        while let Some(form) = waiting.pop() {
            held += Self::PER_FORM + Self::text_bytes(form.kind());
            waiting.extend(form.as_list().unwrap_or_default());
        }
        held
    }

    fn text_bytes(kind: &FormKind) -> usize {
        match kind {
            FormKind::String(text) => text.len(),
            FormKind::Symbol(symbol) => symbol.name().len(),
            FormKind::Number(number) => number.text().len(),
            FormKind::List(_) => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HeldBytes;
    use crate::reader::{Form, FormKind, Symbol};
    use crate::source::{SourceId, Span};

    struct HeldForms;

    impl HeldForms {
        fn span() -> Span {
            Span::new(SourceId::new(0), 0..1)
        }

        fn text(bytes: usize) -> Form {
            Form::new(FormKind::String("x".repeat(bytes).into()), Self::span())
        }
    }

    #[test]
    fn text_holds_its_bytes_beside_its_form() {
        assert_eq!(HeldBytes::of_form(&HeldForms::text(100)), 64 + 100);
    }

    #[test]
    fn a_list_holds_itself_and_each_form_of_its_tree() {
        let symbol = Form::new(FormKind::Symbol(Symbol::new("ab")), HeldForms::span());
        let list = Form::new(
            FormKind::List(vec![symbol, HeldForms::text(3)]),
            HeldForms::span(),
        );
        assert_eq!(HeldBytes::of_form(&list), 64 + (64 + 2) + (64 + 3));
    }
}
