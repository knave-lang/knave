use super::declared_trait::DeclaredTrait;
use super::problems::ExpandProblems;
use super::structural_path_claims::{StructuralPathClaim, StructuralPathClaims};
use crate::core::{
    BindingArena, BindingId, ReferenceKind, StructDerives, StructuralClause, StructuralKey,
    StructuralTemplate, TypeExpr, TypeExprKind,
};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::reader::{Form, FormKind};

pub(super) struct StructuralTraits<'t> {
    bindings: &'t BindingArena,
    claims: &'t mut StructuralPathClaims,
    problems: &'t mut Diagnostics,
}

struct ClauseReading<'f> {
    clause: &'f Form,
    parameters: usize,
    templates: Vec<(StructuralKey, StructuralTemplate, &'f Form)>,
    problems: Vec<Diagnostic>,
}

impl<'t> StructuralTraits<'t> {
    pub(super) fn new(
        bindings: &'t BindingArena,
        claims: &'t mut StructuralPathClaims,
        problems: &'t mut Diagnostics,
    ) -> Self {
        Self {
            bindings,
            claims,
            problems,
        }
    }

    pub(super) fn lower(
        &mut self,
        clause: &Form,
        declared: &DeclaredTrait,
    ) -> Option<StructuralClause> {
        self.judged(clause, declared)
            .map_err(|problems| {
                problems
                    .into_iter()
                    .for_each(|problem| self.problems.push(problem))
            })
            .ok()
    }

    fn judged(
        &mut self,
        clause: &Form,
        declared: &DeclaredTrait,
    ) -> Result<StructuralClause, Vec<Diagnostic>> {
        let structural = self
            .trait_problem(clause, declared)
            .map_or_else(|| ClauseReading::read(clause, declared), Err)?;
        self.claim_path(clause, declared)?;
        Ok(structural)
    }

    fn claim_path(
        &mut self,
        clause: &Form,
        declared: &DeclaredTrait,
    ) -> Result<(), Vec<Diagnostic>> {
        let claim = StructuralPathClaim::new(declared.binding, clause.span());
        self.claims.claim(declared.path, claim).map_err(|first| {
            vec![ExpandProblems::second_structural_path(
                declared.definer,
                self.name(declared.binding),
                declared.path,
                self.name(first.owner()),
                clause.span(),
                first.clause(),
            )]
        })
    }

    fn trait_problem(&self, clause: &Form, declared: &DeclaredTrait) -> Option<Vec<Diagnostic>> {
        let name = self.name(declared.binding);
        let problem = if !Self::has_one_self_method(declared) {
            ExpandProblems::structural_clause_needs_one_method(declared.definer, clause.span())
        } else if Self::derive_writes(declared.path) {
            ExpandProblems::structural_clause_on_derived_trait(
                declared.definer,
                name,
                declared.path,
                clause.span(),
            )
        } else if let Some(existing) = declared.path_owner {
            ExpandProblems::structural_clause_on_bound_path(
                declared.definer,
                name,
                declared.path,
                self.name(existing),
                clause.span(),
            )
        } else {
            return None;
        };
        Some(vec![problem])
    }

    fn name(&self, binding: BindingId) -> &str {
        self.bindings.get(binding).name()
    }

    fn has_one_self_method(declared: &DeclaredTrait) -> bool {
        let [method] = declared.methods else {
            return false;
        };
        let TypeExprKind::Function(_, parameters, _) = method.signature.kind() else {
            return false;
        };
        parameters
            .first()
            .is_some_and(|receiver| Self::is_shared_self(receiver, declared.self_variable))
    }

    fn is_shared_self(receiver: &TypeExpr, self_variable: BindingId) -> bool {
        let TypeExprKind::Reference(ReferenceKind::Shared, _, referent) = receiver.kind() else {
            return false;
        };
        matches!(referent.kind(), TypeExprKind::Named(binding, _) if *binding == self_variable)
    }

    fn derive_writes(path: &str) -> bool {
        StructDerives::DERIVABLE.contains(&StructDerives::derive_name(path))
    }
}

impl<'f> ClauseReading<'f> {
    fn read(
        clause: &'f Form,
        declared: &DeclaredTrait,
    ) -> Result<StructuralClause, Vec<Diagnostic>> {
        let mut reading = ClauseReading {
            clause,
            parameters: Self::method_parameters(declared),
            templates: Vec::new(),
            problems: Vec::new(),
        };
        reading.read_pairs();
        reading.report_missing_keys();
        reading.finish()
    }

    fn method_parameters(declared: &DeclaredTrait) -> usize {
        match declared
            .methods
            .first()
            .map(|method| method.signature.kind())
        {
            Some(TypeExprKind::Function(_, parameters, _)) => parameters.len(),
            _ => 0,
        }
    }

    fn read_pairs(&mut self) {
        let entries = self.clause.as_list().unwrap_or_default();
        let mut pairs = entries[1..].iter().peekable();
        while let Some(key) = pairs.next() {
            let template = pairs.next_if(|_| key.as_symbol().is_some());
            if let Err(problem) = self.read_pair(key, template) {
                self.problems.push(problem);
            }
        }
    }

    fn read_pair(&mut self, key_form: &'f Form, template: Option<&Form>) -> Result<(), Diagnostic> {
        let key = Self::key(key_form)?;
        if let Some((_, _, first)) = self.templates.iter().find(|(held, _, _)| *held == key) {
            return Err(ExpandProblems::repeated_structural_key(
                &key.keyword(),
                key_form.span(),
                first.span(),
            ));
        }
        let Some(FormKind::String(text)) = template.map(Form::kind) else {
            return Err(ExpandProblems::structural_key_without_template(
                &key.keyword(),
                key_form.span(),
            ));
        };
        let parsed = StructuralTemplate::parse(text, key, self.parameters).map_err(|unfilled| {
            ExpandProblems::unfilled_structural_placeholder(
                &key.keyword(),
                &unfilled.0,
                &key.accepted_placeholders(self.parameters),
                template.map_or(key_form.span(), Form::span),
            )
        })?;
        self.templates.push((key, parsed, key_form));
        Ok(())
    }

    fn key(form: &Form) -> Result<StructuralKey, Diagnostic> {
        form.as_symbol()
            .and_then(|symbol| StructuralKey::from_keyword(symbol.name()))
            .ok_or_else(|| {
                let keys: Vec<String> = StructuralKey::ALL.map(StructuralKey::keyword).to_vec();
                ExpandProblems::unknown_structural_key(&form.to_string(), &keys, form.span())
            })
    }

    fn report_missing_keys(&mut self) {
        let missing = StructuralKey::ALL
            .into_iter()
            .find(|key| !self.templates.iter().any(|(held, _, _)| held == key));
        if let Some(missing) = missing.filter(|_| self.problems.is_empty()) {
            self.problems.push(ExpandProblems::missing_structural_key(
                &missing.keyword(),
                self.clause.span(),
            ));
        }
    }

    fn finish(self) -> Result<StructuralClause, Vec<Diagnostic>> {
        if !self.problems.is_empty() {
            return Err(self.problems);
        }
        Ok(StructuralClause::new(
            self.templates
                .into_iter()
                .map(|(key, template, _)| (key, template))
                .collect(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::structural_path_claims::StructuralPathClaims;
    use super::{DeclaredTrait, StructuralTraits};
    use crate::core::{
        BindingArena, BindingId, BindingInfo, BindingKind, ClosureKind, ReferenceKind, TraitMethod,
        TypeExpr, TypeExprKind,
    };
    use crate::diagnostic::Diagnostics;
    use crate::reader::{Form, Reader};
    use crate::source::{SourceId, SourceMap, Span};

    struct StructuralFixture {
        bindings: BindingArena,
        claims: StructuralPathClaims,
        problems: Diagnostics,
        sources: SourceMap,
        self_variable: BindingId,
    }

    impl StructuralFixture {
        const CLAUSE: &str = r#"(structural :struct "write!({1}, \"{name}\"){parts}" :field "" :variant "write!({1}, \"{{{name}}}\")" :part "" :bare "write!({1}, \"{name}\")")"#;
        const PATH: &str = "std::fmt::Octal";

        fn new() -> Self {
            let mut bindings = BindingArena::default();
            let self_variable =
                bindings.add(BindingInfo::new("self".into(), BindingKind::Local, None));
            Self {
                bindings,
                claims: StructuralPathClaims::default(),
                problems: Diagnostics::default(),
                sources: SourceMap::default(),
                self_variable,
            }
        }

        fn declare_trait(&mut self, name: &str) -> BindingId {
            self.bindings
                .add(BindingInfo::new(name.into(), BindingKind::Local, None))
        }

        fn clause(&mut self, text: &str) -> Form {
            let file = self.sources.add("traits.knv", text);
            let mut forms = Reader::new(self.sources.file(file))
                .read()
                .unwrap_or_else(|_| panic!("the clause reads"));
            forms.remove(0)
        }

        fn self_method(&self, receiver: ReferenceKind) -> TraitMethod {
            self.method_taking(receiver, self.self_variable)
        }

        fn method_taking(&self, receiver: ReferenceKind, referent: BindingId) -> TraitMethod {
            let span = Span::new(SourceId::new(0), 0..0);
            let named_self =
                TypeExpr::new(TypeExprKind::Named(self.self_variable, Vec::new()), span);
            let named_referent = TypeExpr::new(TypeExprKind::Named(referent, Vec::new()), span);
            let reference = TypeExprKind::Reference(receiver, None, Box::new(named_referent));
            let signature = TypeExprKind::Function(
                ClosureKind::Fn,
                vec![TypeExpr::new(reference, span), named_self.clone()],
                Box::new(named_self),
            );
            TraitMethod {
                binding: self.self_variable,
                signature: TypeExpr::new(signature, span),
                default: None,
            }
        }

        fn reported_problems(
            &mut self,
            owner: BindingId,
            methods: &[TraitMethod],
            text: &str,
        ) -> usize {
            let clause = self.clause(text);
            let declared = DeclaredTrait {
                binding: owner,
                definer: "rust-trait",
                path: Self::PATH,
                path_owner: None,
                self_variable: self.self_variable,
                methods,
            };
            let before = self.problems.len();
            StructuralTraits::new(&self.bindings, &mut self.claims, &mut self.problems)
                .lower(&clause, &declared);
            self.problems.len() - before
        }
    }

    #[test]
    fn a_clause_of_a_trait_with_one_shared_self_method_lowers() {
        let mut fixture = StructuralFixture::new();
        let octal = fixture.declare_trait("octal");
        let methods = [fixture.self_method(ReferenceKind::Shared)];
        assert_eq!(
            fixture.reported_problems(octal, &methods, StructuralFixture::CLAUSE),
            0
        );
    }

    #[test]
    fn a_trait_whose_method_takes_a_mutable_self_has_no_structural_clause() {
        let mut fixture = StructuralFixture::new();
        let octal = fixture.declare_trait("octal");
        let methods = [fixture.self_method(ReferenceKind::Mutable)];
        assert_eq!(
            fixture.reported_problems(octal, &methods, StructuralFixture::CLAUSE),
            1
        );
    }

    #[test]
    fn a_trait_whose_method_takes_another_type_first_has_no_structural_clause() {
        let mut fixture = StructuralFixture::new();
        let octal = fixture.declare_trait("octal");
        let other = fixture.declare_trait("other");
        let methods = [fixture.method_taking(ReferenceKind::Shared, other)];
        assert_eq!(
            fixture.reported_problems(octal, &methods, StructuralFixture::CLAUSE),
            1
        );
    }

    #[test]
    fn a_second_trait_cannot_claim_the_same_path() {
        let mut fixture = StructuralFixture::new();
        let octal = fixture.declare_trait("octal");
        let other = fixture.declare_trait("other-octal");
        let methods = [fixture.self_method(ReferenceKind::Shared)];
        let _ = fixture.reported_problems(octal, &methods, StructuralFixture::CLAUSE);
        assert_eq!(
            fixture.reported_problems(other, &methods, StructuralFixture::CLAUSE),
            1
        );
    }

    #[test]
    fn a_clause_missing_a_key_reports_it() {
        let mut fixture = StructuralFixture::new();
        let octal = fixture.declare_trait("octal");
        let methods = [fixture.self_method(ReferenceKind::Shared)];
        assert_eq!(
            fixture.reported_problems(octal, &methods, r#"(structural :struct "")"#),
            1
        );
    }
}
