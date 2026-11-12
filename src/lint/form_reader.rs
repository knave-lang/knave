use super::read_back_writer::ReadBackWriter;
use crate::core::{Expr, KernelForm};
use crate::reader::Form;
use crate::source::Span;

pub trait FormReader<'s> {
    fn writer(&mut self) -> ReadBackWriter<'_, 's>;
    fn expr(&mut self, expr: &Expr) -> Form;
    fn kernel_head(&mut self, form: KernelForm, span: Span) -> Form;
}
