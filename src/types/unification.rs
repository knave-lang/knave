use super::declared_types::DeclaredTypes;
use super::display::TypeDisplay;
use super::expectation::Expectation;
use super::peeling::ReferencePeeling;
use super::problems::TypeProblems;
use super::ty::Type;
use super::typed::{TypedExpr, TypedExprKind, TypedStructExpr};
use super::unifier::{LiteralKind, Unifier, UnifyError};
use super::view_coercion::ViewCoercion;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub(super) struct Unification<'a> {
    unifier: &'a mut Unifier,
    display: TypeDisplay<'a>,
    declarations: DeclaredTypes<'a>,
}

impl<'a> Unification<'a> {
    pub(super) fn new(
        unifier: &'a mut Unifier,
        display: TypeDisplay<'a>,
        declarations: DeclaredTypes<'a>,
    ) -> Self {
        Self {
            unifier,
            display,
            declarations,
        }
    }

    pub(super) fn expect(
        &mut self,
        expectation: Expectation,
        found: &Type,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if self.coerces(expectation.expected, found, span) {
            return Ok(());
        }
        match self.unifier.unify(expectation.expected, found, span) {
            Ok(()) => Ok(()),
            Err(UnifyError::Infinite) => Err(TypeProblems::infinite(span)),
            Err(_) => Err(self.mismatch(expectation, found, span)),
        }
    }

    pub(super) fn coerces(&mut self, expected: &Type, found: &Type, span: Span) -> bool {
        match (self.unifier.resolve(expected), self.unifier.resolve(found)) {
            (Type::Ref(_, shared), Type::Ref(_, owned)) => {
                ViewCoercion::new(&shared, &owned).views_string()
                    || self.coerces(&shared, &owned, span)
            }
            (Type::Ref(_, shared), Type::Mut(_, mutable)) => {
                self.unifier.unify(&shared, &mutable, span).is_ok()
            }
            (Type::Mut(_, slice), Type::Mut(_, vector)) => self.coerces(&slice, &vector, span),
            (view, owned) => self.unifies_viewed_items(ViewCoercion::new(&view, &owned), span),
        }
    }

    pub(super) fn describe(&self, described: &Type) -> String {
        match described {
            Type::Var(var) => match self.unifier.literal_kind(*var) {
                Some(LiteralKind::Integer) => "an integer".to_string(),
                Some(LiteralKind::Float) => "a float".to_string(),
                None => "`_`".to_string(),
            },
            other => format!("`{}`", self.display.show(other)),
        }
    }

    pub(super) fn place_type(&self, value: &TypedExpr) -> Type {
        let TypedExprKind::Struct(TypedStructExpr::Field(owner, label)) = &value.kind else {
            return value.ty.clone();
        };
        let (owner, _) = ReferencePeeling::new(self.unifier).peel(&owner.ty);
        let Type::Named(binding, arguments) = owner else {
            return value.ty.clone();
        };
        self.declarations
            .field_type(binding, &label.name, &arguments)
            .unwrap_or_else(|| value.ty.clone())
    }

    fn unifies_viewed_items(&mut self, coercion: ViewCoercion, span: Span) -> bool {
        let Some(items) = self.declarations.viewed_items(coercion) else {
            return false;
        };
        items
            .iter()
            .all(|(item, element)| self.unifier.unify(item, element, span).is_ok())
    }

    fn mismatch(&self, expectation: Expectation, found: &Type, span: Span) -> Diagnostic {
        let expected = self.unifier.resolve(expectation.expected);
        let found = self.unifier.resolve(found);
        match &expected {
            Type::Generic(_) => TypeProblems::generic_mismatch(
                &self.display.show(&expected),
                &self.describe(&found),
                span,
            ),
            _ => TypeProblems::mismatch(
                &self.describe(&expected),
                &self.describe(&found),
                span,
                expectation.context,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::Unification;
    use crate::core::{FieldLabel, FieldSpelling, Literal, PrimitiveType};
    use crate::diagnostic::Diagnostic;
    use crate::source::{SourceId, Span};
    use crate::types::declared_types::DeclaredTypes;
    use crate::types::display::TypeDisplay;
    use crate::types::expectation::Expectation;
    use crate::types::type_declarations::TypeDeclarations;
    use crate::types::typed::{TypedExpr, TypedExprKind, TypedLeaf, TypedStructExpr};
    use crate::types::unifier::{LiteralKind, Unifier};
    use crate::types::{GenericId, GenericParam, Type, TypedProgram, TypedSource};

    struct UnificationFixture {
        program: TypedProgram,
        types: TypeDeclarations,
        unifier: Unifier,
        generics: FxHashMap<GenericId, GenericParam>,
        span: Span,
    }

    impl UnificationFixture {
        const SOURCE: &str = "
            (struct reading (value i64))
            (defn readings ((items (vec i64))) items)
            (defn window ((items (ref (slice i64)))) (len items))
            (defn octets ((items (ref (slice u8)))) (len items))
            (defn measure ((sample (ref reading))) (field sample value))
            (defn main () (println \"Leaf\"))";

        fn new() -> Self {
            let generic = GenericParam {
                id: Self::generic(),
                name: "T".into(),
                bounds: Vec::new(),
            };
            let mut program = TypedSource::expand(Self::SOURCE).typed();
            let types = std::mem::take(&mut program.declarations.types);
            Self {
                program,
                types,
                unifier: Unifier::default(),
                generics: FxHashMap::from_iter([(Self::generic(), generic)]),
                span: Span::new(SourceId::new(0), 0..0),
            }
        }

        fn unification(&mut self) -> Unification<'_> {
            let display = TypeDisplay::new(&self.program.bindings, &self.generics);
            let declarations = DeclaredTypes::new(&self.types);
            Unification::new(&mut self.unifier, display, declarations)
        }

        fn expect(&mut self, expected: &Type, found: &Type) -> Result<(), Diagnostic> {
            let span = self.span;
            self.unification()
                .expect(Expectation::plain(expected), found, span)
        }

        fn mismatch_label(&mut self, expected: &Type, found: &Type) -> String {
            let diagnostic = self.expect(expected, found).expect_err("the types differ");
            diagnostic.primary().message().to_string()
        }

        fn describe(&mut self, described: &Type) -> String {
            self.unification().describe(described)
        }

        fn place_type(&mut self, place: &TypedExpr) -> Type {
            self.unification().place_type(place)
        }

        fn parameter(&self, function: &str) -> Type {
            let definition = self
                .program
                .modules
                .iter()
                .flat_map(|module| &module.definitions)
                .find(|definition| self.program.bindings.get(definition.binding).name() == function)
                .expect("the source defines the function");
            let Type::Function(_, parameters, _) = &definition.scheme.body else {
                panic!("a defn has a function type");
            };
            parameters[0].clone()
        }

        fn referent(&self, function: &str) -> Type {
            self.parameter(function).referent().clone()
        }

        fn field_of(&self, owner: Type, field: &str) -> TypedExpr {
            let owner = TypedExpr {
                kind: TypedExprKind::Leaf(TypedLeaf::Literal(Literal::Bool(false))),
                ty: owner,
                span: self.span,
            };
            TypedExpr {
                kind: TypedExprKind::Struct(TypedStructExpr::Field(
                    Box::new(owner),
                    FieldLabel {
                        name: field.into(),
                        span: self.span,
                        spelling: FieldSpelling::Bare,
                    },
                )),
                ty: Self::boolean(),
                span: self.span,
            }
        }

        fn generic() -> GenericId {
            GenericId::new(0)
        }

        fn integer() -> Type {
            Type::Primitive(PrimitiveType::I64)
        }

        fn boolean() -> Type {
            Type::Primitive(PrimitiveType::Bool)
        }

        fn string() -> Type {
            Type::Primitive(PrimitiveType::String)
        }

        fn text() -> Type {
            Type::Primitive(PrimitiveType::Str)
        }
    }

    #[test]
    fn a_borrowed_string_fits_a_string_view() {
        let mut fixture = UnificationFixture::new();
        let view = Type::shared(UnificationFixture::text());
        let borrowed = Type::shared(UnificationFixture::string());
        assert!(fixture.expect(&view, &borrowed).is_ok());
    }

    #[test]
    fn a_string_view_does_not_fit_a_borrowed_string() {
        let mut fixture = UnificationFixture::new();
        let view = Type::shared(UnificationFixture::text());
        let borrowed = Type::shared(UnificationFixture::string());
        assert!(fixture.expect(&borrowed, &view).is_err());
    }

    #[test]
    fn a_string_borrowed_twice_fits_a_borrowed_view() {
        let mut fixture = UnificationFixture::new();
        let view = Type::shared(Type::shared(UnificationFixture::text()));
        let borrowed = Type::shared(Type::shared(UnificationFixture::string()));
        assert!(fixture.expect(&view, &borrowed).is_ok());
    }

    #[test]
    fn a_mutable_borrow_fits_a_shared_one_and_settles_its_referent() {
        let mut fixture = UnificationFixture::new();
        let referent = fixture.unifier.fresh();
        let shared = Type::shared(referent.clone());
        let mutable = Type::mutable(UnificationFixture::integer());
        assert!(fixture.expect(&shared, &mutable).is_ok());
        assert_eq!(
            fixture.unifier.resolve(&referent),
            UnificationFixture::integer()
        );
    }

    #[test]
    fn a_mutable_borrow_of_another_type_does_not_fit_a_shared_one() {
        let mut fixture = UnificationFixture::new();
        let shared = Type::shared(UnificationFixture::boolean());
        let mutable = Type::mutable(UnificationFixture::integer());
        assert!(fixture.expect(&shared, &mutable).is_err());
    }

    #[test]
    fn a_borrowed_vector_fits_a_borrowed_slice_of_its_items() {
        let mut fixture = UnificationFixture::new();
        let slice = fixture.parameter("window");
        let vector = Type::shared(fixture.parameter("readings"));
        assert!(fixture.expect(&slice, &vector).is_ok());
    }

    #[test]
    fn a_mutably_borrowed_vector_fits_a_mutable_slice_of_its_items() {
        let mut fixture = UnificationFixture::new();
        let slice = Type::mutable(fixture.referent("window"));
        let vector = Type::mutable(fixture.parameter("readings"));
        assert!(fixture.expect(&slice, &vector).is_ok());
    }

    #[test]
    fn a_borrowed_vector_does_not_fit_a_slice_of_other_items() {
        let mut fixture = UnificationFixture::new();
        let slice = fixture.parameter("octets");
        let vector = Type::shared(fixture.parameter("readings"));
        assert!(fixture.expect(&slice, &vector).is_err());
    }

    #[test]
    fn a_mismatch_names_the_expected_type_then_the_found_one() {
        let mut fixture = UnificationFixture::new();
        let label = fixture.mismatch_label(
            &UnificationFixture::integer(),
            &UnificationFixture::boolean(),
        );
        assert_eq!(label, "expected `i64`, found `bool`");
    }

    #[test]
    fn a_mismatch_names_what_a_variable_has_settled_to() {
        let mut fixture = UnificationFixture::new();
        let settled = fixture.unifier.fresh();
        fixture
            .expect(&settled, &UnificationFixture::integer())
            .expect("an open variable fits");
        let label = fixture.mismatch_label(&settled, &UnificationFixture::boolean());
        assert_eq!(label, "expected `i64`, found `bool`");
    }

    #[test]
    fn a_mismatch_against_a_type_parameter_names_the_parameter() {
        let mut fixture = UnificationFixture::new();
        let generic = Type::Generic(UnificationFixture::generic());
        let label = fixture.mismatch_label(&generic, &UnificationFixture::boolean());
        assert_eq!(label, "expected `T`, found `bool`");
    }

    #[test]
    fn a_mismatch_against_a_type_parameter_says_it_stands_for_every_type() {
        let mut fixture = UnificationFixture::new();
        let generic = Type::Generic(UnificationFixture::generic());
        let diagnostic = fixture
            .expect(&generic, &UnificationFixture::boolean())
            .expect_err("a parameter fits no concrete type");
        assert_eq!(
            diagnostic.notes(),
            ["`T` is a type parameter, so this code must work for every type `T` stands for"]
        );
    }

    #[test]
    fn a_type_that_would_contain_itself_is_reported_as_infinite() {
        let mut fixture = UnificationFixture::new();
        let open = fixture.unifier.fresh();
        let diagnostic = fixture
            .expect(&open, &Type::shared(open.clone()))
            .expect_err("the type contains itself");
        assert_eq!(diagnostic.title(), "this value's type would contain itself");
    }

    #[test]
    fn an_open_integer_literal_is_described_as_an_integer() {
        let mut fixture = UnificationFixture::new();
        let literal = fixture.unifier.fresh_literal(LiteralKind::Integer);
        assert_eq!(fixture.describe(&literal), "an integer");
    }

    #[test]
    fn an_open_float_literal_is_described_as_a_float() {
        let mut fixture = UnificationFixture::new();
        let literal = fixture.unifier.fresh_literal(LiteralKind::Float);
        assert_eq!(fixture.describe(&literal), "a float");
    }

    #[test]
    fn an_unknown_type_is_described_as_an_underscore() {
        let mut fixture = UnificationFixture::new();
        let unknown = fixture.unifier.fresh();
        assert_eq!(fixture.describe(&unknown), "`_`");
    }

    #[test]
    fn a_known_type_is_described_in_backticks() {
        let mut fixture = UnificationFixture::new();
        assert_eq!(fixture.describe(&UnificationFixture::integer()), "`i64`");
    }

    #[test]
    fn a_field_through_a_borrow_has_its_declared_type() {
        let mut fixture = UnificationFixture::new();
        let field = fixture.field_of(fixture.parameter("measure"), "value");
        assert_eq!(fixture.place_type(&field), UnificationFixture::integer());
    }

    #[test]
    fn an_undeclared_field_keeps_its_inferred_type() {
        let mut fixture = UnificationFixture::new();
        let field = fixture.field_of(fixture.parameter("measure"), "missing");
        assert_eq!(fixture.place_type(&field), UnificationFixture::boolean());
    }

    #[test]
    fn a_field_of_a_non_struct_keeps_its_inferred_type() {
        let mut fixture = UnificationFixture::new();
        let field = fixture.field_of(UnificationFixture::integer(), "value");
        assert_eq!(fixture.place_type(&field), UnificationFixture::boolean());
    }
}
