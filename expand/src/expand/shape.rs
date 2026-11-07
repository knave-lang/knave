use std::rc::Rc;

use super::problems::TemplateSpelling;
use crate::core::{DefinitionForm, EnumDefiner, KernelForm};
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::reader::{Form, FormKind, Symbol};

pub struct Shape<'a> {
    kernel: KernelForm,
    form: &'a Form,
    definer: Option<WrittenDefiner>,
}

pub struct WrittenDefiner {
    pub name: Rc<str>,
    pub through_prelude: bool,
}

impl<'a> Shape<'a> {
    pub fn new(kernel: KernelForm, form: &'a Form) -> Self {
        Self {
            kernel,
            form,
            definer: None,
        }
    }

    pub fn expanded_from(
        kernel: KernelForm,
        form: &'a Form,
        definer: Option<WrittenDefiner>,
    ) -> Self {
        Self {
            kernel,
            form,
            definer,
        }
    }

    pub fn definer(&self) -> Option<&Rc<str>> {
        self.definer.as_ref().map(|definer| &definer.name)
    }

    pub fn written_name(&self) -> &str {
        self.definer().map_or(self.kernel.name(), |name| name)
    }

    pub fn template_spelling(&self) -> TemplateSpelling {
        let rust_type = KernelForm::Definition(DefinitionForm::RustType);
        match &self.definer {
            None if self.kernel == rust_type => TemplateSpelling::KernelString,
            Some(_) if self.kernel == rust_type && self.through_prelude() => {
                TemplateSpelling::RustClause
            }
            _ => TemplateSpelling::Elided,
        }
    }

    pub fn enum_definer(&self) -> Option<EnumDefiner> {
        self.definer.as_ref().map(|definer| EnumDefiner {
            name: Rc::clone(&definer.name),
            flat_fields: definer.through_prelude,
        })
    }

    fn through_prelude(&self) -> bool {
        self.definer
            .as_ref()
            .is_some_and(|definer| definer.through_prelude)
    }

    fn written_shape(&self) -> String {
        match &self.definer {
            Some(definer) if definer.through_prelude => self.prelude_spelling(&definer.name),
            _ => self.kernel.shape().to_string(),
        }
    }

    fn prelude_spelling(&self, definer: &str) -> String {
        let kernel_head = format!("({}", self.kernel.name());
        let body = self
            .kernel
            .shape()
            .strip_prefix(&kernel_head)
            .expect("a kernel shape starts with the kernel form's name");
        if self.names_rust_thing() {
            format!("({definer}{}", Self::with_rust_clause(body))
        } else {
            format!("({definer}{body}")
        }
    }

    fn names_rust_thing(&self) -> bool {
        matches!(
            self.kernel,
            KernelForm::Definition(DefinitionForm::RustType | DefinitionForm::RustTrait)
        )
    }

    fn with_rust_clause(body: &str) -> String {
        let open = body
            .find('"')
            .expect("the shape of a Rust definition holds its string");
        let close = open
            + 1
            + body[open + 1..]
                .find('"')
                .expect("the string of a shape is closed");
        let string = &body[open..=close];
        format!("{}(rust {string}){}", &body[..open], &body[close + 1..])
    }

    pub fn items(&self) -> &'a [Form] {
        self.form.as_list().unwrap_or_default()
    }

    pub fn exactly(&self, count: usize) -> Result<&'a [Form], Diagnostic> {
        let items = self.items();
        if items.len() == count {
            Ok(items)
        } else {
            Err(self.malformed(self.form))
        }
    }

    pub fn at_least(&self, count: usize) -> Result<&'a [Form], Diagnostic> {
        let items = self.items();
        if items.len() >= count {
            Ok(items)
        } else {
            Err(self.malformed(self.form))
        }
    }

    pub fn symbol(&self, part: &'a Form) -> Result<&'a Symbol, Diagnostic> {
        part.as_symbol().ok_or_else(|| self.malformed(part))
    }

    pub fn list(&self, part: &'a Form) -> Result<&'a [Form], Diagnostic> {
        part.as_list().ok_or_else(|| self.malformed(part))
    }

    pub fn string(&self, part: &'a Form) -> Result<&'a str, Diagnostic> {
        match part.kind() {
            FormKind::String(text) => Ok(text),
            _ => Err(self.malformed(part)),
        }
    }

    pub fn malformed(&self, part: &Form) -> Diagnostic {
        self.malformed_with_help(part, self.shape_help())
    }

    fn shape_help(&self) -> String {
        format!("write it as `{}`", self.written_shape())
    }

    pub fn malformed_form_with_help(&self, help: impl Into<String>) -> Diagnostic {
        self.malformed_with_help(self.form, help)
    }

    pub fn malformed_with_help(&self, part: &Form, help: impl Into<String>) -> Diagnostic {
        let name = self.written_name();
        Diagnostic::error(
            DiagnosticCode::MALFORMED_FORM,
            format!("malformed `{name}`"),
            Label::new(
                part.span(),
                format!("this does not fit the shape of `{name}`"),
            ),
        )
        .with_help(help)
    }
}
