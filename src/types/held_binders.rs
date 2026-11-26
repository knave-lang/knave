use std::rc::Rc;

use rustc_hash::FxHashSet;

use super::given_holders::GivenHolders;
use super::trait_arguments::TraitArguments;
use super::ty::{Bound, GenericId, GenericParam, Scheme, Type};
use super::unheld_at::UnheldAt;
use super::unheld_binder::UnheldBinder;

pub(super) struct HeldBinders<'s, 'a> {
    scheme: &'s Scheme,
    arguments: &'s TraitArguments<'a>,
    held: FxHashSet<GenericId>,
}

impl<'s, 'a> HeldBinders<'s, 'a> {
    pub(super) fn of(
        scheme: &'s Scheme,
        arguments: &'s TraitArguments<'a>,
        given: &GivenHolders,
    ) -> Self {
        let held = (scheme.params.iter())
            .filter(|param| {
                Self::body_holds(scheme, param)
                    || (param.bounds.iter()).any(|bound| Some(bound.binding) == given.receiver)
                    || given.impl_variables.contains(&param.name)
            })
            .map(|param| param.id)
            .collect();
        let mut binders = Self {
            scheme,
            arguments,
            held,
        };
        binders.close_over_fixing_bounds();
        binders
    }

    pub(super) fn body_holds_every_binder(scheme: &Scheme) -> bool {
        (scheme.params.iter()).all(|param| Self::body_holds(scheme, param))
    }

    pub(super) fn unheld(&self) -> Vec<UnheldAt> {
        (self.scheme.params.iter().enumerate())
            .filter(|(_, binder)| !self.held.contains(&binder.id))
            .map(|(position, binder)| self.why_unheld(position, binder))
            .collect()
    }

    pub(super) fn held_scheme_without(&self, reported: &[UnheldAt]) -> Scheme {
        let dropped: Vec<GenericId> = (self.scheme.params.iter())
            .filter(|param| (reported.iter()).any(|unheld| unheld.binder.binder() == &param.name))
            .map(|param| param.id)
            .collect();
        let mut scheme = self.scheme.clone();
        scheme.params.retain(|param| !dropped.contains(&param.id));
        for param in &mut scheme.params {
            param.bounds.retain(|bound| {
                (bound.argument.iter())
                    .all(|argument| !dropped.iter().any(|generic| argument.mentions(*generic)))
            });
        }
        scheme
    }

    fn body_holds(scheme: &Scheme, param: &GenericParam) -> bool {
        scheme.body.mentions(param.id)
    }

    fn close_over_fixing_bounds(&mut self) {
        loop {
            let fixed: Vec<GenericId> = (self.scheme.params.iter())
                .filter(|param| self.held.contains(&param.id))
                .flat_map(|param| param.bounds.iter())
                .filter(|bound| self.fixes(bound))
                .flat_map(|bound| bound.argument.iter())
                .flat_map(|argument| self.params_in(argument))
                .map(|param| param.id)
                .filter(|generic| !self.held.contains(generic))
                .collect();
            if fixed.is_empty() {
                return;
            }
            self.held.extend(fixed);
        }
    }

    fn why_unheld(&self, position: usize, binder: &GenericParam) -> UnheldAt {
        if let Some((holder_position, holder, bound)) = self.bound_naming(binder) {
            let unheld = UnheldBinder::InArgument {
                holder: Rc::clone(&holder.name),
                binder: Rc::clone(&binder.name),
                bound: bound.binding,
            };
            return UnheldAt {
                holder_position,
                binder_position: position,
                holder_held: self.held.contains(&holder.id),
                binder: unheld,
            };
        }
        let unheld = match binder.bounds.first() {
            Some(bound) => UnheldBinder::Implementor {
                binder: Rc::clone(&binder.name),
                bound: bound.binding,
            },
            None => UnheldBinder::Bare {
                binder: Rc::clone(&binder.name),
            },
        };
        UnheldAt {
            holder_position: position,
            binder_position: position,
            holder_held: false,
            binder: unheld,
        }
    }

    fn bound_naming(&self, binder: &GenericParam) -> Option<(usize, &'s GenericParam, &'s Bound)> {
        let by_held_holder =
            (self.bounds_naming(binder)).find(|(_, holder, _)| self.held.contains(&holder.id));
        by_held_holder.or_else(|| self.bounds_naming(binder).next())
    }

    fn bounds_naming(
        &self,
        binder: &GenericParam,
    ) -> impl Iterator<Item = (usize, &'s GenericParam, &'s Bound)> {
        let binder = binder.id;
        (self.scheme.params.iter().enumerate())
            .filter(move |(_, holder)| holder.id != binder)
            .flat_map(|(position, holder)| {
                (holder.bounds.iter()).map(move |bound| (position, holder, bound))
            })
            .filter(move |(_, _, bound)| {
                !self.fixes(bound)
                    && (bound.argument.iter()).any(|argument| argument.mentions(binder))
            })
    }

    fn fixes(&self, bound: &Bound) -> bool {
        self.arguments.fixes(bound.binding)
    }

    fn params_in(&self, argument: &'s Type) -> impl Iterator<Item = &'s GenericParam> {
        (self.scheme.params.iter()).filter(move |param| argument.mentions(param.id))
    }
}
