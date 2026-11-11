use std::borrow::Cow;
use std::rc::Rc;

use super::binding_value::BindingValue;
use super::lifted_temporary::LiftedTemporary;
use super::lifted_value::LiftedValue;
use super::tree_position::TreePosition;
use crate::core::{RustTemplate, TypeExpr, TypeExprKind};
use crate::source::Span;
use crate::types::{
    Borrow, ClosureKind, Lifetime, Type, TypedArgument, TypedControl, TypedExpr, TypedExprKind,
    TypedLeaf, TypedProgram, TypedStructExpr,
};

pub struct LentTemporaries<'a, 'n> {
    program: &'a TypedProgram,
    names: &'n mut dyn FnMut(&str) -> String,
}

impl<'a, 'n> LentTemporaries<'a, 'n> {
    const OWNER: &'static str = "owner";

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
            .map(|lift| lift.take(&mut rewritten, (self.names)(Self::OWNER)))
            .collect();
        LiftedValue::new(temporaries, Cow::Owned(value.with_expression(rewritten)))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TemporaryScope {
    Statement,
    Binding,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Placement {
    BeforeBinding,
    InPlace,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LendSite {
    Argument,
    WrittenBorrow,
}

struct Lift {
    position: TreePosition,
    placement: Placement,
    site: LendSite,
}

impl Lift {
    fn take(&self, root: &mut TypedExpr, name: String) -> LiftedTemporary {
        let node = self.position.node_mut(root);
        match self.placement {
            Placement::BeforeBinding => {
                let named = Self::named(&name, node);
                LiftedTemporary::bound(name, std::mem::replace(node, named))
            }
            Placement::InPlace => {
                let placeholder = Self::named(&name, node);
                let site = std::mem::replace(node, placeholder);
                *node = self.assigned(&name, site);
                LiftedTemporary::declared(name)
            }
        }
    }

    fn assigned(&self, name: &str, site: TypedExpr) -> TypedExpr {
        let (temporary, reference) = match (self.site, site.kind) {
            (LendSite::WrittenBorrow, TypedExprKind::Borrow(_, temporary)) => (*temporary, site.ty),
            (_, kind) => {
                let temporary = TypedExpr { kind, ..site };
                let reference = Type::Ref(Lifetime::Elided, Box::new(temporary.ty.clone()));
                (temporary, reference)
            }
        };
        let assignment = format!("{{{{ {name} = {{0}}; &{name} }}}}");
        let callee_type = Type::Function(
            ClosureKind::Fn,
            vec![temporary.ty.clone()],
            Box::new(reference.clone()),
        );
        let span = temporary.span;
        let callee = Self::rust_leaf(&assignment, callee_type, span);
        let argument = TypedArgument {
            value: temporary,
            borrow: Borrow::None,
        };
        TypedExpr {
            kind: TypedExprKind::Call(Box::new(callee), vec![argument]),
            ty: reference,
            span,
        }
    }

    fn named(name: &str, node: &TypedExpr) -> TypedExpr {
        Self::rust_leaf(name, node.ty.clone(), node.span)
    }

    fn rust_leaf(text: &str, ty: Type, span: Span) -> TypedExpr {
        let template = RustTemplate {
            template: Rc::from(text),
            type_expr: TypeExpr::new(TypeExprKind::Hole, span),
            clauses: Vec::new(),
        };
        TypedExpr {
            kind: TypedExprKind::Leaf(TypedLeaf::Rust(Rc::new(template))),
            ty,
            span,
        }
    }
}

struct TemporarySearch<'p> {
    program: &'p TypedProgram,
    position: TreePosition,
    lifts: Vec<Lift>,
    unsettled: bool,
}

impl<'p> TemporarySearch<'p> {
    fn new(program: &'p TypedProgram) -> Self {
        Self {
            program,
            position: TreePosition::default(),
            lifts: Vec::new(),
            unsettled: false,
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
            TypedExprKind::Call(callee, arguments) => self.call(expr, callee, arguments),
            TypedExprKind::Borrow(..)
            | TypedExprKind::Struct(TypedStructExpr::New(..))
            | TypedExprKind::Variant(..) => self.operands(expr, scope),
            TypedExprKind::Struct(TypedStructExpr::Field(owner, _))
                if scope == TemporaryScope::Statement && self.lends_owner(expr, owner) =>
            {
                self.lift_child(expr, owner)
            }
            TypedExprKind::Try { .. } | TypedExprKind::Struct(TypedStructExpr::Field(..)) => {
                self.operands(expr, TemporaryScope::Statement)
            }
            TypedExprKind::Control(control) => self.control(expr, control, scope),
            _ => {}
        }
    }

    fn call(&mut self, call: &TypedExpr, callee: &TypedExpr, arguments: &[TypedArgument]) {
        self.unsettled |= !Self::is_settled_callee(callee);
        for argument in arguments {
            if Self::lends_temporary(argument) {
                self.lift_child(call, &argument.value);
            } else {
                self.operand(call, &argument.value, TemporaryScope::Statement);
            }
        }
    }

    fn lends_owner(&self, field: &TypedExpr, owner: &TypedExpr) -> bool {
        field.ty.is_reference()
            && Self::is_temporary(owner)
            && !self.program.type_holds_borrow(&owner.ty)
    }

    fn lends_temporary(argument: &TypedArgument) -> bool {
        argument.borrow == Borrow::Shared && Self::is_temporary(&argument.value)
    }

    fn control(&mut self, expr: &TypedExpr, control: &TypedControl, scope: TemporaryScope) {
        self.unsettled = true;
        match control {
            TypedControl::If(test, then, otherwise) => {
                self.operand(expr, test, TemporaryScope::Statement);
                self.operand(expr, then, scope);
                self.operand(expr, otherwise, scope);
            }
            TypedControl::Block(block) => {
                if let Some(tail) = block.tail() {
                    self.operand(expr, tail, scope);
                }
            }
            TypedControl::Match(matched) => {
                self.operand(expr, &matched.value, TemporaryScope::Statement);
                for tail in matched.arms.iter().filter_map(|arm| arm.body.tail()) {
                    self.operand(expr, tail, scope);
                }
            }
        }
    }

    fn operands(&mut self, expr: &TypedExpr, scope: TemporaryScope) {
        for operand in expr.children() {
            self.operand(expr, operand, scope);
        }
    }

    fn operand(&mut self, parent: &TypedExpr, operand: &TypedExpr, scope: TemporaryScope) {
        self.position.enter(parent, operand);
        match Self::borrowed_temporary(operand) {
            Some(temporary) if scope == TemporaryScope::Statement => {
                let site = self.position.clone();
                self.position.enter(operand, temporary);
                self.lift(temporary, site, LendSite::WrittenBorrow);
                self.position.leave();
            }
            _ => {
                self.visit(operand, scope);
                self.unsettled |= !Self::is_settled(operand);
            }
        }
        self.position.leave();
    }

    fn borrowed_temporary(operand: &TypedExpr) -> Option<&TypedExpr> {
        match &operand.kind {
            TypedExprKind::Borrow(_, value) if Self::is_temporary(value) => Some(value),
            _ => None,
        }
    }

    fn lift_child(&mut self, parent: &TypedExpr, temporary: &TypedExpr) {
        self.position.enter(parent, temporary);
        self.lift(temporary, self.position.clone(), LendSite::Argument);
        self.position.leave();
    }

    fn lift(&mut self, temporary: &TypedExpr, site: TreePosition, lend: LendSite) {
        let placement = if self.unsettled {
            Placement::InPlace
        } else {
            Placement::BeforeBinding
        };
        let scope = match placement {
            Placement::BeforeBinding => TemporaryScope::Binding,
            Placement::InPlace => TemporaryScope::Statement,
        };
        let outer = std::mem::replace(&mut self.unsettled, placement == Placement::InPlace);
        self.visit(temporary, scope);
        self.unsettled = outer;
        let position = match (placement, lend) {
            (Placement::InPlace, LendSite::WrittenBorrow) => site,
            _ => self.position.clone(),
        };
        self.lifts.push(Lift {
            position,
            placement,
            site: lend,
        });
    }

    fn is_temporary(value: &TypedExpr) -> bool {
        match &value.kind {
            TypedExprKind::Leaf(_) | TypedExprKind::Deref(_) => false,
            TypedExprKind::Borrow(_, inner) => Self::is_temporary(inner),
            _ => !value.is_place(),
        }
    }

    fn is_settled_callee(callee: &TypedExpr) -> bool {
        matches!(
            callee.kind,
            TypedExprKind::Leaf(TypedLeaf::Rust(_) | TypedLeaf::Global(_))
        )
    }

    fn is_settled(value: &TypedExpr) -> bool {
        match &value.kind {
            TypedExprKind::Leaf(TypedLeaf::Literal(_) | TypedLeaf::Global(_))
            | TypedExprKind::Fn(_) => true,
            _ => false,
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
            (type chooser (fn () (fn ((ref str)) (ref str))))
            (defn chooser () first-letter)
            (defn called-callee ()
              (local t ((chooser) (as-str (announced \"arg\"))))
              (len t))
            (type pick-ref (fn ((ref usize)) (ref usize)))
            (defn pick-ref (n) n)
            (defn dereferenced-call (n)
              (local t (prefix (deref (pick-ref (ref n))) (as-str (announced \"s\"))))
              (len t))
            (defn settled-literal ()
              (local t (prefix 1 (as-str (announced \"s\"))))
              (len t))
            (type applied (fn ((fn ((ref str)) (ref str)) (ref str)) (ref str)))
            (defn applied (f text) (f text))
            (defn settled-closure ()
              (local t (applied (fn (text) text) (as-str (announced \"c\"))))
              (len t))
            (defn settled-function ()
              (local t (applied first-letter (as-str (announced \"f\"))))
              (len t))
            (defn branch (flag name)
              (local t (if flag (first-letter (as-str (str name \"-n\"))) \"z\"))
              (len t))
            (defn block-tail (name)
              (local t (do (len name) (first-letter (as-str (str name \"-o\")))))
              (len t))
            (defn arm (flag name)
              (local t (match flag (true (first-letter (as-str (str name \"-p\")))) (false \"z\")))
              (len t))
            (defn page-title-letter ()
              (local t (first-letter (as-str (:title (->page :title (str \"q\"))))))
              (len t))
            (defn page-title ()
              (local t (:title (->page :title (str \"r\"))))
              (len t))
            (type tried-page (fn ((ref str)) (result usize io-error)))
            (defn tried-page (name)
              (local found (try (ok (ref (->page :title (str name))))))
              (ok (len (:title found))))
            (type card-text (fn ((ref card)) (ref str)))
            (defn card-text (held) (:text held))
            (defn hoisted-card (name)
              (local text (card-text (ref (->card :page (ref (->page :title (str name))) :text \"t\"))))
              (len text))
            (defn templated-callee (name)
              (local letters (vec (as-str (str name \"-v\")) \"w\"))
              (len letters))
            (defn extended-branch (flag name)
              (local held (if flag (ref (->page :title (str name))) (ref (->page :title (str \"z\")))))
              (len (:title held)))
            (defn extended-tail (name)
              (local held (do (len name) (ref (->page :title (str name)))))
              (len (:title held)))
            (defn extended-arm (flag name)
              (local held (match flag (true (ref (->page :title (str name)))) (false (ref (->page :title (str \"z\"))))))
              (len (:title held)))
            (defn nested-in-place ()
              (local t (prefix (len (announced \"a\")) (label-text (ref (->label :text (as-str (announced \"b\")))))))
              (len t))
            (type shown-ref (fn ((ref usize)) (ref usize)))
            (defn shown-ref (n) n)
            (defn lent-deref (n)
              (local shown (shown-ref (ref (deref (pick-ref (ref n))))))
              (deref shown))
            (type page-after (fn (usize (ref page)) (ref str)))
            (defn page-after (size page) (subs (:title page) 0 size))
            (defn written-after-code (name)
              (local t (page-after (len (announced \"w\")) (ref (->page :title (str name)))))
              (len t))
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
                .map(|temporary| match temporary.value() {
                    Some(value) => format!("{} {}", temporary.name(), self.text(value)),
                    None => format!("{} in place", temporary.name()),
                })
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
                TypedExprKind::Call(callee, arguments) if Self::assigns(callee) => {
                    vec![format!("assign {}", Self::source(&arguments[0].value))]
                }
                TypedExprKind::Borrow(_, inner)
                    if Self::named_leaves(inner)
                        .iter()
                        .any(|leaf| leaf.starts_with("assign")) =>
                {
                    vec![format!("borrowed {}", Self::named_leaves(inner).join(" "))]
                }
                _ => expr
                    .children()
                    .into_iter()
                    .flat_map(Self::named_leaves)
                    .collect(),
            }
        }

        fn assigns(callee: &TypedExpr) -> bool {
            matches!(&callee.kind,
                TypedExprKind::Leaf(TypedLeaf::Rust(template)) if template.template.starts_with("{{"))
        }

        fn source(expr: &TypedExpr) -> &'static str {
            &Self::SOURCE[expr.span.range()]
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
    fn keeps_a_borrowed_field_of_a_parameter() {
        assert!(LiftFixture::new().lifted("field-letter").is_empty());
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

    #[test]
    fn assigns_in_place_after_an_argument_that_runs_code() {
        let fixture = LiftFixture::new();
        assert_eq!(fixture.lifted("earlier-argument"), ["owner in place"]);
        assert_eq!(
            fixture.rewritten("earlier-argument"),
            "assign (announced \"second\")"
        );
    }

    #[test]
    fn assigns_in_place_after_a_local_the_owner_could_change() {
        assert_eq!(
            LiftFixture::new().rewritten("earlier-place"),
            "assign (announced \"third\")"
        );
    }

    #[test]
    fn assigns_in_place_after_a_borrowed_local() {
        assert_eq!(
            LiftFixture::new().rewritten("borrowed-local"),
            "assign (announced \"x\")"
        );
    }

    #[test]
    fn assigns_every_later_owner_in_place() {
        assert_eq!(
            LiftFixture::new().rewritten("nested-earlier"),
            "assign (announced \"c\") assign (announced \"e\")"
        );
    }

    #[test]
    fn assigns_in_place_after_two_arguments_that_run_code() {
        assert_eq!(LiftFixture::new().lifted("two-earlier"), ["owner in place"]);
    }

    #[test]
    fn assigns_in_place_after_an_extended_borrow_that_runs_code() {
        assert_eq!(
            LiftFixture::new().rewritten("extended-earlier"),
            "assign (announced \"z\")"
        );
    }

    #[test]
    fn assigns_in_place_after_a_callee_that_runs_code() {
        assert_eq!(
            LiftFixture::new().rewritten("called-callee"),
            "assign (announced \"arg\")"
        );
    }

    #[test]
    fn assigns_in_place_after_a_dereferenced_call() {
        assert_eq!(
            LiftFixture::new().rewritten("dereferenced-call"),
            "assign (announced \"s\")"
        );
    }

    #[test]
    fn binds_before_the_let_after_a_literal() {
        assert_eq!(
            LiftFixture::new().lifted("settled-literal"),
            ["owner (announced \"s\")"]
        );
    }

    #[test]
    fn binds_before_the_let_after_a_closure() {
        assert_eq!(
            LiftFixture::new().lifted("settled-closure"),
            ["owner (announced \"c\")"]
        );
    }

    #[test]
    fn binds_before_the_let_after_a_function_name() {
        assert_eq!(
            LiftFixture::new().lifted("settled-function"),
            ["owner (announced \"f\")"]
        );
    }

    #[test]
    fn assigns_in_place_in_a_branch() {
        assert_eq!(
            LiftFixture::new().rewritten("branch"),
            "assign (str name \"-n\")"
        );
    }

    #[test]
    fn assigns_in_place_in_a_block_tail() {
        assert_eq!(
            LiftFixture::new().rewritten("block-tail"),
            "assign (str name \"-o\")"
        );
    }

    #[test]
    fn assigns_in_place_in_a_match_arm() {
        assert_eq!(
            LiftFixture::new().rewritten("arm"),
            "assign (str name \"-p\")"
        );
    }

    #[test]
    fn lifts_a_temporary_whose_field_is_borrowed() {
        assert_eq!(
            LiftFixture::new().lifted("page-title-letter"),
            ["owner (->page :title (str \"q\"))"]
        );
    }

    #[test]
    fn keeps_a_borrowed_field_of_a_temporary_that_rust_extends() {
        assert!(LiftFixture::new().lifted("page-title").is_empty());
    }

    #[test]
    fn lifts_a_written_borrow_in_a_tried_value() {
        assert_eq!(
            LiftFixture::new().lifted("tried-page"),
            ["owner (->page :title (str name))"]
        );
    }

    #[test]
    fn keeps_a_borrow_that_a_lifted_owner_extends() {
        assert_eq!(
            LiftFixture::new().lifted("hoisted-card"),
            ["owner (->card :page (ref (->page :title (str name))) :text \"t\")"]
        );
    }

    #[test]
    fn binds_before_the_let_after_a_template_callee() {
        assert_eq!(
            LiftFixture::new().lifted("templated-callee"),
            ["owner (str name \"-v\")"]
        );
    }

    #[test]
    fn keeps_a_written_borrow_that_rust_extends_through_a_branch() {
        assert!(LiftFixture::new().lifted("extended-branch").is_empty());
    }

    #[test]
    fn keeps_a_written_borrow_that_rust_extends_through_a_block_tail() {
        assert!(LiftFixture::new().lifted("extended-tail").is_empty());
    }

    #[test]
    fn keeps_a_written_borrow_that_rust_extends_through_a_match_arm() {
        assert!(LiftFixture::new().lifted("extended-arm").is_empty());
    }

    #[test]
    fn assigns_a_temporary_inside_an_owner_assigned_in_place_in_place() {
        assert_eq!(
            LiftFixture::new().lifted("nested-in-place"),
            ["owner in place", "owner in place"]
        );
    }

    #[test]
    fn keeps_a_lent_dereference() {
        assert!(LiftFixture::new().lifted("lent-deref").is_empty());
    }

    #[test]
    fn assigns_a_written_borrow_in_place_of_the_borrow() {
        assert_eq!(
            LiftFixture::new().rewritten("written-after-code"),
            "assign (->page :title (str name))"
        );
    }
}
