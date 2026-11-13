use super::argument_passing::ArgumentPassing;
use super::body_graph::BodyGraph;
use super::call_site::CallSite;
use super::holdings::Holdings;
use super::lending::Lending;
use super::loan_context::LoanContext;
use super::loan_place::LoanPlace;
use super::place_access::{AccessKind, PlaceAccess};
use super::point_id::PointId;
use super::value_regions::ValueRegions;
use super::walked_body::WalkedBody;
use crate::core::ReferenceKind;
use crate::source::Span;
use crate::types::{
    TypedArgument, TypedBinder, TypedBlock, TypedBodyItem, TypedControl, TypedExpr, TypedExprKind,
    TypedLambda, TypedLeaf, TypedMatch, TypedStructExpr,
};

pub struct BodyWalk<'w> {
    context: &'w LoanContext<'w>,
    graph: BodyGraph,
    regions: ValueRegions,
    loops: Vec<LoopHead<'w>>,
    closures: Vec<&'w TypedExpr>,
}

struct LoopHead<'w> {
    head: PointId,
    parameters: &'w [TypedBinder],
}

struct Walked {
    end: PointId,
    held: Holdings,
}

impl<'w> BodyWalk<'w> {
    pub fn new(context: &'w LoanContext<'w>) -> Self {
        Self {
            context,
            graph: BodyGraph::new(),
            regions: ValueRegions::default(),
            loops: Vec::new(),
            closures: Vec::new(),
        }
    }

    pub fn walk(mut self, lambda: &'w TypedLambda) -> WalkedBody<'w> {
        for parameter in &lambda.params {
            self.graph
                .point_mut(PointId::ENTRY)
                .define(parameter.binding);
        }
        let tail = self.block(&lambda.body, PointId::ENTRY);
        WalkedBody::new(self.graph, self.regions)
            .returning(tail.held)
            .with_closures(self.closures)
    }

    fn expr(&mut self, expr: &'w TypedExpr, at: PointId) -> Walked {
        let walked = self.expr_holding(expr, at);
        if expr.ty.holds_reference() {
            walked
        } else {
            Walked::at(walked.end)
        }
    }

    fn expr_holding(&mut self, expr: &'w TypedExpr, at: PointId) -> Walked {
        match &expr.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => {
                let point = self.graph.join(&[at]);
                self.graph.point_mut(point).using(*binding);
                Walked::holding(point, Holdings::of_value(*binding))
            }
            TypedExprKind::Leaf(_) => Walked::at(at),
            TypedExprKind::Fn(_) => {
                self.closures.push(expr);
                Walked::at(at)
            }
            TypedExprKind::Call(callee, arguments) => {
                self.call(CallSite::new(expr, callee, arguments), at)
            }
            TypedExprKind::Recur(arguments) => self.recur(arguments, at),
            TypedExprKind::Try { value, .. }
            | TypedExprKind::Infer(value)
            | TypedExprKind::Deref(value)
            | TypedExprKind::TraitObject(value, _) => self.expr(value, at),
            TypedExprKind::Borrow(kind, inner) => {
                let lending =
                    LoanPlace::of(inner).map(|place| Lending::new(place, *kind, inner.span));
                self.lent_from(inner, lending, at)
            }
            TypedExprKind::Control(control) => self.control(control, at),
            TypedExprKind::Struct(TypedStructExpr::Field(owner, _)) => {
                let lending = LoanPlace::of(expr)
                    .filter(|_| expr.ty.is_reference())
                    .map(|place| Lending::new(place, ReferenceKind::Shared, expr.span));
                self.lent_from(owner, lending, at)
            }
            TypedExprKind::Struct(TypedStructExpr::New(_, fields)) => {
                self.sequence(fields.iter().map(|(_, value)| value), at)
            }
            TypedExprKind::Struct(TypedStructExpr::Update(_, base, fields)) => self.sequence(
                std::iter::once(base.as_ref()).chain(fields.iter().map(|(_, value)| value)),
                at,
            ),
            TypedExprKind::Variant(_, values, _) => self.sequence(values.iter(), at),
            TypedExprKind::Assoc(map, keys, value) => self.sequence(
                std::iter::once(map.as_ref())
                    .chain(keys)
                    .chain(std::iter::once(value.as_ref())),
                at,
            ),
        }
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
                let bound = self.graph.join(&[value.end]);
                for binder in arm.pattern.binders() {
                    self.define(bound, binder, &value.held);
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
        let joined = self.graph.join(&ends);
        Walked::holding(
            joined,
            held.into_iter().fold(Holdings::default(), Holdings::merged),
        )
    }

    fn block(&mut self, block: &'w TypedBlock, at: PointId) -> Walked {
        block
            .items
            .iter()
            .fold(Walked::at(at), |walked, item| match item {
                TypedBodyItem::Let(binder, value) => {
                    let bound = self.expr(value, walked.end);
                    let defined = self.graph.join(&[bound.end]);
                    if let Some(place) = LoanPlace::of(value).filter(|_| self.context.moves(value))
                    {
                        self.moved(place, value.span, defined);
                    }
                    self.define(defined, binder, &bound.held);
                    Walked::at(defined)
                }
                TypedBodyItem::Expr(value) => self.expr(value, walked.end),
            })
    }

    fn define(&mut self, point: PointId, binder: &TypedBinder, held: &Holdings) {
        self.graph.point_mut(point).define(binder.binding);
        self.regions.flow_into(binder.binding, held);
    }

    fn lent_from(&mut self, owner: &'w TypedExpr, lending: Option<Lending>, at: PointId) -> Walked {
        let read = self.expr(owner, at);
        match lending {
            Some(lending) => {
                let point = self.graph.join(&[read.end]);
                let held = read.held.merged(self.lend(lending, point));
                Walked::holding(point, held)
            }
            None => read,
        }
    }

    fn call(&mut self, site: CallSite<'w>, at: PointId) -> Walked {
        if let Some(lambda) = site.repeated_lambda() {
            return self.repeated(lambda, site.arguments(), at);
        }
        let called = self.expr(site.callee(), at);
        let passings: Vec<ArgumentPassing> = site
            .arguments()
            .iter()
            .enumerate()
            .map(|(position, value)| {
                ArgumentPassing::of(value, site.parameter(position), self.context)
            })
            .collect();
        let evaluated = self.arguments(site.arguments(), &passings, called.end);
        let point = self.graph.join(&[evaluated.end]);
        let passed: Vec<Holdings> = site
            .arguments()
            .iter()
            .zip(passings)
            .zip(evaluated.held)
            .map(|((value, passing), held)| self.pass(point, Passed { value, passing }, held))
            .collect();
        Walked::holding(point, self.context.result_holdings(&site, passed))
    }

    fn arguments(
        &mut self,
        arguments: &'w [TypedArgument],
        passings: &[ArgumentPassing],
        at: PointId,
    ) -> EvaluatedArguments {
        let mut evaluated = EvaluatedArguments {
            end: at,
            held: Vec::new(),
        };
        for (position, argument) in arguments.iter().enumerate() {
            let walked = match passings.get(position) {
                Some(passing) if passing.takes_the_place() => {
                    self.place_root(&argument.value, evaluated.end)
                }
                _ => self.expr(&argument.value, evaluated.end),
            };
            evaluated.end = walked.end;
            evaluated.held.push(walked.held);
        }
        evaluated
    }

    fn place_root(&mut self, place: &'w TypedExpr, at: PointId) -> Walked {
        match &place.kind {
            TypedExprKind::Struct(TypedStructExpr::Field(owner, _)) => self.place_root(owner, at),
            _ => self.expr(place, at),
        }
    }

    fn pass(&mut self, point: PointId, argument: Passed<'w>, held: Holdings) -> Holdings {
        let span = argument.value.value.span;
        let passed = match (LoanPlace::of(&argument.value.value), argument.passing) {
            (_, ArgumentPassing::Copies) => Holdings::default(),
            (Some(place), ArgumentPassing::Lends(kind)) => {
                held.merged(self.lend(Lending::new(place, kind, span), point))
            }
            (Some(place), ArgumentPassing::Moves) => {
                self.moved(place, span, point);
                held
            }
            _ => held,
        };
        let received = self.graph.point_mut(point);
        received.receive(passed.loans());
        for value in passed.values() {
            received.using(value);
        }
        passed
    }

    fn moved(&mut self, place: LoanPlace, span: Span, point: PointId) {
        let moved = PlaceAccess::new(place, AccessKind::Move, span);
        self.graph.point_mut(point).access(moved);
    }

    fn lend(&mut self, lending: Lending, point: PointId) -> Holdings {
        Holdings::of_loan(self.graph.lend(lending, point))
    }

    fn repeated(
        &mut self,
        lambda: &'w TypedLambda,
        arguments: &'w [TypedArgument],
        at: PointId,
    ) -> Walked {
        let evaluated = self.arguments(arguments, &[], at);
        let started = self.graph.join(&[evaluated.end]);
        self.bind_parameters(started, &lambda.params, evaluated.held);
        let head = self.graph.join(&[started]);
        self.loops.push(LoopHead {
            head,
            parameters: &lambda.params,
        });
        let body = self.block(&lambda.body, head);
        self.loops.pop();
        body
    }

    fn recur(&mut self, arguments: &'w [TypedArgument], at: PointId) -> Walked {
        let evaluated = self.arguments(arguments, &[], at);
        let again = self.graph.join(&[evaluated.end]);
        if let Some(LoopHead { head, parameters }) = self.loops.last() {
            let (head, parameters) = (*head, *parameters);
            self.bind_parameters(again, parameters, evaluated.held);
            self.graph.link(again, head);
        }
        Walked::at(self.graph.join(&[]))
    }

    fn bind_parameters(&mut self, point: PointId, parameters: &[TypedBinder], held: Vec<Holdings>) {
        for (parameter, held) in parameters.iter().zip(held) {
            self.define(point, parameter, &held);
        }
    }
}

struct Passed<'w> {
    value: &'w TypedArgument,
    passing: ArgumentPassing,
}

struct EvaluatedArguments {
    end: PointId,
    held: Vec<Holdings>,
}

impl Walked {
    fn at(end: PointId) -> Self {
        Self {
            end,
            held: Holdings::default(),
        }
    }

    fn holding(end: PointId, held: Holdings) -> Self {
        Self { end, held }
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
    fn a_prelude_call_with_two_borrowed_parameters_lends_nothing() {
        let line = WalkFixture::change_of_v_in(
            "  (local d 0)\n  (local e (unwrap-or (first v) (ref d)))\n  (push! v 4)\n  (println e)",
        );
        assert!(line.ends_with("no live loan"), "{line}");
    }

    #[test]
    fn a_helper_result_holds_no_loan() {
        let source = LoanSource::new(
            "(defn first-of (v) (at v 0))\n\n(defn main ()\n  (local v (vec 1 2))\n  (local r (first-of v))\n  (push! v 4)\n  (println r))",
        );
        assert!(source.line_of("6:10 change").ends_with("no live loan"));
    }
}
