use crate::core::{BindingId, TypeExpr, TypeExprKind, VariantDeclaration, VariantPayload};
use crate::source::Span;

pub(super) struct ParameterUsage {
    enum_binding: BindingId,
    variable: BindingId,
    outside_self: bool,
    first_recursive: Option<Span>,
}

#[derive(Clone, Copy)]
enum Place {
    OutsideSelf,
    InsideSelf,
}

impl ParameterUsage {
    pub(super) fn scan(
        variants: &[VariantDeclaration],
        enum_binding: BindingId,
        variable: BindingId,
    ) -> Self {
        let mut usage = Self {
            enum_binding,
            variable,
            outside_self: false,
            first_recursive: None,
        };
        for variant in variants {
            usage.visit_payload(&variant.payload, Place::OutsideSelf);
        }
        usage
    }

    pub(super) fn used_outside_self(&self) -> bool {
        self.outside_self
    }

    pub(super) fn first_recursive_use(&self) -> Option<Span> {
        self.first_recursive
    }

    fn visit_payload(&mut self, payload: &VariantPayload, place: Place) {
        match payload {
            VariantPayload::None => {}
            VariantPayload::Positional(types) => {
                for type_expr in types {
                    self.visit(type_expr, place);
                }
            }
            VariantPayload::Named(fields) => {
                for (_, type_expr) in fields {
                    self.visit(type_expr, place);
                }
            }
        }
    }

    fn visit(&mut self, type_expr: &TypeExpr, place: Place) {
        match type_expr.kind() {
            TypeExprKind::Named(..) => self.visit_named(type_expr, place),
            TypeExprKind::Function(_, parameters, result) => {
                for parameter in parameters {
                    self.visit(parameter, place);
                }
                self.visit(result, place);
            }
            TypeExprKind::Reference(_, _, inner) => {
                self.visit(inner, place);
            }
            TypeExprKind::Primitive(_) | TypeExprKind::Hole => {}
            TypeExprKind::Where(..) | TypeExprKind::With(..) | TypeExprKind::Trait(_) => {}
        }
    }

    fn visit_named(&mut self, type_expr: &TypeExpr, place: Place) {
        let TypeExprKind::Named(binding, arguments) = type_expr.kind() else {
            return;
        };
        let place = self.place_inside(*binding, place);
        if *binding == self.variable {
            self.record_use(type_expr.span(), place);
        }
        for argument in arguments {
            self.visit(argument, place);
        }
    }

    fn place_inside(&self, binding: BindingId, place: Place) -> Place {
        if binding == self.enum_binding {
            return Place::InsideSelf;
        }
        place
    }

    fn record_use(&mut self, span: Span, place: Place) {
        match place {
            Place::InsideSelf => {
                self.first_recursive.get_or_insert(span);
            }
            Place::OutsideSelf => self.outside_self = true,
        }
    }
}
