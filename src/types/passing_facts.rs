use super::passing_tables::PassingTables;
use super::ty::{Type, TypeVar};
use super::typed::{TypedExpr, TypedExprKind, TypedLeaf, TypedStructExpr};
use super::unifier::Unifier;
use crate::core::{Keyword, PrimitiveType};

pub(super) struct PassingFacts<'a> {
    tables: PassingTables<'a>,
    unifier: &'a Unifier,
}

impl<'a> PassingFacts<'a> {
    pub(super) fn new(tables: PassingTables<'a>, unifier: &'a Unifier) -> Self {
        Self { tables, unifier }
    }

    pub(super) fn passes_as_written(&self, value: &TypedExpr) -> bool {
        value.is_written_form() && !self.dereference_may_leave_borrow(value)
    }

    pub(super) fn referent_unknown(&self, argument: &Type) -> bool {
        matches!(
            argument.innermost_referent(),
            Type::Var(var) if self.unifier.literal_kind(*var).is_none()
        )
    }

    pub(super) fn holds_copy_value(&self, referent: &Type) -> bool {
        match referent {
            Type::Var(var) => self.unifier.literal_kind(*var).is_some(),
            _ => self.tables.copy_rules().is_small_copy(referent),
        }
    }

    pub(super) fn allows_only_copy_values(&self, var: TypeVar) -> bool {
        self.unifier.bounds(var).iter().any(|bound| {
            self.tables
                .copy_rules()
                .bound_allows_only_copy_values(bound.binding)
        })
    }

    pub(super) fn string_as_str_help(
        &self,
        expected: &Type,
        found: &Type,
        value: &TypedExpr,
    ) -> Option<String> {
        if !matches!(expected, Type::Primitive(PrimitiveType::Str))
            || !matches!(found, Type::Primitive(PrimitiveType::String))
        {
            return None;
        }
        let place = self.local_place(value)?;
        Some(format!("borrow it as a `str` with `(as-str {place})`"))
    }

    pub(super) fn field_place(&self, value: &TypedExpr) -> Option<String> {
        let TypedExprKind::Call(callee, arguments) = &value.kind else {
            return None;
        };
        let TypedExprKind::Leaf(TypedLeaf::Global(binding)) = callee.kind else {
            return None;
        };
        let label = self.tables.single_field_readers.get(&binding)?;
        let [argument] = arguments.as_slice() else {
            return None;
        };
        let place = self.local_place(&argument.value)?;
        Some(format!("({}{label} {place})", Keyword::MARKER))
    }

    fn dereference_may_leave_borrow(&self, value: &TypedExpr) -> bool {
        value.kind.is_dereference() && self.may_hold_borrow(&value.ty)
    }

    fn may_hold_borrow(&self, value: &Type) -> bool {
        let result = self.unifier.resolve(value);
        result.is_reference() || self.referent_unknown(&result)
    }

    fn local_place(&self, value: &TypedExpr) -> Option<String> {
        match &value.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => {
                Some(self.tables.bindings.get(*binding).name().to_string())
            }
            TypedExprKind::Struct(TypedStructExpr::Field(owner, label)) => Some(format!(
                "(field {} {})",
                self.local_place(owner)?,
                label.name
            )),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Type;
    use crate::types::passing_fixture::PassingFixture;
    use crate::types::ty::Bound;

    #[test]
    fn a_borrow_of_an_open_variable_has_an_unknown_referent() {
        let mut fixture = PassingFixture::new();
        let open = Type::shared(fixture.unifier().fresh());
        assert!(fixture.facts().referent_unknown(&open));
    }

    #[test]
    fn a_borrow_of_a_literal_has_a_known_referent() {
        let mut fixture = PassingFixture::new();
        let literal = Type::shared(fixture.literal());
        assert!(!fixture.facts().referent_unknown(&literal));
    }

    #[test]
    fn a_small_copy_or_a_literal_holds_a_copy_value() {
        let mut fixture = PassingFixture::new();
        let literal = fixture.literal();
        assert!(fixture.facts().holds_copy_value(&PassingFixture::int()));
        assert!(fixture.facts().holds_copy_value(&literal));
        assert!(!fixture.facts().holds_copy_value(&PassingFixture::string()));
    }

    #[test]
    fn a_written_borrow_passes_as_written() {
        let mut fixture = PassingFixture::new();
        let borrow = fixture.written_borrow(PassingFixture::int());
        assert!(fixture.facts().passes_as_written(&borrow));
        let local = fixture.local(PassingFixture::int());
        assert!(!fixture.facts().passes_as_written(&local));
    }

    #[test]
    fn a_number_bound_allows_only_copy_values() {
        let mut fixture = PassingFixture::new();
        let Type::Var(var) = fixture.unifier().fresh() else {
            unreachable!("fresh makes a variable")
        };
        assert!(!fixture.facts().allows_only_copy_values(var));
        let number = Bound::plain(fixture.trait_named("number"));
        fixture.unifier().add_bound(var, number);
        assert!(fixture.facts().allows_only_copy_values(var));
    }

    #[test]
    fn a_string_local_passed_as_a_str_suggests_as_str() {
        let mut fixture = PassingFixture::new();
        let local = fixture.local(PassingFixture::string());
        let help = (fixture.facts()).string_as_str_help(
            &PassingFixture::str(),
            &PassingFixture::string(),
            &local,
        );
        assert_eq!(
            help.as_deref(),
            Some("borrow it as a `str` with `(as-str main)`")
        );
    }
}
