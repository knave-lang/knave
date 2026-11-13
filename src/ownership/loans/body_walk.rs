use super::access::Move;
use super::body_builder::BodyBuilder;
use super::body_facts::BodyFacts;
use super::call_site::CallSite;
use super::call_walk::CallWalk;
use super::closure::Closure;
use super::expression_walk::ExpressionWalk;
use super::holdings::Holdings;
use super::lending::Lending;
use super::loan_kind::LoanKind;
use super::loan_place::LoanPlace;
use super::loan_rules::LoanRules;
use super::loop_walk::LoopWalk;
use super::place_access::PlaceAccess;
use super::point_id::PointId;
use super::walked::Walked;
use crate::core::ReferenceKind;
use crate::types::{
    TypedBlock, TypedBodyItem, TypedControl, TypedExpr, TypedExprKind, TypedLambda, TypedLeaf,
    TypedMatch, TypedStructExpr,
};

pub struct BodyWalk<'w> {
    rules: &'w LoanRules<'w>,
    builder: BodyBuilder<'w>,
    closures: Vec<Closure<'w>>,
}

impl<'w> BodyWalk<'w> {
    pub fn new(rules: &'w LoanRules<'w>) -> Self {
        Self {
            rules,
            builder: BodyBuilder::default(),
            closures: Vec::new(),
        }
    }

    pub fn walk(mut self, lambda: &'w TypedLambda) -> BodyFacts<'w> {
        for parameter in &lambda.params {
            self.builder
                .define(PointId::ENTRY, parameter, &Holdings::default());
        }
        let tail = self.block(&lambda.body, PointId::ENTRY);
        self.builder.into_facts(tail.held, self.closures)
    }

    fn expr_holding(&mut self, expr: &'w TypedExpr, at: PointId) -> Walked {
        match &expr.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => {
                let point = self.builder.read(at, *binding);
                Walked::holding(point, Holdings::of_value(*binding))
            }
            TypedExprKind::Fn(lambda) => {
                self.closures.push(Closure::new(lambda, expr.span));
                Walked::at(at)
            }
            TypedExprKind::Call(callee, arguments) => {
                self.call(CallSite::new(callee, arguments), at)
            }
            TypedExprKind::Recur(arguments) => LoopWalk::new(self).recur(arguments, at),
            TypedExprKind::Borrow(kind, inner) => self.borrowed(*kind, inner, at),
            TypedExprKind::Struct(TypedStructExpr::Field(owner, _)) => {
                self.field_read(expr, owner, at)
            }
            TypedExprKind::Control(control) => self.control(control, at),
            _ => self.sequence(expr.children().into_iter(), at),
        }
    }

    fn call(&mut self, site: CallSite<'w>, at: PointId) -> Walked {
        match site.repeated_lambda() {
            Some(lambda) => LoopWalk::new(self).repeated(lambda, site.arguments(), at),
            None => CallWalk::new(self, site).walk(at),
        }
    }

    fn borrowed(&mut self, kind: ReferenceKind, inner: &'w TypedExpr, at: PointId) -> Walked {
        let lending = LoanPlace::of(inner)
            .map(|place| Lending::new(place, <dyn LoanKind>::of(kind), inner.span));
        self.lent_from(inner, lending, at)
    }

    fn field_read(&mut self, expr: &'w TypedExpr, owner: &'w TypedExpr, at: PointId) -> Walked {
        let shared = <dyn LoanKind>::of(ReferenceKind::Shared);
        let lending = LoanPlace::of(expr)
            .filter(|_| expr.ty.is_reference())
            .map(|place| Lending::new(place, shared, expr.span));
        self.lent_from(owner, lending, at)
    }

    fn sequence(&mut self, values: impl Iterator<Item = &'w TypedExpr>, at: PointId) -> Walked {
        values.fold(Walked::at(at), |walked, value| {
            let next = self.expr(value, walked.end);
            Walked::holding(next.end, walked.held.merged(next.held))
        })
    }

    fn control(&mut self, control: &'w TypedControl, at: PointId) -> Walked {
        match control {
            TypedControl::Block(block) => self.block(block, at),
            TypedControl::If(condition, then, otherwise) => {
                let decided = self.expr(condition, at);
                let branches = [
                    self.expr(then, decided.end),
                    self.expr(otherwise, decided.end),
                ];
                self.joined(branches)
            }
            TypedControl::Match(matched) => self.matched(matched, at),
        }
    }

    fn matched(&mut self, matched: &'w TypedMatch, at: PointId) -> Walked {
        let value = self.expr(&matched.value, at);
        let arms: Vec<Walked> = matched
            .arms
            .iter()
            .map(|arm| {
                let bound = self.builder.join(&[value.end]);
                for binder in arm.pattern.binders() {
                    self.builder.define(bound, binder, &value.held);
                }
                self.block(&arm.body, bound)
            })
            .collect();
        self.joined(arms)
    }

    fn joined(&mut self, branches: impl IntoIterator<Item = Walked>) -> Walked {
        let (ends, held): (Vec<PointId>, Vec<Holdings>) = branches
            .into_iter()
            .map(|branch| (branch.end, branch.held))
            .unzip();
        let joined = self.builder.join(&ends);
        Walked::holding(
            joined,
            held.into_iter().fold(Holdings::default(), Holdings::merged),
        )
    }

    fn lent_from(&mut self, owner: &'w TypedExpr, lending: Option<Lending>, at: PointId) -> Walked {
        let read = self.expr(owner, at);
        match lending {
            Some(lending) => {
                let point = self.builder.join(&[read.end]);
                let lent = self.builder.lender(point).lend(lending);
                Walked::holding(point, read.held.merged(lent))
            }
            None => read,
        }
    }
}

impl<'w> ExpressionWalk<'w> for BodyWalk<'w> {
    fn expr(&mut self, expr: &'w TypedExpr, at: PointId) -> Walked {
        let walked = self.expr_holding(expr, at);
        if self.rules.holds_a_borrow(&expr.ty) {
            walked
        } else {
            Walked::at(walked.end)
        }
    }

    fn block(&mut self, block: &'w TypedBlock, at: PointId) -> Walked {
        block
            .items
            .iter()
            .fold(Walked::at(at), |walked, item| match item {
                TypedBodyItem::Let(binder, value) => {
                    let bound = self.expr(value, walked.end);
                    let defined = self.builder.join(&[bound.end]);
                    if let Some(place) = LoanPlace::of(value).filter(|_| self.rules.moves(value)) {
                        let moved = PlaceAccess::new(place, &Move, value.span);
                        self.builder.lender(defined).access(moved);
                    }
                    self.builder.define(defined, binder, &bound.held);
                    Walked::at(defined)
                }
                TypedBodyItem::Expr(value) => self.expr(value, walked.end),
            })
    }

    fn rules(&self) -> &'w LoanRules<'w> {
        self.rules
    }

    fn builder(&mut self) -> &mut BodyBuilder<'w> {
        &mut self.builder
    }
}

#[cfg(test)]
mod tests {
    use crate::ownership::loans::loan_source::LoanSource;

    struct WalkFixture;

    impl WalkFixture {
        fn change_of_v_in(body: &str) -> String {
            let text = format!("(defn main ()\n  (local v (vec 1 2))\n{body})");
            LoanSource::new(&text).line_of("change `v`")
        }
    }

    #[test]
    fn a_copy_out_of_a_borrow_passes_no_loan_to_the_call() {
        let line = WalkFixture::change_of_v_in("  (push! v (at v 0))");
        assert!(line.ends_with("no live loan"), "{line}");
    }

    #[test]
    fn a_borrow_argument_is_live_at_the_call_that_changes_its_place() {
        let source = LoanSource::new(
            "(defstruct car :make string :year i64)\n\n(defn relabel! (c m) (set! (:make c) (str m)))\n\n(defn main ()\n  (local c (->car :make (str \"a\") :year 1))\n  (relabel! c (:make c)))",
        );
        assert!(
            source
                .line_of("change `c`:")
                .ends_with("`c.make` lent at 7:15, passed to the call")
        );
        assert!(source.line_of("change `c.make`").ends_with("no live loan"));
    }

    #[test]
    fn a_loan_held_before_a_loop_is_live_at_a_change_inside_it() {
        let line = WalkFixture::change_of_v_in(
            "  (local e (at v 0))\n  (loop (i 0) (if (< i 2) (do (push! v 1) (recur (+ i 1))) (do)))\n  (println e)",
        );
        assert!(line.ends_with("held by `e`"), "{line}");
    }

    #[test]
    fn a_loan_used_only_before_the_change_is_not_live_there() {
        let line =
            WalkFixture::change_of_v_in("  (local r (at v 0))\n  (println r)\n  (push! v 4)");
        assert!(line.ends_with("no live loan"), "{line}");
    }

    #[test]
    fn a_local_that_takes_a_borrowed_place_moves_it() {
        let source = LoanSource::new(
            "(defn main ()\n  (local v (vec 1 2))\n  (local held (at v 0))\n  (local moved v)\n  (println held (len moved)))",
        );
        assert_eq!(
            source.line_of("move `v`"),
            "4:16 move `v`: `v` lent at 3:19, held by `held`"
        );
    }

    #[test]
    fn a_borrow_on_one_branch_reaches_the_result_from_both() {
        let source = LoanSource::new(
            "(defn pick! (v)\n  (local r (at v 0))\n  (if (> r 0) r (do (push! v 1) (at v 0))))\n\n(defn main ()\n  (local v (vec 1))\n  (println (pick! v)))",
        );
        assert!(
            source
                .line_of("3:28 change")
                .ends_with("lent at 2:16, returned")
        );
    }

    #[test]
    fn a_value_copied_out_of_a_borrow_holds_no_loan() {
        let line = WalkFixture::change_of_v_in(
            "  (local r (at v 0))\n  (local n (deref r))\n  (push! v 4)\n  (println n)",
        );
        assert!(line.ends_with("no live loan"), "{line}");
    }

    #[test]
    fn an_index_read_through_a_borrow_still_lends_the_vector() {
        let line = WalkFixture::change_of_v_in(
            "  (local w (vec 0))\n  (local x (at w 0))\n  (local r (at v x))\n  (push! v 4)\n  (println r)",
        );
        assert!(line.ends_with("held by `r`"), "{line}");
    }

    #[test]
    fn a_map_lookup_holds_the_map_and_not_the_key() {
        let source = LoanSource::new(
            "(defn main ()\n  (local m (hash-map (str \"k\") 1))\n  (local k (str \"k\"))\n  (local r (get m k))\n  (set! k (str \"j\"))\n  (set! m (hash-map (str \"j\") 2))\n  (println (unwrap r) k))",
        );
        assert!(!source.line_of("5:9 change").contains("`k` lent"));
        assert!(
            source
                .line_of("6:9 change")
                .contains("`m` lent at 4:17, held by `r`")
        );
    }

    #[test]
    fn a_prelude_call_with_two_borrowed_parameters_holds_both_loans() {
        let line = WalkFixture::change_of_v_in(
            "  (local d 0)\n  (local e (unwrap-or (first v) (ref d)))\n  (push! v 4)\n  (println e)",
        );
        assert!(line.contains("`v` lent at 4:30, held by `e`"), "{line}");
    }

    #[test]
    fn a_borrow_taken_through_a_borrowed_local_keeps_the_loan_it_holds() {
        let line = WalkFixture::change_of_v_in(
            "  (local r (ref v))\n  (local e (at r 0))\n  (push! v 4)\n  (println e)",
        );
        assert!(line.contains("`v` lent at 3:17, held by `e`"), "{line}");
    }

    #[test]
    fn a_helper_with_a_written_borrowed_parameter_lends_nothing_yet() {
        let source = LoanSource::new(
            "(defn head ((v (ref (vec i64)))) (at v 0))\n\n(defn main ()\n  (local v (vec 1 2))\n  (local r (head (ref v)))\n  (push! v 4)\n  (println r))",
        );
        assert!(source.line_of("6:10 change").ends_with("no live loan"));
    }

    #[test]
    fn a_borrow_read_only_through_a_copy_stays_live_until_that_read() {
        let line = WalkFixture::change_of_v_in(
            "  (local r (at v 0))\n  (push! v 4)\n  (println (+ (deref r) 1))",
        );
        assert!(line.ends_with("`v` lent at 3:16, held by `r`"), "{line}");
    }

    #[test]
    fn a_written_ref_lends_its_place() {
        let line =
            WalkFixture::change_of_v_in("  (local r (ref v))\n  (push! v 4)\n  (println (len r))");
        assert!(line.ends_with("`v` lent at 3:17, held by `r`"), "{line}");
    }

    #[test]
    fn a_struct_holds_the_loans_of_every_field_and_an_update_and_a_variant_hold_theirs() {
        let source = LoanSource::new(
            "(defstruct both :a (ref i64) :b (ref i64))\n\n(defn main ()\n  (local v (vec 1 2))\n  (local w (vec 3))\n  (local p (->both :a (at v 0) :b (at w 0)))\n  (local q (assoc p :b (at v 1)))\n  (local o (some (at w 0)))\n  (push! v 4)\n  (println (:a p) (:b q) (unwrap-or o (at v 0))))",
        );
        let line = source.line_of("change `v`");
        assert!(line.contains("`v` lent at 6:27, held by `p`"), "{line}");
        assert!(line.contains("`v` lent at 7:28, held by `q`"), "{line}");
        assert!(line.contains("`w` lent at 8:22, held by `o`"), "{line}");
    }

    #[test]
    fn a_match_binder_holds_the_loans_of_the_matched_value() {
        let line = WalkFixture::change_of_v_in(
            "  (match (first v)\n    ((some x) (do (push! v 1) (println x)))\n    (none (println 0)))",
        );
        assert!(line.ends_with("`v` lent at 3:17, held by `x`"), "{line}");
    }

    #[test]
    fn a_field_read_through_a_value_holding_a_borrow_keeps_that_borrow() {
        let source = LoanSource::new(
            "(defstruct tag :label (ref string))\n\n(defn main ()\n  (local v (vec (str \"a\")))\n  (local t (->tag :label (at v 0)))\n  (local l (:label t))\n  (push! v (str \"b\"))\n  (println l))",
        );
        assert!(
            source
                .line_of("change `v`")
                .contains("`v` lent at 5:30, held by `l`")
        );
    }

    #[test]
    fn a_borrowed_place_moved_into_a_call_is_a_move() {
        let source = LoanSource::new(
            "(defn keep (xs) xs)\n\n(defn main ()\n  (local v (vec 1 2))\n  (local r (at v 0))\n  (local k (keep v))\n  (println r (len k)))",
        );
        assert_eq!(
            source.line_of("move `v`"),
            "6:18 move `v`: `v` lent at 5:16, held by `r`"
        );
    }

    #[test]
    fn a_local_moved_into_another_is_not_live_at_its_own_move() {
        let source = LoanSource::new(
            "(defstruct tag :label (ref string) :note string)\n\n(defn main ()\n  (local v (vec (str \"a\")))\n  (local t (->tag :label (at v 0) :note (str \"n\")))\n  (local u t)\n  (println (:label u)))",
        );
        assert!(source.line_of("move `t`").ends_with("no live loan"));
    }

    #[test]
    fn a_helper_result_holds_no_loan() {
        let source = LoanSource::new(
            "(defn first-of (v) (at v 0))\n\n(defn main ()\n  (local v (vec 1 2))\n  (local r (first-of v))\n  (push! v 4)\n  (println r))",
        );
        assert!(source.line_of("6:10 change").ends_with("no live loan"));
    }
}
