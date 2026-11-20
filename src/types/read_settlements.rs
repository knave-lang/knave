use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::field_reader::FieldReader;
use super::field_traits::FieldTraits;
use super::ty::{Bound, GenericId, Type};
use super::typed::{TypedDefinition, TypedExpr, TypedExprKind, TypedProgram};
use crate::core::{BindingId, CopyTrait};

type Settled = FxHashMap<GenericId, Type>;

#[derive(Default)]
pub(super) struct ReadSettlements {
    reads: Settled,
    fixed: Settled,
    copied: FxHashSet<GenericId>,
    settled: FxHashSet<BindingId>,
}

impl ReadSettlements {
    pub(super) fn settle(&mut self, reader: &FieldReader, read: Type) {
        self.settled.insert(reader.binding);
        match reader.read {
            Type::Generic(open) => self.reads.insert(open, read),
            _ => self.fixed.insert(reader.owner, read),
        };
    }

    pub(super) fn require_copy(&mut self, reader: &FieldReader) {
        self.copied.insert(reader.owner);
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
            Self::fix_reads(field_traits, &mut param.bounds, self.fixed.get(id));
            if self.copied.contains(id) {
                param.bounds.extend(copy_bound.clone());
            }
        }
        let definitions = (program.modules.iter_mut()).flat_map(|module| &mut module.definitions);
        let readers = definitions.filter(|definition| self.settled.contains(&definition.binding));
        for definition in readers {
            self.settle_definition(field_traits, definition, copy_bound.as_ref());
        }
    }

    fn fix_reads(field_traits: &FieldTraits, bounds: &mut [Bound], fixed: Option<&Type>) {
        let Some(read) = fixed else {
            return;
        };
        let field_bounds = bounds
            .iter_mut()
            .filter(|bound| field_traits.bindings().field_of(bound.binding).is_some());
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
            Self::fix_reads(field_traits, &mut param.bounds, self.fixed.get(&param.id));
            if self.copied.contains(&param.id) {
                param.bounds.extend(copy_bound.cloned());
            }
        }
        scheme.body = scheme.body.substitute(&self.reads);
        self.settle_expr(&mut definition.value);
    }

    fn settle_expr(&self, expr: &mut TypedExpr) {
        expr.ty = expr.ty.substitute(&self.reads);
        if let TypedExprKind::Fn(lambda) = &mut expr.kind {
            for param in &mut Rc::make_mut(lambda).params {
                param.ty = param.ty.substitute(&self.reads);
            }
        }
        for child in expr.children_mut() {
            self.settle_expr(child);
        }
    }
}
