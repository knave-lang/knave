use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::registered_template::RegisteredTemplate;
use super::ty::Scheme;
use super::typed::{TypedExpr, TypedExprKind, TypedLeaf};
use crate::core::{BindingId, Conversion, ParsedTemplate, Piece, ReferenceKind};

pub type Conversions = Rc<[(usize, Conversion)]>;

#[derive(Default)]
pub struct TemplateBorrows {
    templates: FxHashMap<BindingId, RegisteredTemplate>,
    conversions: FxHashMap<BindingId, Conversions>,
    inline: RefCell<FxHashMap<Rc<str>, Option<RegisteredTemplate>>>,
}

impl TemplateBorrows {
    pub fn register(&mut self, binding: BindingId, template: &str, scheme: &Scheme) {
        if let Some(registered) = RegisteredTemplate::parse(template, Some(scheme)) {
            if let Some(conversions) = Self::written_conversions(registered.parsed()) {
                self.conversions.insert(binding, conversions);
            }
            self.templates.insert(binding, registered);
        }
    }

    pub fn conversions(&self, callee: &TypedExpr) -> Option<Conversions> {
        match &callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => {
                self.conversions.get(binding).cloned()
            }
            TypedExprKind::Leaf(TypedLeaf::Rust(template)) => {
                self.inline_conversions(callee, &template.template)
            }
            _ => None,
        }
    }

    fn inline_conversions(&self, callee: &TypedExpr, template: &str) -> Option<Conversions> {
        if !Conversion::may_be_written_in(template) {
            return None;
        }
        self.read_template(callee, None, |registered| {
            Self::written_conversions(registered.parsed())
        })
    }

    fn written_conversions(parsed: &ParsedTemplate) -> Option<Conversions> {
        let mut conversions = Piece::conversions_in(parsed.pieces()).peekable();
        conversions.peek().is_some().then(|| conversions.collect())
    }

    #[cfg(test)]
    pub fn registered(&self) -> impl Iterator<Item = (&BindingId, &RegisteredTemplate)> {
        self.templates.iter()
    }

    pub fn borrows(&self, callee: &TypedExpr, position: usize) -> bool {
        self.read_template(callee, false, |template| {
            template.parsed().borrows(position)
        })
    }

    pub fn is_template(&self, callee: &TypedExpr) -> bool {
        self.read_template(callee, false, |_| true)
    }

    pub fn receives(&self, callee: &TypedExpr, position: usize) -> bool {
        self.read_template(callee, false, |template| {
            template.parsed().receives(position)
        })
    }

    pub fn reference(&self, callee: &TypedExpr, position: usize) -> Option<ReferenceKind> {
        self.borrows(callee, position)
            .then_some(ReferenceKind::Shared)
    }

    pub fn read_template<T>(
        &self,
        callee: &TypedExpr,
        absent: T,
        read: impl Fn(&RegisteredTemplate) -> T,
    ) -> T {
        match &callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => {
                self.templates.get(binding).map(read).unwrap_or(absent)
            }
            TypedExprKind::Leaf(TypedLeaf::Rust(template)) => {
                let mut inline = self.inline.borrow_mut();
                let parsed = inline
                    .entry(template.template.clone())
                    .or_insert_with(|| RegisteredTemplate::parse(&template.template, None));
                parsed.as_ref().map(read).unwrap_or(absent)
            }
            _ => absent,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{ExprKind, Leaf};
    use crate::types::{SyntheticTemplate, TypedSource};

    struct PreludeTemplates;

    impl PreludeTemplates {
        fn unsettled() -> (usize, usize, Vec<String>) {
            let mut source = TypedSource::expand("(defn main () (println 1))");
            let templates: Vec<_> = source
                .program()
                .modules
                .iter()
                .flat_map(|module| module.definitions())
                .filter(|(_, value)| matches!(value.kind(), ExprKind::Leaf(Leaf::Rust(_))))
                .map(|(binding, _)| binding)
                .collect();
            for binding in &templates {
                source.check_also(*binding);
            }
            let program = source.typed();
            let mut unsettled: Vec<String> = program
                .template_borrows
                .registered()
                .filter(|(_, template)| template.ties().is_none())
                .map(|(binding, _)| program.bindings.get(*binding).name().to_string())
                .collect();
            unsettled.sort_unstable();
            let registered = program.template_borrows.registered().count();
            (templates.len(), registered, unsettled)
        }
    }

    #[test]
    fn every_prelude_template_and_synthetic_template_settles_its_ties() {
        let (defined, registered, unsettled) = PreludeTemplates::unsettled();
        assert_eq!(registered, defined);
        assert_eq!(unsettled, Vec::<String>::new());
        assert!(
            SyntheticTemplate::ALL
                .into_iter()
                .all(|template| SyntheticTemplate::named(template.text()).is_some())
        );
    }
}
