use std::borrow::Cow;

use super::assigned_in_place::AssignedInPlace;
use super::bound_before::BoundBefore;
use super::emitted_value::EmittedValue;
use super::lending_expression::LendingExpression;
use super::lift_placement::LiftPlacement;
use super::lifted_value::LiftedValue;
use super::loan::Loan;
use super::short_form::ShortForm;
use super::temporary_scope::TemporaryScope;
use super::tree_position::TreePosition;
use crate::types::{TypedExpr, TypedProgram};

pub struct LentTemporaries<'p, 'n> {
    program: &'p TypedProgram,
    short_form: ShortForm<'p>,
    names: &'n mut dyn FnMut(&str) -> String,
}

impl<'p, 'n> LentTemporaries<'p, 'n> {
    const OWNER: &'static str = "owner";

    pub fn new(
        program: &'p TypedProgram,
        short_form: ShortForm<'p>,
        names: &'n mut dyn FnMut(&str) -> String,
    ) -> Self {
        Self {
            program,
            short_form,
            names,
        }
    }

    pub fn lift<'v, T: EmittedValue>(self, value: &'v T) -> LiftedValue<'v, T> {
        let lifts = TemporarySearch::new(self.program, self.short_form).lifts(value.expression());
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

struct TemporarySearch<'p, 'e> {
    program: &'p TypedProgram,
    short_form: ShortForm<'p>,
    path: Vec<&'e TypedExpr>,
    root: Option<usize>,
    lifts: Vec<Box<dyn LiftPlacement>>,
}

impl<'p, 'e> TemporarySearch<'p, 'e> {
    fn new(program: &'p TypedProgram, short_form: ShortForm<'p>) -> Self {
        Self {
            program,
            short_form,
            path: Vec::new(),
            root: Some(0),
            lifts: Vec::new(),
        }
    }

    fn lifts(mut self, value: &'e TypedExpr) -> Vec<Box<dyn LiftPlacement>> {
        if LendingExpression::new(value).may_lend() {
            self.path.push(value);
            self.visit(value, TemporaryScope::Binding);
        }
        self.lifts
    }

    fn visit(&mut self, expr: &'e TypedExpr, scope: TemporaryScope) {
        if self.program.type_holds_borrow(&expr.ty) {
            LendingExpression::new(expr).for_each_loan(scope, |loan| self.take(loan));
        }
    }

    fn take(&mut self, loan: Loan<'e>) {
        match loan {
            Loan::Temporary(temporary) => self.lift_argument(temporary),
            Loan::Operand(operand, scope) => self.operand(operand, scope),
        }
    }

    fn operand(&mut self, operand: &'e TypedExpr, scope: TemporaryScope) {
        self.path.push(operand);
        match LendingExpression::new(operand).borrowed_temporary() {
            Some(temporary) if scope == TemporaryScope::Statement => {
                let in_place = AssignedInPlace::of_written_borrow(TreePosition::along(&self.path));
                self.path.push(temporary);
                self.lift(in_place);
                self.path.pop();
            }
            _ => self.visit(operand, scope),
        }
        self.path.pop();
    }

    fn lift_argument(&mut self, temporary: &'e TypedExpr) {
        self.path.push(temporary);
        let in_place = AssignedInPlace::of_argument(TreePosition::along(&self.path));
        self.lift(in_place);
        self.path.pop();
    }

    fn lift(&mut self, in_place: AssignedInPlace) {
        let placement = self.placement(in_place);
        let temporary = self.path[self.path.len() - 1];
        let inner_scope = placement.inner_scope();
        let inner_root = (inner_scope == TemporaryScope::Binding).then_some(self.path.len() - 1);
        let outer_root = std::mem::replace(&mut self.root, inner_root);
        self.visit(temporary, inner_scope);
        self.root = outer_root;
        self.lifts.push(placement);
    }

    fn placement(&self, in_place: AssignedInPlace) -> Box<dyn LiftPlacement> {
        match self.root {
            Some(root) if self.short_form.admits(&self.path[root..]) => {
                Box::new(BoundBefore::new(TreePosition::along(&self.path)))
            }
            _ => Box::new(in_place),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LentTemporaries;
    use crate::emit::assigned_in_place::AssignedInPlace;
    use crate::emit::test_program::TestProgram;
    use crate::types::{TypedExpr, TypedExprKind, TypedLeaf};

    struct LiftFixture {
        program: TestProgram,
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
            (defstruct outer :inner label)
            (type make-outer (fn ((ref str)) outer))
            (defn make-outer (text) (->outer :inner (->label :text text)))
            (type owned-label-text (fn (label) (ref str)))
            (defn owned-label-text (tag) (:text tag))
            (defn moved-field (name)
              (local t (owned-label-text (:inner (make-outer name))))
              (len t))
            (defn scrutinee ()
              (local t (match (ref (announced \"m\")) (text (first-letter text))))
              (len t))
            (defn else-branch (flag name)
              (local t (if flag \"z\" (first-letter (as-str (str name \"-q\")))))
              (len t))
            (defn main ())";

        fn new() -> Self {
            Self {
                program: TestProgram::new(Self::SOURCE),
            }
        }

        fn lifted(&self, function: &str) -> Vec<String> {
            let mut names = |base: &str| base.to_string();
            let plan = self.program.plan();
            let short_form = TestProgram::short_form(&plan);
            let lifted = LentTemporaries::new(self.program.program(), short_form, &mut names)
                .lift(self.last_let_value(function));
            let mut assigned = Vec::new();
            Self::collect_owned(lifted.value(), &mut assigned);
            let mut assigned = assigned.into_iter();
            lifted
                .temporaries()
                .iter()
                .map(|temporary| match temporary.value() {
                    Some(value) => format!("short {}", self.text(value)),
                    None => {
                        let value = assigned.next().expect("each declared owner is assigned");
                        format!("in place {}", self.text(value))
                    }
                })
                .collect()
        }

        fn collect_owned<'e>(expr: &'e TypedExpr, owned: &mut Vec<&'e TypedExpr>) {
            if let TypedExprKind::Call(callee, arguments) = &expr.kind
                && Self::assigns(callee)
            {
                Self::collect_owned(&arguments[0].value, owned);
                owned.push(&arguments[0].value);
                return;
            }
            for child in expr.children() {
                Self::collect_owned(child, owned);
            }
        }

        fn rewritten(&self, function: &str) -> String {
            let mut names = |base: &str| base.to_string();
            let plan = self.program.plan();
            let short_form = TestProgram::short_form(&plan);
            let lifted = LentTemporaries::new(self.program.program(), short_form, &mut names)
                .lift(self.last_let_value(function));
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
            AssignedInPlace::is_assignment(callee)
        }

        fn source(expr: &TypedExpr) -> &'static str {
            &Self::SOURCE[expr.span.range()]
        }

        fn last_let_value(&self, function: &str) -> &TypedExpr {
            self.program.last_let_value(function)
        }

        fn text(&self, expr: &TypedExpr) -> &'static str {
            self.program.text(expr)
        }
    }

    #[test]
    fn lifts_a_string_lent_through_as_str() {
        assert_eq!(
            LiftFixture::new().lifted("as-str-argument"),
            ["short (str name \"-a\")",]
        );
    }

    #[test]
    fn names_a_short_form_owner_where_the_value_was() {
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
            ["short (->page :title (str name \"-e\"))",]
        );
    }

    #[test]
    fn lifts_a_struct_the_call_borrows() {
        assert_eq!(
            LiftFixture::new().lifted("borrowed-argument"),
            ["short (->page :title (str name \"-f\"))",]
        );
    }

    #[test]
    fn lifts_a_temporary_lent_to_a_struct_field() {
        assert_eq!(
            LiftFixture::new().lifted("struct-field"),
            ["short (str name \"-g\")",]
        );
    }

    #[test]
    fn lifts_a_temporary_lent_to_a_variant_payload() {
        assert_eq!(
            LiftFixture::new().lifted("variant-payload"),
            ["short (str name \"-h\")",]
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
            ["short (str name \"-i\")",]
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
                "short (->page :title (str name))",
                "in place (announced \"y\")",
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
                "short (str name \"-l\")",
                "short (->label :text (as-str (str name \"-l\")))",
            ]
        );
    }

    #[test]
    fn lifts_a_temporary_lent_inside_a_written_borrow_rust_extends() {
        assert_eq!(
            LiftFixture::new().lifted("written-borrow-of-struct"),
            ["short (str name \"-m\")",]
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
                "short (announced \"p\")",
                "short (->label :text (cut (as-str (announced \"p\")) (len (announced \"q\"))))",
                "in place (announced \"r\")",
            ]
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
            ["short (->page :title (str \"q\"))",]
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
            ["short (->page :title (str name))",]
        );
    }

    #[test]
    fn keeps_a_borrow_that_a_short_form_owner_extends() {
        assert_eq!(
            LiftFixture::new().lifted("hoisted-card"),
            ["short (->card :page (ref (->page :title (str name))) :text \"t\")",]
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
    fn assigns_a_temporary_inside_an_assigned_owner_first() {
        assert_eq!(
            LiftFixture::new().lifted("nested-in-place"),
            [
                "in place (announced \"b\")",
                "in place (->label :text (as-str (announced \"b\")))",
            ]
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

    #[test]
    fn keeps_a_field_moved_out_of_a_temporary() {
        assert!(LiftFixture::new().lifted("moved-field").is_empty());
    }

    #[test]
    fn lifts_a_written_borrow_that_a_match_checks() {
        assert_eq!(
            LiftFixture::new().lifted("scrutinee"),
            ["in place (announced \"m\")",]
        );
    }

    #[test]
    fn lifts_a_temporary_lent_only_in_an_else_branch() {
        assert_eq!(
            LiftFixture::new().lifted("else-branch"),
            ["in place (str name \"-q\")",]
        );
    }
}
