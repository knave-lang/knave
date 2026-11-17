use std::collections::BTreeSet;

use super::first_positions::{FirstPositionsAndBeyond, FirstPositionsOnly};
use super::pipelines::Pipelines;
use super::position_tie::PositionTie;
use super::tied_positions::TiedPositions;
use super::ty::{GenericId, Lifetime, Scheme, Type};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TemplateTies {
    lifetime: TiedPositions,
    held: TiedPositions,
    beneath: FirstPositionsOnly,
    stored: FirstPositionsAndBeyond,
    fixing: FirstPositionsOnly,
}

impl TemplateTies {
    pub fn new(lifetime: &[usize], held: &[usize]) -> Self {
        Self {
            lifetime: TiedPositions::of(lifetime.iter().copied()),
            held: TiedPositions::of(held.iter().copied()),
            beneath: FirstPositionsOnly::default(),
            stored: FirstPositionsAndBeyond::default(),
            fixing: FirstPositionsOnly::default(),
        }
    }

    #[cfg(test)]
    pub fn beneath(self, positions: &[usize]) -> Self {
        Self {
            beneath: FirstPositionsOnly::of(positions.iter().copied()),
            ..self
        }
    }

    #[cfg(test)]
    pub fn fixing(self, positions: &[usize]) -> Self {
        Self {
            fixing: FirstPositionsOnly::of(positions.iter().copied()),
            ..self
        }
    }

    #[cfg(test)]
    pub fn storing(self, positions: &[usize]) -> Self {
        Self {
            stored: FirstPositionsAndBeyond::of(positions.iter().copied()),
            ..self
        }
    }

    pub fn of(scheme: &Scheme, pipelines: Pipelines) -> Option<Self> {
        let Type::Function(_, parameters, result) = &scheme.body else {
            return Some(Self::default());
        };
        let generics: Vec<GenericId> = scheme.params.iter().map(|param| param.id).collect();
        let lifetime = TiedPositions::of(Self::lifetime_ties(parameters, result)?).union(
            &TiedPositions::of(Self::kept_references(parameters, result, pipelines)),
        );
        let held = TiedPositions::of(Self::held_ties(parameters, result, &generics)).union(
            &TiedPositions::of(Self::adapted_iterators(parameters, result, pipelines)),
        );
        let beneath = FirstPositionsOnly::of(held.positions().filter(|position| {
            !lifetime.contains(*position)
                && matches!(parameters[*position], Type::Ref(..) | Type::Mut(..))
        }));
        Some(Self {
            lifetime,
            held,
            beneath,
            stored: FirstPositionsAndBeyond::of(Self::stored_references(parameters)),
            fixing: FirstPositionsOnly::of(Self::fixing_the_stored_variable(parameters)),
        })
    }

    pub fn widened(known: Option<Self>, other: Option<Self>) -> Option<Self> {
        let (known, other) = (known?, other?);
        let held = known.held.union(&other.held);
        let beneath =
            FirstPositionsOnly::of(held.positions().filter(|position| {
                known.reads_beneath(*position) && other.reads_beneath(*position)
            }));
        Some(Self {
            lifetime: known.lifetime.union(&other.lifetime),
            held,
            beneath,
            stored: known.stored.union(other.stored),
            fixing: known.fixing.intersection(other.fixing),
        })
    }

    pub fn tie_at(&self, position: usize) -> Option<PositionTie> {
        if self.lifetime.contains(position) {
            Some(PositionTie::Lent)
        } else if self.beneath.contains(position) {
            Some(PositionTie::HeldBeneath)
        } else if self.held.contains(position) {
            Some(PositionTie::Held)
        } else {
            None
        }
    }

    pub fn stores_the_reference(
        &self,
        position: usize,
        uncollapsed: impl IntoIterator<Item = usize>,
    ) -> bool {
        self.stored.contains(position)
            && uncollapsed
                .into_iter()
                .all(|other| !self.fixing.contains(other))
    }

    fn reads_beneath(&self, position: usize) -> bool {
        !self.held.contains(position) || self.beneath.contains(position)
    }

    fn held_ties(parameters: &[Type], result: &Type, generics: &[GenericId]) -> Vec<usize> {
        let shared: Vec<GenericId> = generics
            .iter()
            .copied()
            .filter(|generic| result.mentions(*generic))
            .collect();
        parameters
            .iter()
            .enumerate()
            .filter(|(_, parameter)| shared.iter().any(|generic| parameter.mentions(*generic)))
            .map(|(position, _)| position)
            .collect()
    }

    fn fixing_the_stored_variable(parameters: &[Type]) -> Vec<usize> {
        let stored: Vec<GenericId> = parameters
            .iter()
            .filter_map(|parameter| match parameter.innermost_referent() {
                Type::Generic(generic) => Some(*generic),
                _ => None,
            })
            .collect();
        match stored.split_first() {
            Some((generic, others)) if others.iter().all(|other| other == generic) => parameters
                .iter()
                .enumerate()
                .filter(|(_, parameter)| parameter.mentions(*generic))
                .map(|(position, _)| position)
                .collect(),
            _ => Vec::new(),
        }
    }

    fn stored_references(parameters: &[Type]) -> Vec<usize> {
        parameters
            .iter()
            .enumerate()
            .filter(|(_, parameter)| matches!(parameter.innermost_referent(), Type::Generic(_)))
            .map(|(position, _)| position)
            .collect()
    }

    fn adapted_iterators(parameters: &[Type], result: &Type, pipelines: Pipelines) -> Vec<usize> {
        if !pipelines.holds_an_iterator(result) {
            return Vec::new();
        }
        parameters
            .iter()
            .enumerate()
            .filter(|(_, parameter)| pipelines.holds_an_iterator(parameter))
            .map(|(position, _)| position)
            .collect()
    }

    fn kept_references(parameters: &[Type], result: &Type, pipelines: Pipelines) -> Vec<usize> {
        if !pipelines.holds_an_iterator(result) {
            return Vec::new();
        }
        let sourced = parameters
            .iter()
            .any(|parameter| pipelines.holds_an_iterator(parameter));
        parameters
            .iter()
            .enumerate()
            .filter(|(_, parameter)| match parameter {
                Type::Ref(Lifetime::Elided, _) | Type::Generic(_) => !sourced,
                Type::Mut(Lifetime::Elided, referent) => {
                    !sourced || pipelines.holds_an_iterator(referent)
                }
                _ => false,
            })
            .map(|(position, _)| position)
            .collect()
    }

    fn lifetime_ties(parameters: &[Type], result: &Type) -> Option<Vec<usize>> {
        let mut tied = BTreeSet::new();
        for output in Self::lifetimes_in(result) {
            tied.extend(Self::sources_of(output, parameters)?);
        }
        Some(tied.into_iter().collect())
    }

    fn sources_of(output: Lifetime, parameters: &[Type]) -> Option<Vec<usize>> {
        let inputs: Vec<(usize, Lifetime)> = parameters
            .iter()
            .enumerate()
            .flat_map(|(position, parameter)| {
                Self::lifetimes_in(parameter)
                    .into_iter()
                    .map(move |lifetime| (position, lifetime))
            })
            .collect();
        match output {
            Lifetime::Written(_) => Some(
                inputs
                    .iter()
                    .filter(|(_, input)| *input == output)
                    .map(|(position, _)| *position)
                    .collect(),
            ),
            Lifetime::Elided => match inputs.as_slice() {
                [(position, _)] => Some(vec![*position]),
                _ => None,
            },
        }
    }

    fn lifetimes_in(ty: &Type) -> Vec<Lifetime> {
        let own = match ty {
            Type::Function(..) => return Vec::new(),
            Type::Ref(lifetime, _) | Type::Mut(lifetime, _) => Some(*lifetime),
            _ => None,
        };
        own.into_iter()
            .chain(ty.parts().into_iter().flat_map(Self::lifetimes_in))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::TemplateTies;
    use crate::core::{BindingId, ClosureKind, PrimitiveType};
    use crate::types::PositionTie;
    use crate::types::first_positions::FirstPositionsOnly;
    use crate::types::pipelines::Pipelines;
    use crate::types::rust_types::RustTypeTable;
    use crate::types::ty::{GenericId, GenericParam, Lifetime, Scheme, Type};

    struct TiesFixture;

    impl TiesFixture {
        fn generic() -> Type {
            Type::Generic(GenericId::new(0))
        }

        fn written(inner: Type) -> Type {
            Type::Ref(Lifetime::written(BindingId::from_index(7)), Box::new(inner))
        }

        fn other_generic() -> Type {
            Type::Generic(GenericId::new(1))
        }

        fn param(index: usize, name: &str) -> GenericParam {
            GenericParam {
                id: GenericId::new(index),
                name: Rc::from(name),
                bounds: Vec::new(),
            }
        }

        fn ties(parameters: Vec<Type>, result: Type) -> Option<TemplateTies> {
            let scheme = Scheme {
                lifetimes: Vec::new(),
                params: vec![Self::param(0, "a"), Self::param(1, "b")],
                body: Type::Function(ClosureKind::Fn, parameters, Box::new(result)),
            };
            TemplateTies::of(&scheme, Pipelines::new(&RustTypeTable::default()))
        }
    }

    #[test]
    fn an_elided_result_borrows_the_one_parameter_that_holds_a_reference() {
        let index = Type::Primitive(PrimitiveType::Usize);
        let at = TiesFixture::ties(
            vec![Type::shared(TiesFixture::generic()), index],
            Type::shared(TiesFixture::generic()),
        );
        assert_eq!(
            at,
            Some(TemplateTies::new(&[0], &[0]).storing(&[0]).fixing(&[0]))
        );
    }

    #[test]
    fn an_elided_result_with_two_borrowed_parameters_is_not_settled() {
        let text = Type::str_ref();
        assert_eq!(
            TiesFixture::ties(vec![text.clone(), text.clone()], text),
            None
        );
    }

    #[test]
    fn a_written_lifetime_ties_every_parameter_that_carries_it() {
        let text = || TiesFixture::written(Type::Primitive(PrimitiveType::Str));
        let ties = TiesFixture::ties(vec![text(), text(), Type::str_ref()], text());
        assert_eq!(ties, Some(TemplateTies::new(&[0, 1], &[])));
    }

    #[test]
    fn a_type_variable_ties_what_a_parameter_holds_and_not_its_loan() {
        let vector = Type::mutable(TiesFixture::generic());
        let pop = TiesFixture::ties(vec![vector], TiesFixture::generic());
        let either = TiesFixture::ties(
            vec![TiesFixture::generic(), TiesFixture::generic()],
            TiesFixture::generic(),
        );
        assert_eq!(pop, Some(TemplateTies::new(&[], &[0]).beneath(&[0])));
        assert_eq!(
            either,
            Some(
                TemplateTies::new(&[], &[0, 1])
                    .storing(&[0, 1])
                    .fixing(&[0, 1])
            )
        );
    }

    #[test]
    fn a_parameter_with_only_a_type_variable_the_result_lacks_holds_nothing_for_it() {
        let lookup = TiesFixture::ties(
            vec![TiesFixture::generic(), TiesFixture::other_generic()],
            TiesFixture::generic(),
        );
        assert_eq!(lookup, Some(TemplateTies::new(&[], &[0]).storing(&[0, 1])));
    }

    #[test]
    fn only_a_type_variable_beneath_a_parameter_s_references_stores_the_reference() {
        let nested = Type::shared(Type::shared(TiesFixture::generic()));
        let vector = Type::mutable(TiesFixture::generic());
        let ties = TiesFixture::ties(vec![nested, Type::str_ref(), vector], Type::unit())
            .expect("the result holds no borrow");
        assert!(ties.stores_the_reference(0, []));
        assert!(!ties.stores_the_reference(1, []));
        assert!(!ties.stores_the_reference(2, []));
    }

    #[test]
    fn each_position_has_the_strongest_tie_its_parameter_carries() {
        let vector = Type::mutable(TiesFixture::generic());
        let index = Type::Primitive(PrimitiveType::Usize);
        let pop = TiesFixture::ties(vec![vector, index], TiesFixture::generic())
            .expect("a type variable ties the result");
        let at = TiesFixture::ties(
            vec![Type::shared(TiesFixture::generic())],
            Type::shared(TiesFixture::generic()),
        )
        .expect("an elided result borrows its one reference");
        let either = TiesFixture::ties(vec![TiesFixture::generic()], TiesFixture::generic())
            .expect("a type variable ties the result");
        assert_eq!(at.tie_at(0), Some(PositionTie::Lent));
        assert_eq!(pop.tie_at(0), Some(PositionTie::HeldBeneath));
        assert_eq!(either.tie_at(0), Some(PositionTie::Held));
        assert_eq!(pop.tie_at(1), None);
    }

    #[test]
    fn a_reference_stored_by_either_widened_signature_stays_stored() {
        let concrete = TiesFixture::ties(vec![Type::str_ref()], Type::str_ref());
        let generic = TiesFixture::ties(
            vec![Type::shared(TiesFixture::generic())],
            Type::shared(TiesFixture::generic()),
        );
        let widened = TemplateTies::widened(concrete, generic).expect("both are settled");
        assert!(widened.stores_the_reference(0, []));
    }

    #[test]
    fn two_stored_type_variables_keep_each_reference_whole_whatever_the_others_pass() {
        let pair = TiesFixture::ties(
            vec![TiesFixture::generic(), TiesFixture::other_generic()],
            Type::unit(),
        )
        .expect("the result holds no borrow");
        let single = TiesFixture::ties(
            vec![TiesFixture::generic(), TiesFixture::generic()],
            Type::unit(),
        )
        .expect("the result holds no borrow");
        assert_eq!(pair.fixing, FirstPositionsOnly::default());
        assert_eq!(single.fixing, FirstPositionsOnly::of([0, 1]));
    }

    #[test]
    fn a_widened_variable_is_fixed_only_where_every_signature_fixes_it() {
        let single = TiesFixture::ties(
            vec![TiesFixture::generic(), TiesFixture::generic()],
            Type::unit(),
        );
        let concrete = TiesFixture::ties(vec![Type::str_ref(), Type::str_ref()], Type::unit());
        let widened = TemplateTies::widened(single, concrete).expect("both are settled");
        assert_eq!(widened.fixing, FirstPositionsOnly::default());
    }

    #[test]
    fn a_position_past_the_first_word_keeps_its_reference_whole() {
        let ties = TiesFixture::ties(vec![Type::str_ref()], Type::unit())
            .expect("the result holds no borrow");
        assert!(!ties.stores_the_reference(0, []));
        assert!(ties.stores_the_reference(64, []));
    }

    #[test]
    fn a_function_parameter_brings_no_input_lifetime() {
        let callback = Type::Function(
            ClosureKind::Fn,
            vec![Type::str_ref()],
            Box::new(Type::bool()),
        );
        let ties = TiesFixture::ties(vec![Type::str_ref(), callback], Type::str_ref());
        assert_eq!(ties, Some(TemplateTies::new(&[0], &[])));
    }

    #[test]
    fn a_template_seen_at_two_types_keeps_the_ties_of_both() {
        let lent = Some(TemplateTies::new(&[0], &[]));
        let held = Some(TemplateTies::new(&[], &[0]));
        assert_eq!(
            TemplateTies::widened(lent, held),
            Some(TemplateTies::new(&[0], &[0]))
        );
    }

    #[test]
    fn a_template_unsettled_at_one_type_is_unsettled() {
        let lent = Some(TemplateTies::new(&[0], &[]));
        assert_eq!(TemplateTies::widened(lent, None), None);
    }

    #[test]
    fn a_widened_tie_reads_beneath_a_reference_only_when_every_type_does() {
        let behind = Some(TemplateTies::new(&[], &[0]).beneath(&[0]));
        let plain = Some(TemplateTies::new(&[], &[0]));
        let untied = Some(TemplateTies::new(&[], &[]));
        assert_eq!(
            TemplateTies::widened(behind.clone(), plain),
            Some(TemplateTies::new(&[], &[0]))
        );
        assert_eq!(
            TemplateTies::widened(behind, untied),
            Some(TemplateTies::new(&[], &[0]).beneath(&[0]))
        );
    }
}
