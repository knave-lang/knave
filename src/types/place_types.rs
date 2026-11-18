use super::declared_types::DeclaredTypes;
use super::peeling::ReferencePeeling;
use super::ty::Type;
use super::typed::{TypedExpr, TypedExprKind, TypedStructExpr};
use super::unifier::Unifier;
use crate::core::FieldSpelling;

pub(super) struct PlaceTypes<'a> {
    unifier: &'a Unifier,
    declarations: DeclaredTypes<'a>,
}

impl<'a> PlaceTypes<'a> {
    pub(super) fn new(unifier: &'a Unifier, declarations: DeclaredTypes<'a>) -> Self {
        Self {
            unifier,
            declarations,
        }
    }

    pub(super) fn read(&self, value: &TypedExpr) -> Type {
        self.declared_type(value)
    }

    pub(super) fn changed(&self, value: &TypedExpr) -> Type {
        let declared = self.declared_type(value);
        if self.pair_part_behind_shared(value) {
            Type::shared(declared)
        } else {
            declared
        }
    }

    fn pair_part_behind_shared(&self, value: &TypedExpr) -> bool {
        matches!(&value.kind, TypedExprKind::Struct(TypedStructExpr::Field(owner, label))
            if matches!(label.spelling, FieldSpelling::Pair(_)) && self.behind_shared(owner))
    }

    fn declared_type(&self, value: &TypedExpr) -> Type {
        let TypedExprKind::Struct(TypedStructExpr::Field(owner, label)) = &value.kind else {
            return value.ty.clone();
        };
        let (peeled, _) = ReferencePeeling::new(self.unifier).peel(&owner.ty);
        let Type::Named(binding, arguments) = peeled else {
            return value.ty.clone();
        };
        let declared = match label.spelling {
            FieldSpelling::Pair(position) => position.part_of(&arguments).cloned(),
            _ => self
                .declarations
                .field_type(binding, &label.name, &arguments),
        };
        declared.unwrap_or_else(|| value.ty.clone())
    }

    fn behind_shared(&self, owner: &TypedExpr) -> bool {
        let peeling = ReferencePeeling::new(self.unifier);
        let mut current = owner;
        loop {
            if peeling.has_shared_layer(&self.declared_type(current)) {
                return true;
            }
            let TypedExprKind::Struct(TypedStructExpr::Field(inner, _)) = &current.kind else {
                return false;
            };
            current = inner;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PlaceTypes;
    use crate::core::{FieldLabel, FieldSpelling, Literal, PairPosition, PrimitiveType};
    use crate::source::{SourceId, Span};
    use crate::types::declared_types::DeclaredTypes;
    use crate::types::type_declarations::TypeDeclarations;
    use crate::types::typed::{TypedExpr, TypedExprKind, TypedLeaf, TypedStructExpr};
    use crate::types::unifier::Unifier;
    use crate::types::{Type, TypedProgram, TypedSource};

    struct PlaceTypesFixture {
        program: TypedProgram,
        types: TypeDeclarations,
        unifier: Unifier,
        span: Span,
    }

    impl PlaceTypesFixture {
        const SOURCE: &str = "
            (struct reading (value i64))
            (defn measure ((sample (ref reading))) (field sample value))
            (defn main () (println \"Leaf\"))";

        fn new() -> Self {
            let mut program = TypedSource::expand(Self::SOURCE).typed();
            let types = std::mem::take(program.declarations.types_mut());
            Self {
                program,
                types,
                unifier: Unifier::default(),
                span: Span::new(SourceId::new(0), 0..0),
            }
        }

        fn read(&self, place: &TypedExpr) -> Type {
            self.place_types().read(place)
        }

        fn place_types(&self) -> PlaceTypes<'_> {
            PlaceTypes::new(&self.unifier, DeclaredTypes::new(&self.types))
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

        fn owner(&self, ty: Type) -> TypedExpr {
            TypedExpr {
                kind: TypedExprKind::Leaf(TypedLeaf::Literal(Literal::Bool(false))),
                ty,
                span: self.span,
            }
        }

        fn field_of(&self, owner: Type, field: &str) -> TypedExpr {
            let label = FieldLabel {
                name: field.into(),
                span: self.span,
                spelling: FieldSpelling::Bare,
            };
            let kind =
                TypedExprKind::Struct(TypedStructExpr::Field(Box::new(self.owner(owner)), label));
            TypedExpr {
                kind,
                ty: Self::boolean(),
                span: self.span,
            }
        }

        fn first_parts(&self, borrow: fn(Type) -> Type, depth: usize) -> TypedExpr {
            let pair = self
                .types
                .rust_types
                .pair_binding()
                .expect("the prelude defines a pair");
            let mut nested = vec![Self::integer()];
            for level in 0..depth {
                nested.push(Type::Named(
                    pair,
                    vec![nested[level].clone(), Self::integer()],
                ));
            }
            let mut read = self.owner(borrow(nested[depth].clone()));
            for level in (0..depth).rev() {
                let label = FieldLabel {
                    name: "0".into(),
                    span: self.span,
                    spelling: FieldSpelling::Pair(PairPosition::First),
                };
                let ty = match level {
                    0 => Self::boolean(),
                    _ => Type::shared(nested[level].clone()),
                };
                let kind = TypedExprKind::Struct(TypedStructExpr::Field(Box::new(read), label));
                read = TypedExpr {
                    kind,
                    ty,
                    span: self.span,
                };
            }
            read
        }

        fn integer() -> Type {
            Type::Primitive(PrimitiveType::I64)
        }

        fn boolean() -> Type {
            Type::Primitive(PrimitiveType::Bool)
        }
    }

    #[test]
    fn a_field_through_a_borrow_has_its_declared_type() {
        let fixture = PlaceTypesFixture::new();
        let field = fixture.field_of(fixture.parameter("measure"), "value");
        assert_eq!(fixture.read(&field), PlaceTypesFixture::integer());
    }

    #[test]
    fn an_undeclared_field_keeps_its_inferred_type() {
        let fixture = PlaceTypesFixture::new();
        let field = fixture.field_of(fixture.parameter("measure"), "missing");
        assert_eq!(fixture.read(&field), PlaceTypesFixture::boolean());
    }

    #[test]
    fn a_field_of_a_non_struct_keeps_its_inferred_type() {
        let fixture = PlaceTypesFixture::new();
        let field = fixture.field_of(PlaceTypesFixture::integer(), "value");
        assert_eq!(fixture.read(&field), PlaceTypesFixture::boolean());
    }

    #[test]
    fn a_deeply_nested_pair_part_through_mut_is_a_place() {
        let fixture = PlaceTypesFixture::new();
        let part = fixture.first_parts(Type::mutable, 30);
        assert_eq!(fixture.read(&part), PlaceTypesFixture::integer());
        assert_eq!(
            fixture.place_types().changed(&part),
            PlaceTypesFixture::integer()
        );
    }

    #[test]
    fn a_deeply_nested_pair_part_through_ref_reads_but_cannot_change() {
        let fixture = PlaceTypesFixture::new();
        let part = fixture.first_parts(Type::shared, 30);
        assert_eq!(fixture.read(&part), PlaceTypesFixture::integer());
        let changed = fixture.place_types().changed(&part);
        assert_eq!(changed, Type::shared(PlaceTypesFixture::integer()));
    }
}
