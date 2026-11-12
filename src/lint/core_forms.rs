use std::mem;

use super::binding_symbols::BindingSymbols;
use super::composite_forms::CompositeForms;
use super::form_reader::FormReader;
use super::pattern_forms::PatternForms;
use super::read_back_definition::ReadBackDefinition;
use super::read_back_table::ReadBackTable;
use super::read_back_writer::ReadBackWriter;
use super::spanned_forms::SpannedForms;
use crate::core::{
    Arm, Binder, BindingArena, BindingId, Block, BodyItem, Control, ControlForm, DefinitionForm,
    Expr, ExprKind, ExpressionForm, KernelForm, Lambda, Leaf, LeafForm,
};
use crate::reader::Form;
use crate::source::Span;
use crate::stack::DeepRecursion;

pub struct CoreForms<'a, 's> {
    arena: &'a BindingArena,
    symbols: &'a mut BindingSymbols<'s>,
    table: ReadBackTable,
}

impl<'a, 's> CoreForms<'a, 's> {
    pub fn new(arena: &'a BindingArena, symbols: &'a mut BindingSymbols<'s>) -> Self {
        Self {
            arena,
            symbols,
            table: ReadBackTable::default(),
        }
    }

    pub fn definition(&mut self, binding: BindingId, value: &Expr) -> ReadBackDefinition {
        let span = value.span();
        let row = self.writer().open();
        let parts = vec![
            self.kernel_head(KernelForm::Definition(DefinitionForm::Def), span),
            self.writer().binder(binding, span),
            self.expr(value),
        ];
        let form = self.writer().close(row, parts, span);
        ReadBackDefinition {
            name: self.arena.get(binding).shared_name(),
            form,
            table: mem::take(&mut self.table),
        }
    }

    fn headed<'e>(
        &mut self,
        form: ExpressionForm,
        arguments: impl IntoIterator<Item = &'e Expr>,
        span: Span,
    ) -> Form {
        let row = self.writer().open();
        let mut parts = vec![self.kernel_head(KernelForm::Expression(form), span)];
        parts.extend(arguments.into_iter().map(|argument| self.expr(argument)));
        self.writer().close(row, parts, span)
    }

    fn expr_of_kind(&mut self, kind: &ExprKind, span: Span) -> Form {
        match kind {
            ExprKind::Leaf(leaf) => self.leaf(leaf, span),
            ExprKind::Fn(lambda) => self.lambda(lambda, span),
            ExprKind::Control(control) => self.control(control, span),
            ExprKind::Recur(arguments) => {
                self.headed(ExpressionForm::Recur, arguments.iter(), span)
            }
            ExprKind::Try(value) => self.headed(ExpressionForm::Try, [value.as_ref()], span),
            ExprKind::Infer(value) => self.headed(ExpressionForm::Infer, [value.as_ref()], span),
            other => self.wrapping_kind(other, span),
        }
    }

    fn wrapping_kind(&mut self, kind: &ExprKind, span: Span) -> Form {
        match kind {
            ExprKind::Borrow(kind, value) => {
                self.headed(ExpressionForm::Borrow(*kind), [value.as_ref()], span)
            }
            ExprKind::Deref(value) => self.headed(ExpressionForm::Deref, [value.as_ref()], span),
            other => CompositeForms::read(self, other, span),
        }
    }

    fn leaf(&mut self, leaf: &Leaf, span: Span) -> Form {
        match leaf {
            Leaf::Literal(literal) => self.writer().leaf(SpannedForms::literal(literal, span)),
            Leaf::Var(binding) => self.writer().reference(*binding, None, span),
            Leaf::Quote(datum) => {
                let row = self.writer().open();
                let mut stripped = datum.clone();
                stripped.retain_scopes(&|_| false);
                let parts = vec![
                    SpannedForms::expression_head(ExpressionForm::Leaf(LeafForm::Quote), span),
                    stripped,
                ];
                self.writer().close(row, parts, span)
            }
            Leaf::Rust(template) => {
                let row = self.writer().open();
                let head = ExpressionForm::Leaf(LeafForm::Rust);
                let parts = vec![
                    self.kernel_head(KernelForm::Expression(head), span),
                    self.writer()
                        .leaf(SpannedForms::string(&template.template, span)),
                ];
                self.writer().close(row, parts, span)
            }
        }
    }

    fn lambda(&mut self, lambda: &Lambda, span: Span) -> Form {
        let row = self.writer().open();
        let head = self.kernel_head(KernelForm::Expression(ExpressionForm::Fn), span);
        let parameters_row = self.writer().open();
        let parameters = lambda
            .params
            .iter()
            .map(|parameter| self.writer().binder(parameter.binding, parameter.span))
            .collect();
        let parameters = self.writer().close(parameters_row, parameters, span);
        let mut parts = vec![head, parameters];
        parts.extend(self.block_items(&lambda.body));
        self.writer().close(row, parts, span)
    }

    fn control(&mut self, control: &Control, span: Span) -> Form {
        match control {
            Control::Block(block) => {
                let row = self.writer().open();
                let head = ExpressionForm::Control(ControlForm::Do);
                let mut parts = vec![self.kernel_head(KernelForm::Expression(head), span)];
                parts.extend(self.block_items(block));
                self.writer().close(row, parts, span)
            }
            Control::If(test, then, otherwise) => {
                let branches = [test.as_ref(), then.as_ref(), otherwise.as_ref()];
                self.headed(ExpressionForm::Control(ControlForm::If), branches, span)
            }
            Control::Match(value, arms) => {
                let row = self.writer().open();
                let mut parts = vec![
                    self.kernel_head(KernelForm::Expression(ExpressionForm::Match), span),
                    self.expr(value),
                ];
                parts.extend(arms.iter().map(|arm| self.arm(arm)));
                self.writer().close(row, parts, span)
            }
        }
    }

    fn block_items(&mut self, block: &Block) -> Vec<Form> {
        block
            .items
            .iter()
            .map(|item| match item {
                BodyItem::Expr(expr) => self.expr(expr),
                BodyItem::Let(binder, value) => self.local(binder, value),
            })
            .collect()
    }

    fn local(&mut self, binder: &Binder, value: &Expr) -> Form {
        let span = Self::binder_through_value(binder.span, value.span());
        let row = self.writer().open();
        let parts = vec![
            self.kernel_head(KernelForm::Local, binder.span),
            self.writer().binder(binder.binding, binder.span),
            self.expr(value),
        ];
        self.writer().close(row, parts, span)
    }

    fn binder_through_value(binder: Span, value: Span) -> Span {
        if binder.source() == value.source() && binder.range().start <= value.range().end {
            Span::new(binder.source(), binder.range().start..value.range().end)
        } else {
            value
        }
    }

    fn arm(&mut self, arm: &Arm) -> Form {
        let row = self.writer().open();
        let arena = self.arena;
        let pattern = PatternForms::new(arena, self.writer()).pattern(&arm.pattern);
        let mut parts = vec![pattern];
        parts.extend(self.block_items(&arm.body));
        self.writer().close(row, parts, arm.span)
    }
}

impl<'s> FormReader<'s> for CoreForms<'_, 's> {
    fn writer(&mut self) -> ReadBackWriter<'_, 's> {
        ReadBackWriter::new(self.arena, self.symbols, &mut self.table)
    }

    fn expr(&mut self, expr: &Expr) -> Form {
        DeepRecursion::guard(|| self.expr_of_kind(expr.kind(), expr.span()))
    }

    fn kernel_head(&mut self, form: KernelForm, span: Span) -> Form {
        self.writer().leaf(SpannedForms::kernel_head(form, span))
    }
}

#[cfg(test)]
mod tests {
    use crate::lint::expanded_source::ExpandedSource;
    use crate::reader::Form;

    struct ReadBackFixture;

    impl ReadBackFixture {
        fn quoted_datum(form: &Form) -> Option<Form> {
            let items = form.as_list()?;
            let is_quote = items
                .first()
                .and_then(Form::as_symbol)
                .is_some_and(|head| head.name() == "quote");
            if is_quote {
                return items.get(1).cloned();
            }
            items.iter().find_map(Self::quoted_datum)
        }
    }

    #[test]
    fn a_body_reads_back_as_do_and_local() {
        let mut source = ExpandedSource::expand("(defn area (width) (let (unused 1) width))");
        assert_eq!(
            source.read_back("area"),
            "(def area (fn (width) (do (local unused 1) width)))"
        );
    }

    #[test]
    fn a_match_reads_back_with_its_patterns() {
        let mut source = ExpandedSource::expand(
            "(defenum shape (circle i64) (labeled :size i64) dot)\n\
             (defn area (form) (match form ((circle radius) radius) ((labeled :size size) size) (dot 0) (_ 1)))",
        );
        assert_eq!(
            source.read_back("area"),
            "(def area (fn (form) (match form ((circle radius) radius) ((labeled :size size) size) (dot 0) (_ 1))))"
        );
    }

    #[test]
    fn a_struct_reads_back_as_new_and_field() {
        let mut source = ExpandedSource::expand(
            "(defstruct point :x i64 :y i64)\n\
             (defn moved (base) (:x (assoc base :x 1)))",
        );
        assert_eq!(
            source.read_back("moved"),
            "(def moved (fn (base) (field (new _ base (x 1)) x)))"
        );
    }

    #[test]
    fn constructions_and_a_literal_pattern_read_back() {
        let mut source = ExpandedSource::expand(
            "(defenum shape (circle i64) (labeled :size i64))\n\
             (defstruct point :x i64)\n\
             (defn made (n) (match n (0 (shape/circle 1)) (_ (shape/labeled :size (:x (->point n))))))",
        );
        assert_eq!(
            source.read_back("made"),
            "(def made (fn (n) (match n (0 (shape/circle 1)) (_ (shape/labeled (size (field (make-point n) x)))))))"
        );
    }

    #[test]
    fn a_quote_keeps_its_datum() {
        let mut source = ExpandedSource::expand(
            "(use code value-eq)\n(defn total? (name) (value-eq name (quote total)))",
        );
        assert_eq!(
            source.read_back("total?"),
            "(def total? (fn (name) (value-eq name (quote total))))"
        );
    }

    #[test]
    fn references_and_propagation_read_back_as_kernel_forms() {
        let mut source = ExpandedSource::expand(
            "(defn parsed (text) (ok (deref (ref (try (parse text))))))\n\
             (type parsed (fn ((ref str)) (result i64 (parse-error i64))))",
        );
        assert_eq!(
            source.read_back("parsed"),
            "(def parsed (fn (text) (result/ok (deref (ref (try (parse text)))))))"
        );
    }

    #[test]
    fn collections_read_back_with_their_keys() {
        let mut source = ExpandedSource::expand(
            "(defn grown (numbers) (local more (assoc numbers 0 1)) (push! more 2) (set! (at more 0) 3) more)",
        );
        assert_eq!(
            source.read_back("grown"),
            "(def grown (fn (numbers) (local more (assoc-at numbers (0) 1)) (map-at push more 2) (set-at more (0) 3) more))"
        );
    }

    #[test]
    fn a_loop_reads_back_as_recur_and_if() {
        let mut source =
            ExpandedSource::expand("(defn down (count) (if (= count 0) true (recur (- count 1))))");
        assert_eq!(
            source.read_back("down"),
            "(def down (fn (count) (if (equal count 0) true (recur (sub count 1)))))"
        );
    }

    #[test]
    fn a_quoted_symbol_carries_no_scope_and_names_no_binding() {
        let mut source = ExpandedSource::expand(
            "(use code value-eq)\n(defn total? (total) (value-eq total (quote total)))",
        );
        let (definition, names) = source.read_back_named("total?");
        let quoted = ReadBackFixture::quoted_datum(&definition.form).expect("the form quotes");
        let symbol = quoted.as_symbol().expect("the datum is a symbol");
        assert!(!symbol.scopes().any(|_| true));
        assert_eq!(names.binding_of(symbol), None);
    }
}
