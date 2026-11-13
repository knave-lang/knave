use super::access::Move;
use super::holdings::Holdings;
use super::lender::Lender;
use super::lending::Lending;
use super::loan_kind::{LoanKind, MutableLoan, SharedLoan};
use super::loan_place::LoanPlace;
use super::loan_rules::LoanRules;
use super::place_access::PlaceAccess;
use crate::source::Span;
use crate::types::{Borrow, Type, TypedArgument};

pub trait Passing {
    fn takes_the_place(&self) -> bool;

    fn pass(&self, argument: PassedArgument, lender: &mut Lender<'_>) -> Holdings;
}

pub struct PassedArgument {
    place: Option<LoanPlace>,
    span: Span,
    held: Holdings,
}

pub struct Lends {
    kind: &'static dyn LoanKind,
}

pub struct Moves;

pub struct Copies;

pub struct Holds;

impl Lends {
    const SHARED: Self = Self { kind: &SharedLoan };
    const MUTABLE: Self = Self { kind: &MutableLoan };
}

impl dyn Passing {
    pub fn of(
        argument: &TypedArgument,
        parameter: Option<&Type>,
        rules: &LoanRules<'_>,
    ) -> &'static dyn Passing {
        match argument.borrow {
            Borrow::Shared => &Lends::SHARED,
            Borrow::Mutable | Borrow::Reborrow => &Lends::MUTABLE,
            Borrow::Deref(_) => &Copies,
            Borrow::Passthrough | Borrow::Collapse(_) if argument.value.ty.is_reference() => {
                <dyn Passing>::through_reference(parameter)
            }
            Borrow::None if rules.moves(&argument.value) => &Moves,
            _ => &Holds,
        }
    }

    fn through_reference(parameter: Option<&Type>) -> &'static dyn Passing {
        match parameter {
            Some(Type::Mut(..)) => &Lends::MUTABLE,
            Some(Type::Ref(..)) => &Lends::SHARED,
            _ => &Holds,
        }
    }
}

impl PassedArgument {
    pub fn new(argument: &TypedArgument, held: Holdings) -> Self {
        Self {
            place: LoanPlace::of(&argument.value),
            span: argument.value.span,
            held,
        }
    }
}

impl Passing for Lends {
    fn takes_the_place(&self) -> bool {
        true
    }

    fn pass(&self, argument: PassedArgument, lender: &mut Lender<'_>) -> Holdings {
        match argument.place {
            Some(place) => {
                let lent = lender.lend(Lending::new(place, self.kind, argument.span));
                argument.held.merged(lent)
            }
            None => argument.held,
        }
    }
}

impl Passing for Moves {
    fn takes_the_place(&self) -> bool {
        true
    }

    fn pass(&self, argument: PassedArgument, lender: &mut Lender<'_>) -> Holdings {
        if let Some(place) = argument.place {
            lender.access(PlaceAccess::new(place, &Move, argument.span));
        }
        argument.held
    }
}

impl Passing for Copies {
    fn takes_the_place(&self) -> bool {
        false
    }

    fn pass(&self, _argument: PassedArgument, _lender: &mut Lender<'_>) -> Holdings {
        Holdings::default()
    }
}

impl Passing for Holds {
    fn takes_the_place(&self) -> bool {
        false
    }

    fn pass(&self, argument: PassedArgument, _lender: &mut Lender<'_>) -> Holdings {
        argument.held
    }
}

#[cfg(test)]
mod tests {
    use super::{PassedArgument, Passing};
    use crate::ownership::lambda_source::LambdaSource;
    use crate::ownership::loans::control_flow_graph::ControlFlowGraph;
    use crate::ownership::loans::holdings::Holdings;
    use crate::ownership::loans::lender::Lender;
    use crate::ownership::loans::loan_names::LoanNames;
    use crate::ownership::loans::loan_rules::LoanRules;
    use crate::ownership::loans::loan_table::LoanTable;
    use crate::ownership::loans::point_id::PointId;
    use crate::source::SourceMap;
    use crate::types::{Borrow, Type, TypedArgument, TypedExpr, TypedExprKind, TypedLeaf};

    struct PassingFixture {
        source: LambdaSource,
        sources: SourceMap,
    }

    struct Outcome {
        takes_the_place: bool,
        accesses: Vec<String>,
        passes_a_loan: bool,
        passes_its_own_holdings: bool,
    }

    impl PassingFixture {
        fn binding_of(argument: &TypedArgument) -> crate::core::BindingId {
            match argument.value.kind {
                TypedExprKind::Leaf(TypedLeaf::Local(binding)) => binding,
                _ => unreachable!("the fixture passes locals"),
            }
        }

        const SOURCE: &str = "(defn main ()\n  (local v (vec 1))\n  (local n 1)\n  (local r (ref v))\n  (println n (len v) (len r)))";

        fn new() -> Self {
            let mut sources = SourceMap::default();
            sources.add("fixture.knv", Self::SOURCE);
            Self {
                source: LambdaSource::new(Self::SOURCE),
                sources,
            }
        }

        fn argument(&self, local: &str, borrow: Borrow) -> TypedArgument {
            let binder = self.source.binder(local);
            TypedArgument {
                value: TypedExpr {
                    kind: TypedExprKind::Leaf(TypedLeaf::Local(binder.binding)),
                    ty: binder.ty.clone(),
                    span: binder.span,
                },
                borrow,
            }
        }

        fn outcome(&self, local: &str, borrow: Borrow, parameter: Option<&Type>) -> Outcome {
            let argument = self.argument(local, borrow);
            let rules = LoanRules::of(self.source.program());
            let passing = <dyn Passing>::of(&argument, parameter, &rules);
            let mut graph = ControlFlowGraph::new();
            let mut loans = LoanTable::default();
            let passed = {
                let mut lender = Lender::new(&mut graph, &mut loans, PointId::ENTRY);
                let held = Holdings::of_value(Self::binding_of(&argument));
                passing.pass(PassedArgument::new(&argument, held), &mut lender)
            };
            let names = LoanNames::new(&self.source.program().bindings, &self.sources);
            let accesses = graph
                .points()
                .flat_map(|(_, facts)| {
                    facts
                        .accesses()
                        .iter()
                        .map(|access| access.written(&names))
                        .collect::<Vec<_>>()
                })
                .map(|written| {
                    written
                        .split_once(' ')
                        .map(|(_, verb)| verb.to_string())
                        .unwrap_or(written)
                })
                .collect();
            Outcome {
                takes_the_place: passing.takes_the_place(),
                accesses,
                passes_a_loan: passed.loans().next().is_some(),
                passes_its_own_holdings: passed.values().next().is_some(),
            }
        }
    }

    #[test]
    fn a_borrow_adjustment_lends_its_place_and_passes_the_loan() {
        let fixture = PassingFixture::new();
        let shared = fixture.outcome("v", Borrow::Shared, None);
        let changed = fixture.outcome("v", Borrow::Reborrow, None);
        assert!(shared.takes_the_place && shared.passes_a_loan);
        assert_eq!(shared.accesses, ["borrow `v`"]);
        assert_eq!(changed.accesses, ["change `v`"]);
    }

    #[test]
    fn a_dereference_copies_and_passes_nothing() {
        let copied = PassingFixture::new().outcome("r", Borrow::Deref(1), None);
        assert!(!copied.takes_the_place && !copied.passes_a_loan);
        assert!(copied.accesses.is_empty());
    }

    #[test]
    fn a_reference_passed_through_lends_as_its_parameter_takes_it() {
        let fixture = PassingFixture::new();
        let changing = Type::mutable(Type::unit());
        let reading = Type::shared(Type::unit());
        let lent = fixture.outcome("r", Borrow::Passthrough, Some(&changing));
        assert!(lent.passes_its_own_holdings);
        assert_eq!(
            fixture
                .outcome("r", Borrow::Passthrough, Some(&changing))
                .accesses,
            ["change `r`"]
        );
        assert_eq!(
            fixture
                .outcome("r", Borrow::Passthrough, Some(&reading))
                .accesses,
            ["borrow `r`"]
        );
        assert!(
            fixture
                .outcome("r", Borrow::Passthrough, None)
                .accesses
                .is_empty()
        );
    }

    #[test]
    fn an_unadjusted_value_moves_unless_it_is_copy() {
        let fixture = PassingFixture::new();
        let moved = fixture.outcome("v", Borrow::None, None);
        assert!(moved.takes_the_place);
        assert_eq!(moved.accesses, ["move `v`"]);
        assert!(fixture.outcome("n", Borrow::None, None).accesses.is_empty());
    }
}
