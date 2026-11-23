use super::field_trait_path::FieldTraitPath;
use super::fragment::{Code, Precedence};
use crate::core::{FieldLabel, ModuleId, ReferenceKind};
use crate::types::{Type, TypedExpr, TypedExprKind, TypedProgram, Unborrowed, WrittenBorrows};

pub(super) struct FieldGetter<'a> {
    program: &'a TypedProgram,
    module: ModuleId,
}

impl<'a> FieldGetter<'a> {
    pub(super) fn reads_generic_owner(owner: &Type) -> bool {
        matches!(Unborrowed::of(owner), Type::Generic(_))
    }

    pub(super) fn new(program: &'a TypedProgram, module: ModuleId) -> Self {
        Self { program, module }
    }

    pub(super) fn call(&self, owner: &TypedExpr, receiver: Code, label: &FieldLabel) -> Code {
        let getter = FieldTraitPath::new(self.program)
            .getter(&label.name, self.module)
            .expect("a read of a generic owner has a field trait");
        let receiver = if Self::is_reference(Self::receiver_of(owner)) {
            receiver
        } else {
            Self::borrowed(receiver)
        };
        Code::joined(
            vec![Code::atom(format!("{getter}(")), receiver, Code::atom(")")],
            Precedence::Atom,
        )
    }

    pub(super) fn receiver_of(owner: &TypedExpr) -> &TypedExpr {
        let owner = WrittenBorrows::without_dereferenced(owner);
        match &owner.kind {
            TypedExprKind::Deref(inner) | TypedExprKind::Borrow(ReferenceKind::Shared, inner) => {
                Self::receiver_of(inner)
            }
            _ => owner,
        }
    }

    fn is_reference(receiver: &TypedExpr) -> bool {
        matches!(receiver.ty, Type::Ref(..) | Type::Mut(..))
    }

    fn borrowed(receiver: Code) -> Code {
        let receiver = if matches!(
            receiver.precedence,
            Precedence::Operator | Precedence::Block
        ) {
            receiver.parenthesized()
        } else {
            receiver
        };
        Code::joined_outside_delimiters(vec![Code::atom("&"), receiver], Precedence::Prefix)
    }
}
