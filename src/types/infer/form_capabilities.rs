use std::rc::Rc;

use crate::core::{BindingId, Expr, TypeExpr, VariantInfo};
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::checker::TypeChecker;
use crate::types::collection::Collection;
use crate::types::expectation::Expectation;
use crate::types::forms::{
    AnnotationLowering, ChildTyping, CollectionShapes, ExpectedChildTyping, FieldMoves, LocalScope,
    MacroOrigins, MapKeys, Obligations, OperandTyping, PlaceReads, StructBases, TypeFitting,
    TypeTextRecords, Unknowns, VariantSites,
};
use crate::types::ty::Type;
use crate::types::typed::TypedExpr;
use crate::types::variant_site::VariantSite;

impl ChildTyping for TypeChecker {
    fn type_child(&mut self, child: &Expr) -> Result<TypedExpr, Diagnostic> {
        self.infer(child)
    }
}

impl ExpectedChildTyping for TypeChecker {
    fn type_child_against(
        &mut self,
        child: &Expr,
        expectation: Expectation,
    ) -> Result<TypedExpr, Diagnostic> {
        self.infer_against(child, expectation.expected, expectation.context)
    }
}

impl TypeFitting for TypeChecker {
    fn fit(
        &mut self,
        expectation: Expectation,
        found: &Type,
        span: Span,
    ) -> Result<(), Diagnostic> {
        self.unification().expect(expectation, found, span)
    }
}

impl MacroOrigins for TypeChecker {
    fn macro_name(&self, span: Span) -> Option<Rc<str>> {
        TypeChecker::macro_name(self, span)
    }
}

impl PlaceReads for TypeChecker {
    fn read_type(&self, value: &TypedExpr) -> Type {
        self.place_types().read(value)
    }

    fn changed_type(&self, value: &TypedExpr) -> Type {
        self.place_types().changed(value)
    }

    fn field_place(&self, value: &TypedExpr) -> Option<String> {
        self.passing_facts().field_place(value)
    }
}

impl Unknowns for TypeChecker {
    fn fresh(&mut self) -> Type {
        self.unifier.fresh()
    }

    fn resolve(&self, value: &Type) -> Type {
        self.unifier.resolve(value)
    }
}

impl Obligations for TypeChecker {
    fn require_copy(&mut self, value: &Type) {
        TypeChecker::require_copy(self, value);
    }
}

impl VariantSites for TypeChecker {
    fn variant_site(&mut self, binding: BindingId, expected: Option<&Type>) -> VariantSite {
        TypeChecker::variant_site(self, binding, expected)
    }

    fn is_result_ok(&self, info: VariantInfo) -> bool {
        self.declarations
            .types()
            .enums
            .variant_shape(info)
            .is_result_ok()
    }
}

impl OperandTyping for TypeChecker {
    fn type_operand(&mut self, operand: &Expr) -> Result<TypedExpr, Diagnostic> {
        self.within_operand(|checker| checker.infer(operand))
    }
}

impl TypeTextRecords for TypeChecker {
    fn note_type_text(&mut self) {
        self.holds_infer_forms = true;
    }
}

impl AnnotationLowering for TypeChecker {
    fn lower_annotation(&mut self, annotation: &TypeExpr) -> Result<Type, Diagnostic> {
        self.written_types().lower_without_generics(annotation)
    }
}

impl LocalScope for TypeChecker {
    fn bind_local(&mut self, binding: BindingId, ty: Type) {
        self.locals.insert(binding, ty);
    }
}

impl CollectionShapes for TypeChecker {
    fn collection_of(&self, ty: &Type) -> Option<Collection> {
        Collection::of(ty, &self.declarations.types().rust_types)
    }

    fn nested_vector_of(&self, depth: usize, element: &Type) -> Type {
        self.prelude_types().nested_vector_of(depth, element)
    }
}

impl MapKeys for TypeChecker {
    fn require_map_key(&mut self, map: &Collection, span: Span) {
        self.check_map_key(map, span);
    }
}

impl StructBases for TypeChecker {
    fn explain_struct_base(&self, error: Diagnostic, base: &Type) -> Diagnostic {
        self.point_struct_base_to_fields(error, base)
    }
}

impl FieldMoves for TypeChecker {
    fn moved_base(&mut self, base: TypedExpr) -> TypedExpr {
        self.moved_field_base(base)
    }
}
