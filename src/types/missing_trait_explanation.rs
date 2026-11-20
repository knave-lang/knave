use super::bound_origin_problems::BoundOriginProblems;
use super::bound_origins::BoundOrigin;
use super::declared_traits::DeclaredTraits;
use super::declared_types::DeclaredTypes;
use super::derivable_traits::DerivableTraits;
use super::enum_trait_help::EnumTraitHelp;
use super::explanation_sources::ExplanationSources;
use super::field_read_mismatch::FieldReadMismatch;
use super::implementor_listing::ImplementorListing;
use super::missing_trait::MissingTrait;
use super::problems::TypeProblems;
use super::struct_trait_help::StructTraitHelp;
use super::trait_boxing::TraitBoxing;
use super::trait_help::TraitHelp;
use super::trait_help_visibility::TraitHelpVisibility;
use super::ty::{Bound, Type};
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;

pub(super) struct MissingTraitExplanation<'a> {
    sources: ExplanationSources<'a>,
}

impl<'a> MissingTraitExplanation<'a> {
    pub(super) fn new(sources: ExplanationSources<'a>) -> Self {
        Self { sources }
    }

    pub(super) fn explain(&self, missing: MissingTrait<'_>) -> Diagnostic {
        if let Some(mismatch) = FieldReadMismatch::new(&self.sources).explain(&missing) {
            return mismatch;
        }
        let inclusions = self.sources.declarations.traits().table.inclusions();
        let generic = inclusions.generic_bound(missing.bound);
        let missing = MissingTrait {
            bound: generic.as_ref().unwrap_or(missing.bound),
            ..missing
        };
        let (bound, implementor) = (missing.bound, missing.implementor);
        let mut diagnostic = self.with_including_help(self.diagnostic(missing), bound, implementor);
        if let Type::Named(binding, _) = implementor {
            diagnostic = self.type_trait_help(diagnostic, *binding, bound);
            diagnostic = self.with_replacing_impl(diagnostic, *binding, bound.binding);
        }
        BoundOriginProblems::annotate(diagnostic, &bound.origins)
    }

    fn with_including_help(
        &self,
        diagnostic: Diagnostic,
        bound: &Bound,
        implementor: &Type,
    ) -> Diagnostic {
        let table = &self.sources.declarations.traits().table;
        let implemented = |including: &BindingId| {
            table.satisfies(&Bound::plain(*including), implementor, &|_, _| false)
        };
        let help = bound
            .origins
            .as_slice()
            .iter()
            .filter_map(BoundOrigin::including_trait)
            .filter(implemented)
            .find_map(|including| table.get(including)?.help.clone());
        match help {
            Some(help) => diagnostic.with_help(&*help),
            None => diagnostic,
        }
    }

    fn diagnostic(&self, missing: MissingTrait<'_>) -> Diagnostic {
        let MissingTrait {
            bound,
            implementor,
            span,
            shown_argument,
        } = missing;
        let display = &self.sources.display;
        let needed = display.needed(bound.binding, shown_argument, implementor);
        let found = display.show(implementor);
        let types = self.sources.declarations.types();
        let program_type = DeclaredTypes::new(types).declares_type(implementor);
        let diagnostic = match self.unimplemented_box(implementor, bound) {
            Some(boxed) => TypeProblems::missing_trait_on_box(&needed, &display.show(&boxed), span),
            None => match implementor {
                Type::Named(..) if program_type => {
                    TypeProblems::missing_trait_on_struct(&needed, &found, span)
                }
                _ => {
                    let listing = ImplementorListing::new(&self.sources);
                    TypeProblems::missing_trait(&needed, &found, span, listing.list(bound.binding))
                }
            },
        };
        let visibility = TraitHelpVisibility::new(self.sources.declarations);
        if !visibility.shows(bound.binding, implementor) {
            return diagnostic;
        }
        let known = self.sources.declarations.traits().table.get(bound.binding);
        match known.and_then(|known| known.help.as_deref()) {
            Some(help) => diagnostic.with_help(help),
            None => diagnostic,
        }
    }

    fn unimplemented_box(&self, implementor: &Type, bound: &Bound) -> Option<Type> {
        let mut found = implementor;
        while let Type::Ref(_, inner) | Type::Mut(_, inner) = found {
            found = inner;
        }
        let Type::Named(_, arguments) = found else {
            return None;
        };
        let declarations = self.sources.declarations;
        let argument = (arguments.first()).filter(|_| declarations.types().is_box(found))?;
        let traits = declarations.traits();
        let boxable = TraitBoxing::new(traits, self.sources.bindings).suggests_box(bound.binding);
        (boxable && traits.table.satisfies(bound, argument, &|_, _| false)).then(|| found.clone())
    }

    fn type_trait_help(
        &self,
        diagnostic: Diagnostic,
        binding: BindingId,
        bound: &Bound,
    ) -> Diagnostic {
        if let Some(help) = EnumTraitHelp::of(binding, bound, &self.sources) {
            return help.add_to(diagnostic);
        }
        match StructTraitHelp::of(binding, bound, &self.sources) {
            Some(help) => help.add_to(diagnostic),
            None => diagnostic,
        }
    }

    fn with_replacing_impl(
        &self,
        diagnostic: Diagnostic,
        head: BindingId,
        trait_binding: BindingId,
    ) -> Diagnostic {
        let traits = DeclaredTraits::new(self.sources.declarations.traits());
        let replacing = DerivableTraits::new(self.sources.declarations.traits())
            .is_structural(trait_binding)
            .then(|| traits.written_impl(trait_binding, head))
            .flatten();
        let Some(replacing) = replacing else {
            return diagnostic;
        };
        let bindings = self.sources.bindings;
        let trait_name = bindings.get(trait_binding).name();
        let head_name = bindings.get(head).name();
        diagnostic.with_label(TypeProblems::replacing_impl(
            replacing.implementor_span,
            trait_name,
            head_name,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::MissingTraitExplanation;
    use crate::core::PrimitiveType;
    use crate::diagnostic::Diagnostic;
    use crate::source::{SourceId, Span};
    use crate::types::Type;
    use crate::types::bound_origins::BoundOrigin;
    use crate::types::declarations_fixture::DeclarationsFixture;
    use crate::types::missing_trait::MissingTrait;
    use crate::types::ty::Bound;

    const SOURCE: &str = "
        (defstruct score :points i64)
        (trait shown (show (fn ((ref self)) usize)))
        (impl shown string (show (text) 1))
        (trait shape (area (fn ((ref self)) f64)))
        (struct circle (radius f64))
        (impl shape circle (area (shape) (:radius shape)))
        (enum level (low) (high))
        (derive level debug)
        (defn main () (do))";

    struct ExplanationFixture {
        declarations: DeclarationsFixture,
    }

    impl ExplanationFixture {
        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(SOURCE),
            }
        }

        fn explain(&self, bound: &Bound, implementor: &Type) -> Diagnostic {
            let sources = self.declarations.explanation_sources();
            MissingTraitExplanation::new(sources).explain(MissingTrait {
                bound,
                implementor,
                span: Self::span(),
                shown_argument: None,
            })
        }

        fn shown(&self) -> Bound {
            Bound::plain(self.declarations.trait_named("shown"))
        }

        fn score(&self) -> Type {
            Type::Named(self.declarations.type_named("score"), Vec::new())
        }

        fn span() -> Span {
            Span::new(SourceId::new(0), 0..0)
        }
    }

    #[test]
    fn a_prelude_type_lists_the_implementors() {
        let fixture = ExplanationFixture::new();
        let boolean = Type::Primitive(PrimitiveType::Bool);
        let diagnostic = fixture.explain(&fixture.shown(), &boolean);
        assert_eq!(diagnostic.title(), "`bool` does not implement `shown`");
        assert_eq!(diagnostic.notes(), ["`shown` is implemented for: string"]);
    }

    #[test]
    fn a_program_type_lists_no_implementors() {
        let fixture = ExplanationFixture::new();
        let diagnostic = fixture.explain(&fixture.shown(), &fixture.score());
        assert_eq!(diagnostic.title(), "`score` does not implement `shown`");
        assert!(diagnostic.notes().is_empty());
    }

    #[test]
    fn a_borrowed_box_of_an_implementor_suggests_boxing_the_trait() {
        let fixture = ExplanationFixture::new();
        let declarations = &fixture.declarations;
        let circle = Type::Named(declarations.type_named("circle"), Vec::new());
        let boxed = Type::Named(declarations.rust_type_named("box"), vec![circle]);
        let shape = Bound::plain(declarations.trait_named("shape"));
        let diagnostic = fixture.explain(&shape, &Type::mutable(boxed));
        let helps: Vec<&str> = diagnostic
            .helps()
            .iter()
            .map(|help| help.message())
            .collect();
        assert_eq!(
            helps,
            [
                "write the type `(box shape)` where the box is made, such as `(let ((shapes (vec (box shape))) (vec)) ...)`"
            ]
        );
    }

    #[test]
    fn an_enum_gets_the_enum_help() {
        let fixture = ExplanationFixture::new();
        let declarations = &fixture.declarations;
        let level = Type::Named(declarations.type_named("level"), Vec::new());
        let clone = Bound::plain(declarations.trait_named("clone"));
        let diagnostic = fixture.explain(&clone, &level);
        let helps: Vec<&str> = diagnostic
            .helps()
            .iter()
            .map(|help| help.message())
            .collect();
        assert_eq!(helps, ["add `clone` to the enum's `derive`"]);
    }

    #[test]
    fn a_bound_from_a_copied_borrow_labels_the_copy_and_suggests_a_clone() {
        let fixture = ExplanationFixture::new();
        let mut bound = fixture.shown();
        bound
            .origins
            .add(BoundOrigin::BorrowCopied(ExplanationFixture::span()));
        let diagnostic = fixture.explain(&bound, &fixture.score());
        let labels: Vec<&str> = diagnostic.labels().map(|label| label.message()).collect();
        assert_eq!(
            labels,
            ["this needs `shown`", "this copies the borrowed value"]
        );
        let helps: Vec<&str> = diagnostic
            .helps()
            .iter()
            .map(|help| help.message())
            .collect();
        assert_eq!(
            helps,
            ["to copy a value that is not `copy`, write `(clone ...)` around it"]
        );
    }
}
