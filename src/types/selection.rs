use rustc_hash::FxHashMap;

use super::traits::Implementor;
use super::ty::{Bound, GenericId, Type};
pub struct Requirement {
    pub bound: Bound,
    pub implementor: Type,
}

pub enum Selection {
    Implemented {
        requirements: Vec<Requirement>,
        argument: Option<Type>,
    },
    Undecided,
    Missing,
}

impl Selection {
    #[inline]
    pub fn best(self, other: Selection) -> Selection {
        match (self, other) {
            (implemented @ Selection::Implemented { .. }, _)
            | (_, implemented @ Selection::Implemented { .. }) => implemented,
            (Selection::Undecided, _) | (_, Selection::Undecided) => Selection::Undecided,
            (Selection::Missing, Selection::Missing) => Selection::Missing,
        }
    }
}

pub enum PatternMatch {
    Matched(FxHashMap<GenericId, Type>),
    Undecided,
    Missing,
}

pub struct ImplementorMatch<'s> {
    implementor: &'s Implementor,
    bindings: FxHashMap<GenericId, Type>,
    undecided: bool,
}

impl<'s> ImplementorMatch<'s> {
    // Called out of line, the hot selection loop costs tens of millions more
    // instructions on the corpus, and `#[inline]` alone does not keep it inline.
    #[inline(always)]
    pub fn select(
        implementor: &'s Implementor,
        bound: &Bound,
        target: &Type,
        associated: bool,
    ) -> Selection {
        let mut matcher = Self::new(implementor);
        if !matcher.matches(&implementor.scheme.body, target) {
            return Selection::Missing;
        }
        if associated {
            matcher.associated(bound)
        } else {
            matcher.parameter(bound, target)
        }
    }

    pub fn settle(implementor: &'s Implementor, bound: &Bound, target: &Type) -> Option<Selection> {
        let mut matcher = Self::new(implementor);
        let pattern_argument = matcher.pattern_argument();
        let target_argument = bound.argument.as_ref()?;
        let settles = matcher.matches(&implementor.scheme.body, target)
            && matcher.matches(pattern_argument, target_argument);
        settles.then(|| Selection::Implemented {
            requirements: matcher.requirements(),
            argument: Some(pattern_argument.substitute(&matcher.bindings)),
        })
    }

    fn new(implementor: &'s Implementor) -> Self {
        Self {
            implementor,
            bindings: FxHashMap::default(),
            undecided: false,
        }
    }

    fn pattern_argument(&self) -> &'s Type {
        self.implementor
            .argument
            .as_ref()
            .unwrap_or(&self.implementor.scheme.body)
    }

    pub fn match_pattern(
        implementor: &'s Implementor,
        pattern: &Type,
        target: &Type,
    ) -> PatternMatch {
        let mut matcher = Self::new(implementor);
        match (matcher.matches(pattern, target), matcher.undecided) {
            (false, _) => PatternMatch::Missing,
            (true, true) => PatternMatch::Undecided,
            (true, false) => PatternMatch::Matched(matcher.bindings),
        }
    }

    fn associated(&mut self, bound: &Bound) -> Selection {
        if self.undecided {
            return Selection::Undecided;
        }
        let argument = self.pattern_argument().substitute(&self.bindings);
        if let Some(expected) = &bound.argument
            && !matches!(expected, Type::Var(_))
            && !matches!(argument, Type::Var(_))
            && argument != *expected
        {
            return Selection::Missing;
        }
        Selection::Implemented {
            requirements: self.requirements(),
            argument: Some(argument),
        }
    }

    fn parameter(&mut self, bound: &Bound, target: &Type) -> Selection {
        let pattern_argument = self.pattern_argument();
        let target_argument = bound.argument.as_ref().unwrap_or(target);
        if !self.matches(pattern_argument, target_argument) {
            return Selection::Missing;
        }
        if self.undecided {
            return Selection::Undecided;
        }
        Selection::Implemented {
            requirements: self.requirements(),
            argument: None,
        }
    }

    fn matches(&mut self, pattern: &Type, target: &Type) -> bool {
        match (pattern, target) {
            (Type::Generic(generic), _) if self.is_parameter(*generic) => {
                self.bind(*generic, target)
            }
            (_, Type::Var(_)) => self.leave_undecided(),
            _ => {
                pattern.shares_head_with(target)
                    && self.matches_all(pattern.parts(), target.parts())
            }
        }
    }

    fn matches_all(&mut self, patterns: Vec<&Type>, targets: Vec<&Type>) -> bool {
        patterns
            .into_iter()
            .zip(targets)
            .all(|(pattern, target)| self.matches(pattern, target))
    }

    fn leave_undecided(&mut self) -> bool {
        self.undecided = true;
        true
    }

    fn is_parameter(&self, generic: GenericId) -> bool {
        self.implementor
            .scheme
            .params
            .iter()
            .any(|param| param.id == generic)
    }

    fn bind(&mut self, generic: GenericId, target: &Type) -> bool {
        let bound = self
            .bindings
            .entry(generic)
            .or_insert_with(|| target.clone());
        if bound == target {
            return true;
        }
        Self::could_equal(bound, target) && self.leave_undecided()
    }

    fn could_equal(left: &Type, right: &Type) -> bool {
        match (left, right) {
            (Type::Var(_), _) | (_, Type::Var(_)) => true,
            (Type::Named(left_head, left_arguments), Type::Named(right_head, right_arguments)) => {
                left_head == right_head
                    && left_arguments.len() == right_arguments.len()
                    && left_arguments
                        .iter()
                        .zip(right_arguments)
                        .all(|(left, right)| Self::could_equal(left, right))
            }
            (Type::Ref(_, left), Type::Ref(_, right))
            | (Type::Mut(_, left), Type::Mut(_, right)) => Self::could_equal(left, right),
            _ => left == right,
        }
    }

    fn requirements(&self) -> Vec<Requirement> {
        self.implementor
            .scheme
            .params
            .iter()
            .filter_map(|param| Some((param, self.bindings.get(&param.id)?)))
            .flat_map(|(param, implementor)| {
                param.bounds.iter().map(|bound| Requirement {
                    bound: bound.substituted(&self.bindings),
                    implementor: implementor.clone(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind, PrimitiveType};
    use crate::types::ty::{GenericParam, Scheme, TypeVar};

    struct Shapes {
        pair: BindingId,
        option: BindingId,
        vec: BindingId,
    }

    impl Shapes {
        fn new() -> Self {
            let mut bindings = BindingArena::default();
            let mut add = |name: &str| {
                bindings.add(BindingInfo::new(
                    name.into(),
                    BindingKind::TypeVariable,
                    None,
                ))
            };
            Self {
                pair: add("pair"),
                option: add("option"),
                vec: add("vec"),
            }
        }

        fn pair(&self, left: Type, right: Type) -> Type {
            Type::Named(self.pair, vec![left, right])
        }

        fn pair_of_one_parameter(&self) -> Implementor {
            let item = GenericId::new(0);
            Implementor {
                scheme: Scheme {
                    lifetimes: Vec::new(),
                    params: vec![GenericParam {
                        id: item,
                        name: "item".into(),
                        bounds: Vec::new(),
                    }],
                    body: self.pair(Type::Generic(item), Type::Generic(item)),
                },
                argument: None,
            }
        }

        fn matches_row(&self, target: &Type) -> PatternMatch {
            let row = self.pair_of_one_parameter();
            ImplementorMatch::match_pattern(&row, &row.scheme.body, target)
        }

        fn open() -> Type {
            Type::Var(TypeVar::new(0))
        }

        fn primitive(primitive: PrimitiveType) -> Type {
            Type::Primitive(primitive)
        }
    }

    #[test]
    fn a_row_that_gives_one_parameter_two_closed_types_is_missing() {
        let shapes = Shapes::new();
        let target = shapes.pair(
            Shapes::primitive(PrimitiveType::I64),
            Shapes::primitive(PrimitiveType::U32),
        );
        assert!(matches!(shapes.matches_row(&target), PatternMatch::Missing));
    }

    #[test]
    fn a_row_that_gives_one_parameter_an_open_type_waits() {
        let shapes = Shapes::new();
        let target = shapes.pair(Shapes::open(), Shapes::primitive(PrimitiveType::U32));
        assert!(matches!(
            shapes.matches_row(&target),
            PatternMatch::Undecided
        ));
    }

    #[test]
    fn an_open_part_waits_for_a_type_of_the_same_shape() {
        let shapes = Shapes::new();
        let u32 = Shapes::primitive(PrimitiveType::U32);
        let target = shapes.pair(
            shapes.pair(Shapes::open(), u32.clone()),
            shapes.pair(Shapes::primitive(PrimitiveType::I64), u32),
        );
        assert!(matches!(
            shapes.matches_row(&target),
            PatternMatch::Undecided
        ));
    }

    #[test]
    fn another_shape_never_meets_a_type_with_an_open_part() {
        let shapes = Shapes::new();
        let i64 = Shapes::primitive(PrimitiveType::I64);
        let target = shapes.pair(i64.clone(), shapes.pair(Shapes::open(), i64));
        assert!(matches!(shapes.matches_row(&target), PatternMatch::Missing));
    }

    #[test]
    fn a_borrow_waits_only_for_a_borrow_of_an_open_type() {
        let shapes = Shapes::new();
        let i64 = Shapes::primitive(PrimitiveType::I64);
        let borrows = shapes.pair(Type::shared(Shapes::open()), Type::shared(i64.clone()));
        let borrow_and_value = shapes.pair(Type::shared(Shapes::open()), i64);
        assert!(matches!(
            shapes.matches_row(&borrows),
            PatternMatch::Undecided
        ));
        assert!(matches!(
            shapes.matches_row(&borrow_and_value),
            PatternMatch::Missing
        ));
    }

    #[test]
    fn a_closed_part_that_differs_keeps_two_shapes_apart() {
        let shapes = Shapes::new();
        let target = shapes.pair(
            shapes.pair(Shapes::open(), Shapes::primitive(PrimitiveType::Bool)),
            shapes.pair(Shapes::open(), Shapes::primitive(PrimitiveType::Str)),
        );
        assert!(matches!(shapes.matches_row(&target), PatternMatch::Missing));
    }

    #[test]
    fn two_heads_of_one_part_never_meet() {
        let shapes = Shapes::new();
        let target = shapes.pair(
            Type::Named(shapes.option, vec![Shapes::open()]),
            Type::Named(shapes.vec, vec![Shapes::primitive(PrimitiveType::I64)]),
        );
        assert!(matches!(shapes.matches_row(&target), PatternMatch::Missing));
    }
}
