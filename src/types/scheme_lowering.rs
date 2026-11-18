use rustc_hash::FxHashMap;

use super::lowered_associated_value::LoweredAssociatedValue;
use super::lowered_binders::LoweredBinders;
use super::problems::TypeProblems;
use super::trait_rows_reach::Implementor;
use super::ty::{GenericId, GenericParam, Scheme, Type};
use super::written_type_lowering::WrittenTypeLowering;
use super::written_type_variables::WrittenTypeVariables;
use crate::core::{
    AssociatedValue, BindingId, ImplAssociatedType, RustTraitImplementor, TypeExpr, TypeExprKind,
};
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub(super) type LoweredImplementor = (Type, Vec<GenericParam>, Vec<(BindingId, Type)>);

pub(super) struct SchemeLowering<'l> {
    types: WrittenTypeLowering<'l>,
    variables: WrittenTypeVariables<'l>,
}

impl<'l> SchemeLowering<'l> {
    pub(super) fn new(types: WrittenTypeLowering<'l>, variables: WrittenTypeVariables<'l>) -> Self {
        Self { types, variables }
    }

    pub(super) fn lower_scheme(&mut self, type_expr: &TypeExpr) -> Result<Scheme, Diagnostic> {
        let TypeExprKind::Where(binders, inner) = type_expr.kind() else {
            return Ok(Scheme::monomorphic(
                self.types.lower_type(type_expr, &FxHashMap::default())?,
            ));
        };
        let binders = self.types.lower_binders(binders)?;
        Ok(Scheme {
            lifetimes: binders.lifetimes,
            params: binders.params,
            body: self.types.lower_type(inner, &binders.scope)?,
        })
    }

    pub(super) fn lower_implementor(
        &mut self,
        entry: &RustTraitImplementor,
        trait_name: &str,
    ) -> Result<(Implementor, Vec<LoweredAssociatedValue>), Diagnostic> {
        let implementor = &entry.implementor;
        let binders = match implementor.kind() {
            TypeExprKind::Where(binders, _) => self.types.lower_binders(binders)?,
            _ => LoweredBinders::default(),
        };
        let (body, argument) =
            self.lower_implementor_body(Self::where_body(implementor), &binders.scope)?;
        Self::reject_bare_implementor(&body, implementor.span(), trait_name)?;
        let values = self.lower_associated_values(&entry.associated, &binders.scope)?;
        let lowered = Implementor {
            scheme: Scheme {
                lifetimes: binders.lifetimes,
                params: binders.params,
                body,
            },
            argument,
        };
        Ok((lowered, values))
    }

    pub(super) fn lower_impl_implementor(
        &mut self,
        implementor: &TypeExpr,
        associated: &[ImplAssociatedType],
    ) -> Result<LoweredImplementor, Diagnostic> {
        let mut params = Vec::new();
        let mut scope = FxHashMap::default();
        for variable in self.variables.of(implementor) {
            let param = self.types.declare_variable(variable);
            scope.insert(variable, param.id);
            params.push(param);
        }
        let body = self.types.lower_type(implementor, &scope)?;
        let values = associated
            .iter()
            .map(|associated| {
                Ok((
                    associated.trait_type,
                    self.types.lower_type(&associated.value, &scope)?,
                ))
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        Ok((body, params, values))
    }

    fn where_body(implementor: &TypeExpr) -> &TypeExpr {
        match implementor.kind() {
            TypeExprKind::Where(_, inner) => inner,
            _ => implementor,
        }
    }

    fn lower_associated_values(
        &mut self,
        values: &[AssociatedValue],
        scope: &FxHashMap<BindingId, GenericId>,
    ) -> Result<Vec<LoweredAssociatedValue>, Diagnostic> {
        values
            .iter()
            .map(|associated| {
                Ok(LoweredAssociatedValue {
                    projection: associated.binding,
                    value: self.types.lower_type(&associated.value, scope)?,
                    span: associated.span,
                })
            })
            .collect()
    }

    fn lower_implementor_body(
        &mut self,
        type_expr: &TypeExpr,
        scope: &FxHashMap<BindingId, GenericId>,
    ) -> Result<(Type, Option<Type>), Diagnostic> {
        if let TypeExprKind::With(implementor, argument) = type_expr.kind() {
            let body = self.types.lower_type(implementor, scope)?;
            let argument = self.types.lower_type(argument, scope)?;
            Ok((body, Some(argument)))
        } else {
            Ok((self.types.lower_type(type_expr, scope)?, None))
        }
    }

    fn reject_bare_implementor(
        body: &Type,
        span: Span,
        trait_name: &str,
    ) -> Result<(), Diagnostic> {
        if matches!(body, Type::Generic(_)) {
            return Err(TypeProblems::bare_implementor(trait_name, span));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{ImplAssociatedType, RustTraitImplementor, TypeExprKind};
    use crate::types::ty::{Bound, Type};
    use crate::types::written_type_fixture::WrittenTypeFixture as Fixture;

    #[test]
    fn a_type_without_where_lowers_to_a_monomorphic_scheme() {
        let mut fixture = Fixture::new();
        let scheme = fixture
            .checker
            .written_schemes()
            .lower_scheme(&Fixture::integer())
            .expect("a primitive lowers");
        assert!(scheme.params.is_empty());
        assert!(scheme.lifetimes.is_empty());
    }

    #[test]
    fn a_where_declares_a_generic_per_type_binder_and_keeps_its_lifetimes() {
        let mut fixture = Fixture::new();
        let lifetime = fixture.lifetime("'a");
        let variable = fixture.type_variable("t");
        let written = Fixture::expression(TypeExprKind::Where(
            vec![
                Fixture::lifetime_binder(lifetime),
                Fixture::bounded(variable, &[]),
            ],
            Box::new(Fixture::named(variable, Vec::new())),
        ));
        let scheme = fixture
            .checker
            .written_schemes()
            .lower_scheme(&written)
            .expect("the scheme lowers");
        assert_eq!(scheme.lifetimes, vec![lifetime]);
        assert_eq!(scheme.params.len(), 1);
        assert_eq!(scheme.body, Type::Generic(scheme.params[0].id));
    }

    #[test]
    fn each_type_binder_keeps_its_own_bounds_after_a_lifetime() {
        let mut fixture = Fixture::new();
        let lifetime = fixture.lifetime("'a");
        let variable = fixture.type_variable("t");
        let display = fixture.named_trait("display");
        let written = Fixture::expression(TypeExprKind::Where(
            vec![
                Fixture::lifetime_binder(lifetime),
                Fixture::bounded(variable, &[display]),
            ],
            Box::new(Fixture::integer()),
        ));
        let scheme = fixture
            .checker
            .written_schemes()
            .lower_scheme(&written)
            .expect("the scheme lowers");
        assert_eq!(scheme.params[0].bounds, vec![Bound::plain(display)]);
    }

    #[test]
    fn an_impl_implementor_turns_its_variables_into_generics() {
        let mut fixture = Fixture::new();
        let variable = fixture.type_variable("t");
        let wrapper = fixture.declared_struct("wrapper", 1);
        let item = fixture.associated_type("item");
        let implementor = Fixture::named(wrapper, vec![Fixture::named(variable, Vec::new())]);
        let associated = [ImplAssociatedType {
            trait_type: item,
            name: "item".into(),
            value: Fixture::named(variable, Vec::new()),
        }];
        let (body, params, values) = fixture
            .checker
            .written_schemes()
            .lower_impl_implementor(&implementor, &associated)
            .expect("the implementor lowers");
        let generic = Type::Generic(params[0].id);
        assert_eq!(body, Type::Named(wrapper, vec![generic.clone()]));
        assert_eq!(values, vec![(item, generic)]);
    }

    #[test]
    fn a_bare_generic_implementor_is_refused() {
        let mut fixture = Fixture::new();
        let variable = fixture.type_variable("t");
        let entry = RustTraitImplementor {
            implementor: Fixture::expression(TypeExprKind::Where(
                vec![Fixture::bounded(variable, &[])],
                Box::new(Fixture::named(variable, Vec::new())),
            )),
            associated: Vec::new(),
        };
        let lowered = fixture
            .checker
            .written_schemes()
            .lower_implementor(&entry, "shown");
        assert!(lowered.is_err());
    }

    #[test]
    fn a_named_implementor_lowers_with_its_generics() {
        let mut fixture = Fixture::new();
        let variable = fixture.type_variable("t");
        let wrapper = fixture.declared_struct("wrapper", 1);
        let entry = RustTraitImplementor {
            implementor: Fixture::expression(TypeExprKind::Where(
                vec![Fixture::bounded(variable, &[])],
                Box::new(Fixture::named(
                    wrapper,
                    vec![Fixture::named(variable, Vec::new())],
                )),
            )),
            associated: Vec::new(),
        };
        let (implementor, values) = fixture
            .checker
            .written_schemes()
            .lower_implementor(&entry, "shown")
            .expect("a named implementor lowers");
        assert_eq!(implementor.scheme.params.len(), 1);
        assert!(implementor.argument.is_none());
        assert!(values.is_empty());
    }

    #[test]
    fn an_implementor_with_an_argument_keeps_the_argument() {
        let mut fixture = Fixture::new();
        let wrapper = fixture.declared_struct("wrapper", 0);
        let entry = RustTraitImplementor {
            implementor: Fixture::expression(TypeExprKind::With(
                Box::new(Fixture::named(wrapper, Vec::new())),
                Box::new(Fixture::integer()),
            )),
            associated: Vec::new(),
        };
        let (implementor, _) = fixture
            .checker
            .written_schemes()
            .lower_implementor(&entry, "shown")
            .expect("a named implementor lowers");
        assert_eq!(
            implementor.argument,
            Some(Type::Primitive(crate::core::PrimitiveType::I64))
        );
    }
}
