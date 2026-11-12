use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::BindingId;
use crate::types::{
    Pipelines, ReturnedValues, Type, TypedArgument, TypedBinder, TypedBlock, TypedBodyItem,
    TypedControl, TypedExpr, TypedExprKind, TypedLambda, TypedLeaf, TypedMatch, TypedProgram,
    TypedStructExpr,
};

pub struct HeldBorrows<'a> {
    program: &'a TypedProgram,
}

struct FunctionScope<'l> {
    borrows: &'l HeldBorrows<'l>,
    locals: FxHashMap<BindingId, FxHashSet<BindingId>>,
    rebound: Vec<&'l [TypedBinder]>,
}

impl<'a> HeldBorrows<'a> {
    pub fn new(program: &'a TypedProgram) -> Self {
        Self { program }
    }

    pub fn kept_parameters(&self, lambda: &TypedLambda) -> FxHashSet<BindingId> {
        let scope = FunctionScope::of(self, lambda);
        let mut held = Vec::new();
        for closure in ReturnedValues::new(self.program).closures(lambda) {
            Self::collect_held(closure, &mut held);
        }
        let parameters: FxHashSet<BindingId> =
            lambda.params.iter().map(|param| param.binding).collect();
        self.sources(held, &scope)
            .intersection(&parameters)
            .copied()
            .collect()
    }

    fn collect_held(expr: &TypedExpr, held: &mut Vec<BindingId>) {
        match &expr.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => held.push(*binding),
            _ => expr
                .children()
                .into_iter()
                .for_each(|child| Self::collect_held(child, held)),
        }
    }

    fn sources(&self, mut pending: Vec<BindingId>, scope: &FunctionScope) -> FxHashSet<BindingId> {
        let mut reached = FxHashSet::default();
        while let Some(binding) = pending.pop() {
            let Some(sources) = scope.locals.get(&binding) else {
                continue;
            };
            for &source in sources {
                if reached.insert(source) {
                    pending.push(source);
                }
            }
        }
        reached
    }

    fn may_hold_borrow(&self, ty: &Type) -> bool {
        matches!(ty, Type::Function(..))
            || self.program.type_holds_borrow(ty)
            || Pipelines::new(self.program.declarations.types.rust_types()).is_pipeline(ty)
    }
}

impl<'l> FunctionScope<'l> {
    fn of(borrows: &'l HeldBorrows<'l>, lambda: &'l TypedLambda) -> Self {
        let mut scope = Self {
            borrows,
            locals: FxHashMap::default(),
            rebound: Vec::new(),
        };
        scope.block(&lambda.body);
        scope
    }

    fn block(&mut self, block: &'l TypedBlock) {
        for item in &block.items {
            match item {
                TypedBodyItem::Let(binder, value) => {
                    self.bind(binder.binding, value);
                    self.expr(value);
                }
                TypedBodyItem::Expr(expr) => self.expr(expr),
            }
        }
    }

    fn expr(&mut self, expr: &'l TypedExpr) {
        match &expr.kind {
            TypedExprKind::Call(callee, arguments)
                if let TypedExprKind::Fn(body) = &callee.kind =>
            {
                self.enter_loop(body, arguments)
            }
            TypedExprKind::Call(callee, arguments) => self.call(callee, arguments),
            TypedExprKind::Recur(arguments) => self.recur(arguments),
            TypedExprKind::Fn(lambda) => self.closure(lambda, expr),
            TypedExprKind::Control(TypedControl::Block(block)) => self.block(block),
            TypedExprKind::Control(TypedControl::Match(matched)) => self.matched(matched),
            _ => self.children(expr),
        }
    }

    fn closure(&mut self, lambda: &'l TypedLambda, expr: &'l TypedExpr) {
        self.rebound.push(&lambda.params);
        self.children(expr);
        self.rebound.pop();
    }

    fn matched(&mut self, matched: &'l TypedMatch) {
        self.expr(&matched.value);
        for arm in &matched.arms {
            for binder in arm.pattern.binders() {
                self.bind(binder.binding, &matched.value);
            }
            self.block(&arm.body);
        }
    }

    fn children(&mut self, expr: &'l TypedExpr) {
        expr.children()
            .into_iter()
            .for_each(|child| self.expr(child));
    }

    fn enter_loop(&mut self, body: &'l TypedLambda, arguments: &'l [TypedArgument]) {
        for (param, argument) in body.params.iter().zip(arguments) {
            self.bind(param.binding, &argument.value);
        }
        self.rebound.push(&body.params);
        self.block(&body.body);
        self.rebound.pop();
        arguments
            .iter()
            .for_each(|argument| self.expr(&argument.value));
    }

    fn call(&mut self, callee: &'l TypedExpr, arguments: &'l [TypedArgument]) {
        for changed in Self::changed_locals(callee, arguments) {
            for argument in arguments {
                self.add_sources(changed, &argument.value);
            }
        }
        self.expr(callee);
        arguments
            .iter()
            .for_each(|argument| self.expr(&argument.value));
    }

    fn changed_locals(callee: &TypedExpr, arguments: &[TypedArgument]) -> Vec<BindingId> {
        let Type::Function(_, params, _) = &callee.ty else {
            return Vec::new();
        };
        params
            .iter()
            .zip(arguments)
            .filter(|(param, _)| matches!(param, Type::Mut(..)))
            .filter_map(|(_, argument)| Self::place_local(&argument.value))
            .collect()
    }

    fn place_local(target: &TypedExpr) -> Option<BindingId> {
        match &target.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(local)) => Some(*local),
            _ => None,
        }
    }

    fn recur(&mut self, arguments: &'l [TypedArgument]) {
        let Some(params) = self.rebound.last().copied() else {
            return;
        };
        for (param, argument) in params.iter().zip(arguments) {
            self.add_sources(param.binding, &argument.value);
        }
        arguments
            .iter()
            .for_each(|argument| self.expr(&argument.value));
    }

    fn bind(&mut self, binding: BindingId, value: &TypedExpr) {
        let sources = self.borrowed_names(value);
        self.locals.insert(binding, sources);
    }

    fn add_sources(&mut self, binding: BindingId, value: &TypedExpr) {
        let sources = self.borrowed_names(value);
        if let Some(local) = self.locals.get_mut(&binding) {
            local.extend(sources);
        }
    }

    fn borrowed_names(&self, value: &TypedExpr) -> FxHashSet<BindingId> {
        let mut names = FxHashSet::default();
        self.collect_borrowed(value, &mut names);
        names
    }

    fn collect_borrowed(&self, expr: &TypedExpr, names: &mut FxHashSet<BindingId>) {
        match &expr.kind {
            TypedExprKind::Borrow(_, place) => self.collect_place(place, names),
            _ if !self.borrows.may_hold_borrow(&expr.ty) => {}
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => {
                names.insert(*binding);
            }
            TypedExprKind::Control(TypedControl::Match(matched)) => matched
                .arms
                .iter()
                .flat_map(|arm| arm.body.expressions())
                .for_each(|value| self.collect_borrowed(value, names)),
            _ => expr
                .children()
                .into_iter()
                .for_each(|child| self.collect_borrowed(child, names)),
        }
    }

    fn collect_place(&self, place: &TypedExpr, names: &mut FxHashSet<BindingId>) {
        match &place.kind {
            TypedExprKind::Deref(inner)
            | TypedExprKind::Struct(TypedStructExpr::Field(inner, _)) => {
                self.collect_place(inner, names)
            }
            _ => self.collect_borrowed(place, names),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HeldBorrows;
    use crate::core::BindingId;
    use crate::ownership::OwnershipChecker;
    use crate::types::{TypedExprKind, TypedLambda, TypedProgram, TypedSource};

    struct KeptFixture {
        program: TypedProgram,
    }

    impl KeptFixture {
        const COUNTERS: &str = "
            (defn looped! (items)
              (push! items 1)
              (loop (it (iter items))
                (fn () (count it))))
            (defn beside! (x items)
              (set! x (+ x 1))
              (local it (iter items))
              (fn () (count it)))
            (defn adder! (x)
              (set! x (+ x 1))
              (local g (fn (y) (+ y 1)))
              (fn (y) (g y)))
            (defn rebound! (items other)
              (push! items 1)
              (loop (r (iter other) i 0)
                (if (< i 1)
                  (recur (iter items) (+ i 1))
                  (fn () (count r)))))
            (defn reset! (items other)
              (push! items 1)
              (local it (iter other))
              (set! it (iter items))
              (fn () (count it)))
            (defn referred! (x)
              (set! x (+ x 1))
              (local r (ref x))
              (fn () (deref r)))
            (defn taken! (x items)
              (set! x (+ x 1))
              (local it (take (iter items) x))
              (fn () (count it)))
            (defn same-len (s n) (= (len s) (deref n)))
            (defn checked! (x name)
              (set! x (+ x 1))
              (local s (as-str name))
              (println (same-len s (ref x)))
              (fn () (len s)))
            (defn framed! (items other)
              (loop (r (iter other))
                (local g
                  (fn (v) (if (> (len v) 9) (recur (ref items)) (len v))))
                (println (g (ref other)))
                (fn () (count r))))
            (defn matched! (x items)
              (set! x (+ x 1))
              (match (iter items)
                (it (fn () (count it)))))
            (defn main ())";

        fn new() -> Self {
            let mut program = TypedSource::expand(Self::COUNTERS).typed();
            OwnershipChecker::check(&mut program).expect("the fixture checks");
            Self { program }
        }

        fn kept_names(&self, function: &str) -> Vec<&str> {
            let lambda = self.lambda(function);
            let kept = HeldBorrows::new(&self.program).kept_parameters(lambda);
            lambda
                .params
                .iter()
                .filter(|param| kept.contains(&param.binding))
                .map(|param| self.name(param.binding))
                .collect()
        }

        fn lambda(&self, function: &str) -> &TypedLambda {
            self.program
                .modules
                .iter()
                .flat_map(|module| &module.definitions)
                .filter(|definition| self.name(definition.binding) == function)
                .find_map(|definition| match &definition.value.kind {
                    TypedExprKind::Fn(lambda) => Some(lambda.as_ref()),
                    _ => None,
                })
                .expect("the function is defined")
        }

        fn name(&self, binding: BindingId) -> &str {
            self.program.bindings.get(binding).name()
        }
    }

    #[test]
    fn a_recur_argument_adds_to_the_loop_binder() {
        assert_eq!(
            KeptFixture::new().kept_names("rebound!"),
            vec!["items", "other"]
        );
    }

    #[test]
    fn a_set_of_a_local_adds_to_its_sources() {
        assert_eq!(
            KeptFixture::new().kept_names("reset!"),
            vec!["items", "other"]
        );
    }

    #[test]
    fn a_borrow_of_a_changed_copy_keeps_it() {
        assert_eq!(KeptFixture::new().kept_names("referred!"), vec!["x"]);
    }

    #[test]
    fn a_copied_read_of_a_changed_parameter_keeps_nothing() {
        assert_eq!(KeptFixture::new().kept_names("taken!"), vec!["items"]);
    }

    #[test]
    fn a_local_read_by_a_call_gains_no_sources() {
        assert_eq!(KeptFixture::new().kept_names("checked!"), vec!["name"]);
    }

    #[test]
    fn a_recur_inside_a_closure_leaves_the_loop_binder() {
        assert_eq!(KeptFixture::new().kept_names("framed!"), vec!["other"]);
    }

    #[test]
    fn a_loop_binder_keeps_the_parameter_it_borrows() {
        assert_eq!(KeptFixture::new().kept_names("looped!"), vec!["items"]);
    }

    #[test]
    fn a_held_local_keeps_only_the_parameter_it_borrows() {
        assert_eq!(KeptFixture::new().kept_names("beside!"), vec!["items"]);
    }

    #[test]
    fn a_match_binder_keeps_the_parameter_its_scrutinee_borrows() {
        assert_eq!(KeptFixture::new().kept_names("matched!"), vec!["items"]);
    }

    #[test]
    fn a_local_closure_that_captures_nothing_keeps_no_parameter() {
        assert!(KeptFixture::new().kept_names("adder!").is_empty());
    }
}
