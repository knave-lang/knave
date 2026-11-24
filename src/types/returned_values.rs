use rustc_hash::FxHashMap;

use super::{
    Pipelines, Type, TypedArgument, TypedBlock, TypedBodyItem, TypedControl, TypedExpr,
    TypedExprKind, TypedLambda, TypedLeaf, TypedProgram,
};
use crate::core::BindingId;

type Lets<'e> = FxHashMap<BindingId, &'e TypedExpr>;

pub struct ReturnedValues<'p> {
    program: &'p TypedProgram,
}

impl<'p> ReturnedValues<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub fn closures<'e>(&self, lambda: &'e TypedLambda) -> Vec<&'e TypedExpr> {
        let mut closures = Vec::new();
        Self::visit_block(&lambda.body, Lets::default(), &mut |value, lets| {
            self.collect_closures(value, lets, &mut closures);
        });
        closures
    }

    pub fn visit_values<'e>(&self, lambda: &'e TypedLambda, mut visit: impl FnMut(&'e TypedExpr)) {
        Self::visit_block(&lambda.body, Lets::default(), &mut |value, _| visit(value));
    }

    pub fn of_value<'e>(&self, value: &'e TypedExpr) -> Vec<&'e TypedExpr> {
        let mut values = Vec::new();
        Self::visit_value(value, &Lets::default(), &mut |value, _| values.push(value));
        values
    }

    pub fn chosen_values<'e>(&self, value: &'e TypedExpr) -> Option<Vec<&'e TypedExpr>> {
        let chosen = self.of_value(value);
        let is_its_own_value = matches!(chosen.as_slice(), [only] if std::ptr::eq(*only, value));
        (!is_its_own_value).then_some(chosen)
    }

    fn visit_block<'e>(
        block: &'e TypedBlock,
        mut lets: Lets<'e>,
        visit: &mut impl FnMut(&'e TypedExpr, &Lets<'e>),
    ) {
        for item in &block.items {
            if let TypedBodyItem::Let(binder, value) = item {
                lets.insert(binder.binding, value);
            }
        }
        if let Some(tail) = block.tail() {
            Self::visit_value(tail, &lets, visit);
        }
    }

    fn visit_value<'e>(
        value: &'e TypedExpr,
        lets: &Lets<'e>,
        visit: &mut impl FnMut(&'e TypedExpr, &Lets<'e>),
    ) {
        match &value.kind {
            TypedExprKind::Control(TypedControl::Block(block)) => {
                Self::visit_block(block, lets.clone(), visit);
            }
            TypedExprKind::Control(TypedControl::If(_, then, otherwise)) => {
                Self::visit_value(then, lets, visit);
                Self::visit_value(otherwise, lets, visit);
            }
            TypedExprKind::Control(TypedControl::Match(matched)) => {
                for arm in &matched.arms {
                    Self::visit_block(&arm.body, lets.clone(), visit);
                }
            }
            TypedExprKind::Call(callee, arguments)
                if let TypedExprKind::Fn(body) = &callee.kind =>
            {
                Self::visit_block(&body.body, Self::bind_loop(body, arguments, lets), visit);
            }
            _ => visit(value, lets),
        }
    }

    fn bind_loop<'e>(
        body: &'e TypedLambda,
        arguments: &'e [TypedArgument],
        lets: &Lets<'e>,
    ) -> Lets<'e> {
        let mut bound = lets.clone();
        for (param, argument) in body.params.iter().zip(arguments) {
            bound.insert(param.binding, &argument.value);
        }
        bound
    }

    fn collect_closures<'e>(
        &self,
        value: &'e TypedExpr,
        lets: &Lets<'e>,
        closures: &mut Vec<&'e TypedExpr>,
    ) {
        match &value.kind {
            TypedExprKind::Fn(_) => {
                closures.push(value);
                self.collect_captured(value, lets, closures);
            }
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) if lets.contains_key(binding) => {
                self.collect_closures(lets[binding], lets, closures);
            }
            _ if self.may_hold_closure(&value.ty) => {
                for child in value.children() {
                    self.collect_closures(child, lets, closures);
                }
            }
            _ => {}
        }
    }

    fn collect_captured<'e>(
        &self,
        closure: &'e TypedExpr,
        lets: &Lets<'e>,
        closures: &mut Vec<&'e TypedExpr>,
    ) {
        for child in closure.children() {
            match &child.kind {
                TypedExprKind::Leaf(TypedLeaf::Local(binding)) if lets.contains_key(binding) => {
                    self.collect_closures(lets[binding], lets, closures);
                }
                _ => self.collect_captured(child, lets, closures),
            }
        }
    }

    fn may_hold_closure(&self, ty: &Type) -> bool {
        match ty {
            Type::Function(..) => true,
            Type::Ref(_, inner) | Type::Mut(_, inner) => self.may_hold_closure(inner),
            Type::Named(_, arguments) => {
                Pipelines::new(self.program.declarations.types().rust_types()).is_pipeline(ty)
                    || arguments
                        .iter()
                        .any(|argument| self.may_hold_closure(argument))
            }
            Type::Var(_) | Type::Generic(_) | Type::Primitive(_) | Type::TraitObject(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReturnedValues;
    use crate::types::{TypedExpr, TypedExprKind, TypedProgram, TypedSource};

    struct ChoosingFixture {
        program: TypedProgram,
    }

    impl ChoosingFixture {
        const SOURCE: &str = "
            (defn branch (c) (if c 1 2))
            (defn arms (o) (match o ((some n) n) (none 0)))
            (defn block (n) (do (println n) n))
            (defn looped (n) (loop (i n) (if (> i 0) (recur (- i 1)) i)))
            (defn called (n) (+ n 1))
            (defn main ())";

        fn new() -> Self {
            Self {
                program: TypedSource::expand(Self::SOURCE).typed(),
            }
        }

        fn tail(&self, function: &str) -> &TypedExpr {
            self.program
                .modules
                .iter()
                .flat_map(|module| &module.definitions)
                .filter(|definition| {
                    self.program.bindings.get(definition.binding).name() == function
                })
                .find_map(|definition| match &definition.value.kind {
                    TypedExprKind::Fn(lambda) => lambda.body.tail(),
                    _ => None,
                })
                .expect("the function returns a value")
        }

        fn chooses_other_values(&self, function: &str) -> bool {
            let value = self.tail(function);
            ReturnedValues::new(&self.program)
                .chosen_values(value)
                .is_some_and(|chosen| chosen.iter().all(|other| !std::ptr::eq(*other, value)))
        }
    }

    #[test]
    fn each_form_that_chooses_its_value_chooses_other_expressions() {
        let fixture = ChoosingFixture::new();
        for function in ["branch", "arms", "block", "looped"] {
            assert!(fixture.chooses_other_values(function), "{function}");
        }
    }

    #[test]
    fn a_call_chooses_no_value() {
        let fixture = ChoosingFixture::new();
        let value = fixture.tail("called");
        assert!(
            ReturnedValues::new(&fixture.program)
                .chosen_values(value)
                .is_none()
        );
    }
}
