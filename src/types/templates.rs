use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::inline_templates::InlineTemplates;
use super::pipelines::Pipelines;
use super::registered_template::RegisteredTemplate;
use super::rust_types::RustTypeTable;
use super::template_author::TemplateAuthor;
use super::template_ties::TemplateTies;
use super::ty::Scheme;
use super::typed::{TypedExpr, TypedExprKind, TypedLeaf};
use crate::core::{BindingId, Conversion, ParsedTemplate, Piece};

pub type Conversions = Rc<[(usize, Conversion)]>;

pub enum TemplateName<'a> {
    Declared(BindingId, &'a Rc<str>, TemplateAuthor),
    Inline(&'a Rc<str>),
}

impl TemplateName<'_> {
    fn author(&self) -> TemplateAuthor {
        match self {
            Self::Declared(_, _, author) => *author,
            Self::Inline(_) => TemplateAuthor::User,
        }
    }
}

#[derive(Default)]
pub struct TemplateBorrows {
    declared: FxHashMap<BindingId, RegisteredTemplate>,
    conversions: FxHashMap<BindingId, Conversions>,
    inline: InlineTemplates,
}

impl From<InlineTemplates> for TemplateBorrows {
    fn from(inline: InlineTemplates) -> Self {
        Self {
            inline,
            ..Self::default()
        }
    }
}

impl TemplateBorrows {
    pub fn register(
        &mut self,
        name: TemplateName<'_>,
        scheme: &Scheme,
        rust_types: &RustTypeTable,
    ) {
        let ties = TemplateTies::of(scheme, Pipelines::new(rust_types), name.author());
        match name {
            TemplateName::Declared(binding, text, _) => {
                let registered = RegisteredTemplate::declared(text, ties);
                let conversions =
                    registered.read(None, |parsed, _| Self::written_conversions(parsed));
                if let Some(conversions) = conversions {
                    self.conversions.insert(binding, conversions);
                }
                self.declared.insert(binding, registered);
            }
            TemplateName::Inline(text) => self.inline.register(text, scheme, ties),
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
        self.read_template(callee, None, |parsed, _| Self::written_conversions(parsed))
    }

    fn written_conversions(parsed: &ParsedTemplate) -> Option<Conversions> {
        let mut conversions = Piece::conversions_in(parsed.pieces()).peekable();
        conversions.peek().is_some().then(|| conversions.collect())
    }

    #[cfg(test)]
    pub fn registered(&self) -> impl Iterator<Item = (&BindingId, &RegisteredTemplate)> {
        self.declared.iter()
    }

    #[cfg(test)]
    pub fn inline(&self) -> &InlineTemplates {
        &self.inline
    }

    pub fn borrows(&self, callee: &TypedExpr, position: usize) -> bool {
        self.read_template(callee, false, |parsed, _| parsed.borrows(position))
    }

    pub fn is_template(&self, callee: &TypedExpr) -> bool {
        self.read_template(callee, false, |_, _| true)
    }

    pub fn receives(&self, callee: &TypedExpr, position: usize) -> bool {
        self.read_template(callee, false, |parsed, _| parsed.receives(position))
    }

    pub fn inline_scheme(&self, text: &str) -> Option<&Scheme> {
        self.inline.scheme(text)
    }

    pub fn read_template<T>(
        &self,
        callee: &TypedExpr,
        absent: T,
        read: impl FnOnce(&ParsedTemplate, Option<&TemplateTies>) -> T,
    ) -> T {
        match &callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => match self.declared.get(binding) {
                Some(template) => template.read(absent, read),
                None => absent,
            },
            TypedExprKind::Leaf(TypedLeaf::Rust(template)) => self
                .inline
                .read(template, |registered| registered.read(absent, read)),
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

    #[test]
    fn every_template_a_prelude_macro_builds_settles_its_ties() {
        let program = TypedSource::expand(
            "(defn main ()
               (local s (str \"a\"))
               (local empty (the (vec i64) (vec)))
               (local one (vec (as-str s)))
               (local two (vec 1 2))
               (local hashed (hash-map (as-str s) 1))
               (local sorted (sorted-map 1 (as-str s)))
               (local none (the (hash-map i64 i64) (hash-map)))
               (local shown (str \"{}\" s))
               (println \"{} {} {}\" (len empty) (len one) (len two))
               (println (+ (len hashed) (len sorted) (len none)) shown))",
        )
        .typed();
        let written: Vec<_> = program.template_borrows.inline().written().collect();
        let unsettled: Vec<&str> = written
            .iter()
            .filter(|(_, template)| template.ties().is_none())
            .map(|(text, _)| text.as_ref())
            .collect();
        assert!(written.len() >= 8);
        assert_eq!(unsettled, Vec::<&str>::new());
    }
}
