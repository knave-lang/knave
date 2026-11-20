use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::field_reader::FieldReader;
use super::field_traits::FieldTraits;
use super::ty::{Bound, GenericId, Type};
use super::typed::{TypedDefinition, TypedExpr, TypedExprKind, TypedProgram, TypedStructExpr};
use super::unborrowed::Unborrowed;
use crate::core::{BindingId, CopyTrait};

type Settled = FxHashMap<GenericId, Type>;

#[derive(Default)]
pub(super) struct ReadSettlements {
    reads: Settled,
    fixed: FxHashMap<(BindingId, GenericId), Type>,
    copied: Vec<(BindingId, GenericId)>,
}

impl ReadSettlements {
    pub(super) fn settle(&mut self, reader: &FieldReader, read: Type) {
        match reader.read {
            Type::Generic(open) => self.reads.insert(open, read),
            _ => self.fixed.insert((reader.binding, reader.owner), read),
        };
    }

    pub(super) fn require_copy(&mut self, reader: &FieldReader) {
        self.copied.push((reader.binding, reader.owner));
    }

    pub(super) fn apply_to(&self, program: &mut TypedProgram) {
        let copy = program
            .declarations
            .traits()
            .table()
            .with_paths(&[CopyTrait::PATH]);
        let copy_bound = copy.first().map(|binding| Bound::plain(*binding));
        let field_traits = program.declarations.traits().rust_paths().field_traits();
        for (id, param) in &mut program.generics {
            for bound in &mut param.bounds {
                *bound = bound.substituted(&self.reads);
            }
            Self::fix_reads(field_traits, &mut param.bounds, self.fixed_for(*id));
            if self.copied.iter().any(|(_, owner)| owner == id) {
                param.bounds.extend(copy_bound.clone());
            }
        }
        let definitions = (program.modules.iter_mut()).flat_map(|module| &mut module.definitions);
        for definition in definitions {
            self.settle_definition(field_traits, definition, copy_bound.as_ref());
        }
    }

    fn fixed_for(&self, owner: GenericId) -> Option<&Type> {
        (self.fixed.iter())
            .find_map(|((_, fixed_owner), read)| (*fixed_owner == owner).then_some(read))
    }

    fn fix_reads(field_traits: &FieldTraits, bounds: &mut [Bound], fixed: Option<&Type>) {
        let Some(read) = fixed else {
            return;
        };
        let field_bounds = bounds
            .iter_mut()
            .filter(|bound| field_traits.field_of(bound.binding).is_some());
        for bound in field_bounds {
            bound.argument = Some(read.clone());
        }
    }

    fn settle_definition(
        &self,
        field_traits: &FieldTraits,
        definition: &mut TypedDefinition,
        copy_bound: Option<&Bound>,
    ) {
        let scheme = &mut definition.scheme;
        scheme
            .params
            .retain(|param| !self.reads.contains_key(&param.id));
        for param in &mut scheme.params {
            for bound in &mut param.bounds {
                *bound = bound.substituted(&self.reads);
            }
            let fixed = self.fixed.get(&(definition.binding, param.id));
            Self::fix_reads(field_traits, &mut param.bounds, fixed);
            if self.copied.contains(&(definition.binding, param.id)) {
                param.bounds.extend(copy_bound.cloned());
            }
        }
        scheme.body = scheme.body.substitute(&self.reads);
        let fixed_reads: Vec<(Type, Type)> = (self.fixed.iter())
            .filter(|((binding, _), _)| *binding == definition.binding)
            .map(|((_, owner), read)| (Type::Generic(*owner), read.clone()))
            .collect();
        self.settle_expr(&mut definition.value, &fixed_reads);
    }

    fn settle_expr(&self, expr: &mut TypedExpr, fixed_reads: &[(Type, Type)]) {
        expr.ty = match &expr.kind {
            TypedExprKind::Struct(TypedStructExpr::Field(value, _)) => (fixed_reads.iter())
                .find(|(owner, _)| Unborrowed::of(&value.ty) == owner)
                .map_or_else(|| expr.ty.substitute(&self.reads), |(_, read)| read.clone()),
            _ => expr.ty.substitute(&self.reads),
        };
        if let TypedExprKind::Fn(lambda) = &mut expr.kind {
            for param in &mut Rc::make_mut(lambda).params {
                param.ty = param.ty.substitute(&self.reads);
            }
        }
        for child in expr.children_mut() {
            self.settle_expr(child, fixed_reads);
        }
    }
}
