use std::rc::Rc;

use super::assumed_collections::CollectionAssumption;
use super::copy::CopyRules;
use super::pair_position::PairPosition;
use super::problems::TypeProblems;
use super::rust_types::RustTypeTable;
use super::ty::Type;
use super::typed::{TypedExpr, TypedExprKind, TypedStructExpr};
use crate::core::{Expr, FieldLabel, FieldSpelling};
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub struct PositionalRead {
    position: PairPosition,
    span: Span,
}

pub struct PairPart {
    part: Type,
    label: FieldLabel,
    owned: bool,
}

impl PairPart {
    pub fn read(self, pair: TypedExpr, copy: &CopyRules) -> TypedExpr {
        let span = self.label.span;
        let ty = match self.owned || copy.is_small_copy(&self.part) {
            true => self.part,
            false => Type::shared(self.part),
        };
        let kind = TypedExprKind::Struct(TypedStructExpr::Field(Box::new(pair), self.label));
        TypedExpr { kind, ty, span }
    }
}

impl PositionalRead {
    pub fn first(span: Span) -> Self {
        let position = PairPosition::First;
        Self { position, span }
    }

    pub fn second(span: Span) -> Self {
        let position = PairPosition::Second;
        Self { position, span }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn slice_assumption(&self) -> CollectionAssumption {
        self.position.slice_assumption()
    }

    pub fn reader<'e>(&self, arguments: &'e [Expr]) -> Result<&'e Expr, Diagnostic> {
        match arguments {
            [reader] => Ok(reader),
            _ => Err(TypeProblems::wrong_argument_count(
                1,
                arguments.len(),
                self.span,
            )),
        }
    }

    pub fn pair_part(&self, value: &Type, types: &RustTypeTable) -> Option<PairPart> {
        let mut referent = value;
        while let Type::Ref(_, inner) | Type::Mut(_, inner) = referent {
            referent = inner;
        }
        match referent {
            Type::Named(pair, parts) if Some(*pair) == types.pair_binding() => Some(PairPart {
                part: parts[self.position.index()].clone(),
                label: self.label(),
                owned: std::ptr::eq(referent, value),
            }),
            _ => None,
        }
    }

    fn label(&self) -> FieldLabel {
        FieldLabel {
            name: Rc::from(self.position.field_name()),
            span: self.span,
            spelling: FieldSpelling::Pair(self.position.operation()),
        }
    }
}
