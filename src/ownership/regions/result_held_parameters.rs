use std::cell::RefCell;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::BindingId;
use crate::types::{
    BorrowingTypes, DefinitionTable, ReturnedValues, Type, TypedArgument, TypedBinder, TypedBlock,
    TypedBodyItem, TypedControl, TypedExpr, TypedExprKind, TypedLambda, TypedLeaf, TypedMatch,
    TypedProgram, TypedStructExpr,
};

pub(super) struct ResultHeldParameters<'a> {
    program: &'a TypedProgram,
    definitions: &'a DefinitionTable<'a>,
    callees: RefCell<FxHashMap<BindingId, CalleeHeld>>,
}

enum CalleeHeld {
    Open,
    Held(FxHashSet<BindingId>),
}

struct FunctionScope<'l> {
    borrows: &'l ResultHeldParameters<'l>,
    locals: FxHashMap<BindingId, FxHashSet<BindingId>>,
    rebound: Vec<&'l [TypedBinder]>,
}

impl<'a> ResultHeldParameters<'a> {
    pub(super) fn new(program: &'a TypedProgram, definitions: &'a DefinitionTable<'a>) -> Self {
        Self {
            program,
            definitions,
            callees: RefCell::default(),
        }
    }

    pub(super) fn of(&self, lambda: &TypedLambda) -> FxHashSet<BindingId> {
        let scope = FunctionScope::of(self, lambda);
        let returned = ReturnedValues::new(self.program);
        let mut held = Vec::new();
        for closure in returned.closures(lambda) {
            Self::collect_held(closure, &mut held);
        }
        for value in returned.values(lambda) {
            if matches!(value.ty, Type::Function(..)) {
                held.extend(self.held_through(value, &scope));
            }
        }
        let parameters: FxHashSet<BindingId> =
            lambda.params.iter().map(|param| param.binding).collect();
        self.sources(held, &scope)
            .intersection(&parameters)
            .copied()
            .collect()
    }

    fn held_through(&self, value: &TypedExpr, scope: &FunctionScope) -> FxHashSet<BindingId> {
        match self.held_arguments(value) {
            Some(arguments) => arguments
                .into_iter()
                .flat_map(|argument| scope.borrowed_names(argument))
                .collect(),
            None => scope.borrowed_names(value),
        }
    }

    fn held_arguments<'e>(&self, value: &'e TypedExpr) -> Option<Vec<&'e TypedExpr>> {
        let called = self.definitions.called_function(value)?;
        let held = self.callee_held(called.function, called.lambda)?;
        let held_arguments = called
            .lambda
            .params
            .iter()
            .zip(called.arguments)
            .filter(|(parameter, _)| held.contains(&parameter.binding))
            .map(|(_, argument)| &argument.value)
            .collect();
        Some(held_arguments)
    }

    fn callee_held(
        &self,
        function: BindingId,
        lambda: &TypedLambda,
    ) -> Option<FxHashSet<BindingId>> {
        if let Some(known) = self.callees.borrow().get(&function) {
            return match known {
                CalleeHeld::Open => None,
                CalleeHeld::Held(held) => Some(held.clone()),
            };
        }
        self.callees.borrow_mut().insert(function, CalleeHeld::Open);
        let held = self.of(lambda);
        self.callees
            .borrow_mut()
            .insert(function, CalleeHeld::Held(held.clone()));
        Some(held)
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
        let mut reached: FxHashSet<BindingId> = pending.iter().copied().collect();
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
        matches!(ty, Type::Function(..)) || BorrowingTypes::of(self.program).keeps_borrow(ty)
    }
}

impl<'l> FunctionScope<'l> {
    fn of(borrows: &'l ResultHeldParameters<'l>, lambda: &'l TypedLambda) -> Self {
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
    use super::ResultHeldParameters;
    use crate::core::BindingId;
    use crate::ownership::OwnershipChecker;
    use crate::types::{DefinitionTable, TypedExprKind, TypedLambda, TypedProgram, TypedSource};

    struct HeldFixture {
        program: TypedProgram,
    }

    impl HeldFixture {
        const COUNTERS: &str = "
            (defn sizer (items) (fn () (len items)))
            (defn wrap (items other) (println (len other)) (sizer items))
            (defn counter (items) (let (n (len items)) (fn () n)))
            (defn wrapc (items) (counter items))
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
            (defn wrapped! (s)
              (set! s (str \"xy\"))
              (local cs (some (chars (as-str s))))
              (fn () (match cs ((some c) (count c)) (none 0))))
            (defn main ())";

        fn new() -> Self {
            let mut program = TypedSource::expand(Self::COUNTERS).typed();
            OwnershipChecker::check(&mut program).expect("the fixture checks");
            Self { program }
        }

        fn held_names(&self, function: &str) -> Vec<&str> {
            let lambda = self.lambda(function);
            let definitions = DefinitionTable::of(&self.program);
            let held = ResultHeldParameters::new(&self.program, &definitions).of(lambda);
            lambda
                .params
                .iter()
                .filter(|param| held.contains(&param.binding))
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
    fn a_captured_parameter_is_held() {
        assert_eq!(HeldFixture::new().held_names("sizer"), vec!["items"]);
    }

    #[test]
    fn a_borrow_passed_to_a_returned_call_is_held() {
        assert_eq!(HeldFixture::new().held_names("wrap"), vec!["items"]);
    }

    #[test]
    fn a_borrow_passed_to_a_returned_call_that_holds_nothing_is_not_held() {
        assert!(HeldFixture::new().held_names("wrapc").is_empty());
    }

    #[test]
    fn a_parameter_read_into_a_copied_local_is_not_held() {
        assert!(HeldFixture::new().held_names("counter").is_empty());
    }

    #[test]
    fn a_recur_argument_adds_to_the_loop_binder() {
        assert_eq!(
            HeldFixture::new().held_names("rebound!"),
            vec!["items", "other"]
        );
    }

    #[test]
    fn a_set_of_a_local_adds_to_its_sources() {
        assert_eq!(
            HeldFixture::new().held_names("reset!"),
            vec!["items", "other"]
        );
    }

    #[test]
    fn a_borrow_of_a_changed_copy_keeps_it() {
        assert_eq!(HeldFixture::new().held_names("referred!"), vec!["x"]);
    }

    #[test]
    fn a_copied_read_of_a_changed_parameter_keeps_nothing() {
        assert_eq!(HeldFixture::new().held_names("taken!"), vec!["items"]);
    }

    #[test]
    fn a_local_read_by_a_call_gains_no_sources() {
        assert_eq!(HeldFixture::new().held_names("checked!"), vec!["name"]);
    }

    #[test]
    fn a_recur_inside_a_closure_leaves_the_loop_binder() {
        assert_eq!(HeldFixture::new().held_names("framed!"), vec!["other"]);
    }

    #[test]
    fn a_loop_binder_keeps_the_parameter_it_borrows() {
        assert_eq!(HeldFixture::new().held_names("looped!"), vec!["items"]);
    }

    #[test]
    fn a_held_local_keeps_only_the_parameter_it_borrows() {
        assert_eq!(HeldFixture::new().held_names("beside!"), vec!["items"]);
    }

    #[test]
    fn a_match_binder_keeps_the_parameter_its_scrutinee_borrows() {
        assert_eq!(HeldFixture::new().held_names("matched!"), vec!["items"]);
    }

    #[test]
    fn a_local_holding_an_iterator_inside_another_type_keeps_the_parameter_it_borrows() {
        assert_eq!(HeldFixture::new().held_names("wrapped!"), vec!["s"]);
    }

    #[test]
    fn a_local_closure_that_captures_nothing_keeps_no_parameter() {
        assert!(HeldFixture::new().held_names("adder!").is_empty());
    }
}
