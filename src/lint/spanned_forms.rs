use std::rc::Rc;

use crate::core::{ExpressionForm, KernelForm, Literal};
use crate::reader::{Form, FormKind, Symbol};
use crate::source::Span;

pub struct SpannedForms;

impl SpannedForms {
    pub fn expression_head(form: ExpressionForm, span: Span) -> Form {
        Self::kernel_head(KernelForm::Expression(form), span)
    }

    pub fn kernel_head(form: KernelForm, span: Span) -> Form {
        Self::symbol(form.name(), span)
    }

    pub fn literal(literal: &Literal, span: Span) -> Form {
        match literal {
            Literal::Number(number) => Form::new(FormKind::Number(number.clone()), span),
            Literal::String(text) => Self::string(text, span),
            Literal::Bool(value) => Self::symbol(&value.to_string(), span),
        }
    }

    pub fn string(text: &Rc<str>, span: Span) -> Form {
        Form::new(FormKind::String(Rc::clone(text)), span)
    }

    pub fn symbol(name: &str, span: Span) -> Form {
        Form::new(FormKind::Symbol(Symbol::new(name)), span)
    }

    pub fn list(parts: Vec<Form>, span: Span) -> Form {
        Form::new(FormKind::List(parts), span)
    }
}
