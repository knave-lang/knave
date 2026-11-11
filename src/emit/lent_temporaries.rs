use std::borrow::Cow;
use std::rc::Rc;

use super::binding_value::BindingValue;
use super::lifted_temporary::LiftedTemporary;
use super::lifted_value::LiftedValue;
use super::tree_position::TreePosition;
use crate::core::{RustTemplate, TypeExpr, TypeExprKind};
use crate::types::{
    Borrow, TypedArgument, TypedExpr, TypedExprKind, TypedLeaf, TypedProgram, TypedStructExpr,
};

pub struct LentTemporaries<'a, 'n> {
    program: &'a TypedProgram,
    names: &'n mut dyn FnMut(&str) -> String,
}

impl<'a, 'n> LentTemporaries<'a, 'n> {
    pub fn new(program: &'a TypedProgram, names: &'n mut dyn FnMut(&str) -> String) -> Self {
        Self { program, names }
    }

    pub fn lift<'v, T: BindingValue>(self, value: &'v T) -> LiftedValue<'v, T> {
        let lifts = TemporarySearch::new(self.program).lifts(value.expression());
        if lifts.is_empty() {
            return LiftedValue::new(Vec::new(), Cow::Borrowed(value));
        }
        let mut rewritten = value.expression().clone();
        let temporaries = lifts
            .iter()
            .map(|lift| lift.take(&mut rewritten, &mut *self.names))
            .collect();
        LiftedValue::new(temporaries, Cow::Owned(value.with_expression(rewritten)))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TemporaryScope {
    Statement,
    Binding,
}

#[derive(Clone, Copy)]
enum LiftKind {
    Owner,
    Earlier,
}

impl LiftKind {
    fn name(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Earlier => "earlier",
        }
    }
}

struct Lift {
    position: TreePosition,
    kind: LiftKind,
}

impl Lift {
    fn take(&self, root: &mut TypedExpr, names: &mut dyn FnMut(&str) -> String) -> LiftedTemporary {
        let name = names(self.kind.name());
        let node = self.position.node_mut(root);
        let named = Self::named(&name, node);
        LiftedTemporary::new(name, std::mem::replace(node, named))
    }

    fn named(name: &str, node: &TypedExpr) -> TypedExpr {
        let template = RustTemplate {
            template: Rc::from(name),
            type_expr: TypeExpr::new(TypeExprKind::Hole, node.span),
            clauses: Vec::new(),
        };
        TypedExpr {
            kind: TypedExprKind::Leaf(TypedLeaf::Rust(Rc::new(template))),
            ty: node.ty.clone(),
            span: node.span,
        }
    }
}

struct TemporarySearch<'p> {
    program: &'p TypedProgram,
    position: TreePosition,
    lifts: Vec<Lift>,
    evaluated: Vec<TreePosition>,
}

impl<'p> TemporarySearch<'p> {
    fn new(program: &'p TypedProgram) -> Self {
        Self {
            program,
            position: TreePosition::default(),
            lifts: Vec::new(),
            evaluated: Vec::new(),
        }
    }

    fn lifts(mut self, value: &TypedExpr) -> Vec<Lift> {
        self.visit(value, TemporaryScope::Binding);
        self.lifts
    }

    fn visit(&mut self, expr: &TypedExpr, scope: TemporaryScope) {
        if !self.program.type_holds_borrow(&expr.ty) {
            return;
        }
        match &expr.kind {
            TypedExprKind::Call(_, arguments) => self.arguments(expr, arguments),
            TypedExprKind::Borrow(..)
            | TypedExprKind::Struct(TypedStructExpr::New(..))
            | TypedExprKind::Variant(..) => self.operands(expr, scope),
            TypedExprKind::Try { .. } => self.operands(expr, TemporaryScope::Statement),
            _ => {}
        }
    }

    fn arguments(&mut self, call: &TypedExpr, arguments: &[TypedArgument]) {
        for argument in arguments {
            if Self::lends_temporary(argument) {
                self.lift(call, &argument.value);
            } else {
                self.operand(call, &argument.value, TemporaryScope::Statement);
            }
        }
    }

    fn lends_temporary(argument: &TypedArgument) -> bool {
        argument.borrow == Borrow::Shared
            && !argument.value.ty.is_reference()
            && Self::is_temporary(&argument.value)
    }

    fn operands(&mut self, expr: &TypedExpr, scope: TemporaryScope) {
        for operand in expr.children() {
            self.operand(expr, operand, scope);
        }
    }

    fn operand(&mut self, parent: &TypedExpr, operand: &TypedExpr, scope: TemporaryScope) {
        self.position.enter(parent, operand);
        if let Some(value) = Self::borrowed_temporary(operand)
            && scope == TemporaryScope::Statement
        {
            self.lift(operand, value);
        } else {
            self.evaluate(operand, scope);
        }
        self.position.leave();
    }

    fn borrowed_temporary(operand: &TypedExpr) -> Option<&TypedExpr> {
        match &operand.kind {
            TypedExprKind::Borrow(_, value) if Self::is_temporary(value) => Some(value),
            _ => None,
        }
    }

    fn evaluate(&mut self, operand: &TypedExpr, scope: TemporaryScope) {
        let lifts_before = self.lifts.len();
        let evaluated_before = self.evaluated.len();
        self.visit(operand, scope);
        if self.lifts.len() > lifts_before {
            self.evaluated.clear();
        } else {
            self.evaluated.truncate(evaluated_before);
        }
        if !Self::is_pure(operand) {
            self.evaluated.push(self.position.clone());
        }
    }

    fn lift(&mut self, parent: &TypedExpr, temporary: &TypedExpr) {
        let earlier = self.evaluated.drain(..).map(|position| Lift {
            position,
            kind: LiftKind::Earlier,
        });
        self.lifts.extend(earlier);
        self.position.enter(parent, temporary);
        self.visit(temporary, TemporaryScope::Binding);
        self.evaluated.clear();
        self.lifts.push(Lift {
            position: self.position.clone(),
            kind: LiftKind::Owner,
        });
        self.position.leave();
    }

    fn is_temporary(value: &TypedExpr) -> bool {
        !Self::is_pure(value)
    }

    fn is_pure(value: &TypedExpr) -> bool {
        match &value.kind {
            TypedExprKind::Leaf(_) | TypedExprKind::Fn(_) | TypedExprKind::Deref(_) => true,
            TypedExprKind::Borrow(_, inner) => Self::is_pure(inner),
            _ => value.is_place(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LentTemporaries;
    use crate::types::{
        TypedBodyItem, TypedExpr, TypedExprKind, TypedLeaf, TypedProgram, TypedSource,
    };

    struct LiftFixture {
        program: TypedProgram,
    }

    impl LiftFixture {
        const SOURCE: &str = "
            (defstruct label :text (ref str))
            (defstruct page :title string)
            (type first-letter (fn ((ref str)) (ref str)))
            (defn first-letter (text) (subs text 0 1))
            (type title-of (fn ((ref page)) (ref str)))
            (defn title-of (page) (as-str (:title page)))
            (type announced (fn ((ref str)) string))
            (defn announced (text) (str text))
            (type prefix (fn (usize (ref str)) (ref str)))
            (defn prefix (size text) (subs text 0 size))
            (defn as-str-argument (name)
              (local letter (first-letter (as-str (str name \"-a\"))))
              (len letter))
            (defn owned-result (name)
              (local size (len (as-str (str name \"-b\"))))
              size)
            (defn borrowed-place (name)
              (local owned (str name \"-c\"))
              (local letter (first-letter (as-str owned)))
              (len letter))
            (defn written-borrow (name)
              (local text (ref (str name \"-d\")))
              (len text))
            (defn written-borrow-argument (name)
              (local letter (title-of (ref (->page :title (str name \"-e\")))))
              (len letter))
            (defn borrowed-argument (name)
              (local letter (title-of (->page :title (str name \"-f\"))))
              (len letter))
            (defn struct-field (name)
              (local tag (->label :text (as-str (str name \"-g\"))))
              (len (:text tag)))
            (defn variant-payload (name)
              (local found (some (as-str (str name \"-h\"))))
              (len (unwrap found)))
            (defn earlier-argument ()
              (local shown (prefix (len (announced \"first\")) (as-str (announced \"second\"))))
              (len shown))
            (defn earlier-place (size)
              (local shown (prefix size (as-str (announced \"third\"))))
              (len shown))
            (defstruct shelf :page (ref page))
            (defn struct-written-borrow (name)
              (local held (->shelf :page (ref (->page :title (str name \"-j\")))))
              (len (:title (:page held))))
            (defn variant-written-borrow (name)
              (local found (some (ref (->page :title (str name \"-k\")))))
              (len (:title (unwrap found))))
            (type checked (fn ((ref str)) (result (ref str) io-error)))
            (defn checked (text) (ok text))
            (defn tried (name)
              (local letter (try (checked (as-str (str name \"-i\")))))
              (ok (len letter)))
            (type outer (fn (usize (ref str) (ref str)) (ref str)))
            (defn outer (size first second) (subs first 0 (min size (len second))))
            (type cut (fn ((ref str) usize) (ref str)))
            (defn cut (text size) (subs text 0 size))
            (defn nested-earlier ()
              (local shown
                (outer (len (announced \"a\"))
                       (cut (as-str (announced \"c\")) (len (announced \"d\")))
                       (as-str (announced \"e\"))))
              (len shown))
            (type triple (fn (usize usize (ref str)) (ref str)))
            (defn triple (low high text) (subs text low high))
            (defn two-earlier ()
              (local shown (triple (len (announced \"a\")) (len (announced \"b\")) (as-str (announced \"c\"))))
              (len shown))
            (type field-letter (fn ((ref page)) (ref str)))
            (defn field-letter (page)
              (local letter (first-letter (as-str (:title page))))
              letter)
            (type page-prefix (fn ((ref page) (ref str)) (ref str)))
            (defn page-prefix (page text) (subs text 0 (len (:title page))))
            (defn borrowed-local (name)
              (local page (->page :title (str name)))
              (local shown (page-prefix (ref page) (as-str (announced \"x\"))))
              (len shown))
            (defn two-written-borrows (name)
              (local shown
                (page-prefix (ref (->page :title (str name))) (as-str (announced \"y\"))))
              (len shown))
            (defstruct card :page (ref page) :text (ref str))
            (defn extended-earlier (name)
              (local held
                (->card :page (ref (->page :title (str name))) :text (as-str (announced \"z\"))))
              (len (:text held)))
            (defn owned-field (name)
              (local page (->page :title (str name)))
              (local letter (first-letter (as-str (:title page))))
              (len letter))
            (type label-text (fn ((ref label)) (ref str)))
            (defn label-text (tag) (:text tag))
            (defn lent-inside-lifted (name)
              (local text (label-text (ref (->label :text (as-str (str name \"-l\"))))))
              (len text))
            (defn written-borrow-of-struct (name)
              (local tag (ref (->label :text (as-str (str name \"-m\")))))
              (len (:text tag)))
            (defstruct folder :page page)
            (defn lent-field (name)
              (local folder (->folder :page (->page :title (str name))))
              (local text (title-of (ref (:page folder))))
              (len text))
            (type label-beside (fn ((ref label) (ref str)) (ref str)))
            (defn label-beside (tag text) (subs text 0 (len (:text tag))))
            (defn lifted-beside-lifted ()
              (local text
                (label-beside (ref (->label :text (cut (as-str (announced \"p\")) (len (announced \"q\")))))
                              (as-str (announced \"r\"))))
              (len text))
            (defn main ())";

        fn new() -> Self {
            Self {
                program: TypedSource::expand(Self::SOURCE).typed(),
            }
        }

        fn lifted(&self, function: &str) -> Vec<String> {
            let mut names = |base: &str| base.to_string();
            let lifted =
                LentTemporaries::new(&self.program, &mut names).lift(self.last_let_value(function));
            lifted
                .temporaries()
                .iter()
                .map(|temporary| format!("{} {}", temporary.name(), self.text(temporary.value())))
                .collect()
        }

        fn rewritten(&self, function: &str) -> String {
            let mut names = |base: &str| base.to_string();
            let lifted =
                LentTemporaries::new(&self.program, &mut names).lift(self.last_let_value(function));
            Self::named_leaves(lifted.value()).join(" ")
        }

        fn named_leaves(expr: &TypedExpr) -> Vec<String> {
            match &expr.kind {
                TypedExprKind::Leaf(TypedLeaf::Rust(template)) => {
                    vec![template.template.to_string()]
                }
                _ => expr
                    .children()
                    .into_iter()
                    .flat_map(Self::named_leaves)
                    .collect(),
            }
        }

        fn last_let_value(&self, function: &str) -> &TypedExpr {
            let body = self
                .program
                .modules
                .iter()
                .flat_map(|module| &module.definitions)
                .filter(|definition| {
                    self.program.bindings.get(definition.binding).name() == function
                })
                .find_map(|definition| match &definition.value.kind {
                    TypedExprKind::Fn(lambda) => Some(&lambda.body),
                    _ => None,
                })
                .expect("the fixture defines the function");
            body.items
                .iter()
                .rev()
                .find_map(|item| match item {
                    TypedBodyItem::Let(_, value) => Some(value),
                    TypedBodyItem::Expr(_) => None,
                })
                .expect("the function binds a local")
        }

        fn text(&self, expr: &TypedExpr) -> &'static str {
            &Self::SOURCE[expr.span.range()]
        }
    }

    #[test]
    fn lifts_a_string_lent_through_as_str() {
        assert_eq!(
            LiftFixture::new().lifted("as-str-argument"),
            ["owner (str name \"-a\")"]
        );
    }

    #[test]
    fn replaces_the_lifted_temporary_with_its_name() {
        assert_eq!(LiftFixture::new().rewritten("as-str-argument"), "owner");
    }

    #[test]
    fn keeps_a_temporary_whose_borrow_ends_in_the_statement() {
        assert!(LiftFixture::new().lifted("owned-result").is_empty());
    }

    #[test]
    fn keeps_a_borrowed_local() {
        assert!(LiftFixture::new().lifted("borrowed-place").is_empty());
    }

    #[test]
    fn keeps_a_written_borrow_that_rust_extends_to_the_binding() {
        assert!(LiftFixture::new().lifted("written-borrow").is_empty());
    }

    #[test]
    fn lifts_a_written_borrow_passed_to_a_call() {
        assert_eq!(
            LiftFixture::new().lifted("written-borrow-argument"),
            ["owner (->page :title (str name \"-e\"))"]
        );
    }

    #[test]
    fn lifts_a_struct_the_call_borrows() {
        assert_eq!(
            LiftFixture::new().lifted("borrowed-argument"),
            ["owner (->page :title (str name \"-f\"))"]
        );
    }

    #[test]
    fn lifts_a_temporary_lent_to_a_struct_field() {
        assert_eq!(
            LiftFixture::new().lifted("struct-field"),
            ["owner (str name \"-g\")"]
        );
    }

    #[test]
    fn lifts_a_temporary_lent_to_a_variant_payload() {
        assert_eq!(
            LiftFixture::new().lifted("variant-payload"),
            ["owner (str name \"-h\")"]
        );
    }

    #[test]
    fn lifts_an_earlier_argument_first_to_keep_the_written_order() {
        assert_eq!(
            LiftFixture::new().lifted("earlier-argument"),
            [
                "earlier (len (announced \"first\"))",
                "owner (announced \"second\")"
            ]
        );
    }

    #[test]
    fn leaves_an_earlier_local_in_place() {
        assert_eq!(
            LiftFixture::new().lifted("earlier-place"),
            ["owner (announced \"third\")"]
        );
    }

    #[test]
    fn keeps_a_written_borrow_in_a_struct_field_that_rust_extends() {
        assert!(
            LiftFixture::new()
                .lifted("struct-written-borrow")
                .is_empty()
        );
    }

    #[test]
    fn keeps_a_written_borrow_in_a_variant_payload_that_rust_extends() {
        assert!(
            LiftFixture::new()
                .lifted("variant-written-borrow")
                .is_empty()
        );
    }

    #[test]
    fn lifts_a_temporary_lent_through_a_tried_call() {
        assert_eq!(
            LiftFixture::new().lifted("tried"),
            ["owner (str name \"-i\")"]
        );
    }

    #[test]
    fn lifts_an_earlier_argument_that_holds_a_lifted_temporary_as_a_whole() {
        assert_eq!(
            LiftFixture::new().lifted("nested-earlier"),
            [
                "earlier (len (announced \"a\"))",
                "owner (announced \"c\")",
                "earlier (cut (as-str (announced \"c\")) (len (announced \"d\")))",
                "owner (announced \"e\")"
            ]
        );
    }

    #[test]
    fn lifts_every_earlier_argument() {
        assert_eq!(
            LiftFixture::new().lifted("two-earlier"),
            [
                "earlier (len (announced \"a\"))",
                "earlier (len (announced \"b\"))",
                "owner (announced \"c\")"
            ]
        );
    }

    #[test]
    fn keeps_a_borrowed_field_of_a_parameter() {
        assert!(LiftFixture::new().lifted("field-letter").is_empty());
    }

    #[test]
    fn leaves_an_earlier_borrow_of_a_local_in_place() {
        assert_eq!(
            LiftFixture::new().lifted("borrowed-local"),
            ["owner (announced \"x\")"]
        );
    }

    #[test]
    fn lifts_a_written_borrow_without_an_earlier_binding_for_it() {
        assert_eq!(
            LiftFixture::new().lifted("two-written-borrows"),
            [
                "owner (->page :title (str name))",
                "owner (announced \"y\")"
            ]
        );
    }

    #[test]
    fn lifts_an_earlier_extended_borrow_to_keep_the_written_order() {
        assert_eq!(
            LiftFixture::new().lifted("extended-earlier"),
            [
                "earlier (ref (->page :title (str name)))",
                "owner (announced \"z\")"
            ]
        );
    }

    #[test]
    fn keeps_a_borrowed_field_of_an_owned_local() {
        assert!(LiftFixture::new().lifted("owned-field").is_empty());
    }

    #[test]
    fn lifts_a_temporary_that_a_lifted_value_lends_first() {
        assert_eq!(
            LiftFixture::new().lifted("lent-inside-lifted"),
            [
                "owner (str name \"-l\")",
                "owner (->label :text (as-str (str name \"-l\")))"
            ]
        );
    }

    #[test]
    fn lifts_a_temporary_lent_inside_a_written_borrow_rust_extends() {
        assert_eq!(
            LiftFixture::new().lifted("written-borrow-of-struct"),
            ["owner (str name \"-m\")"]
        );
    }

    #[test]
    fn keeps_a_written_borrow_of_a_field_of_a_local() {
        assert!(LiftFixture::new().lifted("lent-field").is_empty());
    }

    #[test]
    fn lifts_a_value_after_a_lifted_value_that_holds_a_call() {
        assert_eq!(
            LiftFixture::new().lifted("lifted-beside-lifted"),
            [
                "owner (announced \"p\")",
                "owner (->label :text (cut (as-str (announced \"p\")) (len (announced \"q\"))))",
                "owner (announced \"r\")"
            ]
        );
    }
}
