use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::body_flows::BodyFlows;
use super::callee_parameters::{CalleeParameters, OpenCallees};
use super::callee_visit::CalleeVisit;
use super::definition_lookup::DefinitionLookup;
use super::definition_stores::DefinitionStores;
use super::lambda_flows::{FunctionFlows, LambdaFlows};
use super::parameter_joins::ParameterJoins;
use super::region_rules::RegionRules;
use super::store_sites::StoreSites;
use crate::core::BindingId;
use crate::ownership::TailCalls;
use crate::types::{StoredFlows, Type, TypedExprKind, TypedLambda, TypedProgram};

pub struct ResultBorrows<'a> {
    program: &'a TypedProgram,
    definitions: &'a dyn DefinitionLookup,
    parameters_by_callee: RefCell<FxHashMap<BindingId, CalleeParameters>>,
    open_callees: RefCell<OpenCallees>,
    stores_by_definition: RefCell<FxHashMap<BindingId, Rc<DefinitionStores>>>,
    no_stores: Rc<DefinitionStores>,
}

impl<'a> ResultBorrows<'a> {
    pub fn new(program: &'a TypedProgram, definitions: &'a dyn DefinitionLookup) -> Self {
        Self {
            program,
            definitions,
            parameters_by_callee: RefCell::new(FxHashMap::default()),
            open_callees: RefCell::new(OpenCallees::default()),
            stores_by_definition: RefCell::new(FxHashMap::default()),
            no_stores: Rc::default(),
        }
    }

    pub fn analyse(&self, lambda: &TypedLambda) -> LambdaFlows {
        BodyFlows::of(self, lambda, None)
    }

    pub fn analyse_shared_arguments(&self, lambda: &TypedLambda) -> LambdaFlows {
        BodyFlows::of_shared_arguments(self, lambda)
    }

    pub fn analyse_function(&self, function: BindingId, lambda: &TypedLambda) -> FunctionFlows {
        ParameterJoins::of_function(self, lambda, &TailCalls::of(Some(function), lambda))
    }

    pub fn definition_stores(&self, definition: BindingId) -> Rc<DefinitionStores> {
        if let Some(stores) = self.stores_by_definition.borrow().get(&definition) {
            return Rc::clone(stores);
        }
        let stores = match self.callee_lambda(definition) {
            Some(lambda) if self.may_receive_store(lambda) => {
                self.receiving_stores(definition, lambda)
            }
            _ => Rc::clone(&self.no_stores),
        };
        self.remember_stores(definition, &stores);
        stores
    }

    fn receiving_stores(
        &self,
        definition: BindingId,
        lambda: &TypedLambda,
    ) -> Rc<DefinitionStores> {
        self.remember_stores(definition, &self.no_stores);
        Rc::new(DefinitionStores::new(self.stored_at_sites(lambda), lambda))
    }

    fn stored_at_sites(&self, lambda: &TypedLambda) -> StoredFlows {
        let sites = StoreSites::of(self, lambda);
        if sites.is_empty() {
            return StoredFlows::default();
        }
        ParameterJoins::of_stores(self, lambda, &sites)
    }

    fn remember_stores(&self, definition: BindingId, stores: &Rc<DefinitionStores>) {
        self.stores_by_definition
            .borrow_mut()
            .insert(definition, Rc::clone(stores));
    }

    fn may_receive_store(&self, lambda: &TypedLambda) -> bool {
        lambda
            .params
            .iter()
            .any(|param| self.referent_holds_borrow(&param.ty))
    }

    fn referent_holds_borrow(&self, ty: &Type) -> bool {
        match ty {
            Type::Ref(..) => false,
            Type::Mut(_, referent) => self.program.type_holds_borrow(referent),
            owned => self.program.type_holds_borrow(owned),
        }
    }

    fn visit(&self, callee: BindingId, lambda: &TypedLambda) -> CalleeVisit {
        let order = self.open_callees.borrow_mut().open(callee);
        let visit = self.proposed_visit(callee, lambda, order);
        if visit.earliest_reached == Some(order) {
            self.settle_group(order);
            return self.settled_visit(callee);
        }
        visit
    }

    fn proposed_visit(&self, callee: BindingId, lambda: &TypedLambda, order: usize) -> CalleeVisit {
        let analysed = BodyFlows::of(self, lambda, Some(order));
        let parameters = analysed.parameters(lambda);
        self.open_callees.borrow_mut().widen(callee, &parameters);
        CalleeVisit {
            parameters,
            earliest_reached: analysed.earliest_reached,
        }
    }

    fn settled_visit(&self, callee: BindingId) -> CalleeVisit {
        let parameters = self.parameters_by_callee.borrow().get(&callee).cloned();
        CalleeVisit::settled(parameters.unwrap_or_default())
    }

    fn settle_group(&self, order: usize) {
        let group = self.open_callees.borrow().group_from(order);
        if self.open_callees.borrow().any_assumed(&group) {
            self.reach_fixed_point(&group);
        }
        let settled = self.open_callees.borrow_mut().close(&group);
        self.parameters_by_callee.borrow_mut().extend(settled);
    }

    fn reach_fixed_point(&self, group: &[BindingId]) {
        while self.widen_group(group) {}
    }

    fn widen_group(&self, group: &[BindingId]) -> bool {
        let grown: Vec<bool> = group
            .iter()
            .map(|callee| self.widen_member(*callee))
            .collect();
        grown.contains(&true)
    }

    fn widen_member(&self, callee: BindingId) -> bool {
        let Some(lambda) = self.callee_lambda(callee) else {
            return false;
        };
        let analysed = BodyFlows::of(self, lambda, None);
        let parameters = analysed.parameters(lambda);
        self.open_callees.borrow_mut().widen(callee, &parameters)
    }

    fn callee_lambda(&self, callee: BindingId) -> Option<&'a TypedLambda> {
        match &self.definitions.definition(callee)?.value.kind {
            TypedExprKind::Fn(lambda) => Some(lambda),
            _ => None,
        }
    }
}

impl RegionRules for ResultBorrows<'_> {
    fn program(&self) -> &TypedProgram {
        self.program
    }

    fn carries_borrow(&self, ty: &Type) -> bool {
        self.program.type_holds_borrow(ty) || ty.is_unsized()
    }

    fn callee_visit(&self, callee: BindingId) -> CalleeVisit {
        let settled = self.parameters_by_callee.borrow().get(&callee).cloned();
        if let Some(parameters) = settled {
            return CalleeVisit::settled(parameters);
        }
        let open = self.open_callees.borrow().order(callee);
        if let Some(order) = open {
            return CalleeVisit {
                parameters: self.open_callees.borrow_mut().assume(callee),
                earliest_reached: Some(order),
            };
        }
        match self.callee_lambda(callee) {
            Some(lambda) => self.visit(callee, lambda),
            None => CalleeVisit::default(),
        }
    }

    fn stores_of(&self, definition: BindingId) -> Rc<DefinitionStores> {
        self.definition_stores(definition)
    }
}
