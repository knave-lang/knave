use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use knave::core::{CorePrinter, KernelForm, ModulePath};
use knave::expand::{Expander, Program};
use knave::lint::ReadBack;
use knave::modules::ModuleSources;
use knave::reader::Form;
use knave::source::SourceMap;

pub struct ReadBackAgreement {
    program: Program,
}

impl ReadBackAgreement {
    const FIXTURE: &str = "tests/fixtures/read_back/kernel_forms.knv";
    const MODULE: &str = "kernel_forms";
    pub const DELIBERATE_DIFFERENCES: [&str; 3] = [
        "the printer writes a Rust template's type signature",
        "the printer writes a parameter's type annotation",
        "the printer renames a repeated name with an `_N` suffix",
    ];
    const ONLY_DEF_ITEMS_ARE_READ_BACK: &str = "the read-back reads only `def` items";
    pub const NEVER_HELD: [(&str, &str); 12] = [
        ("type", Self::ONLY_DEF_ITEMS_ARE_READ_BACK),
        ("struct", Self::ONLY_DEF_ITEMS_ARE_READ_BACK),
        ("enum", Self::ONLY_DEF_ITEMS_ARE_READ_BACK),
        ("derive", Self::ONLY_DEF_ITEMS_ARE_READ_BACK),
        ("rust-trait", Self::ONLY_DEF_ITEMS_ARE_READ_BACK),
        ("rust-type", Self::ONLY_DEF_ITEMS_ARE_READ_BACK),
        ("trait", Self::ONLY_DEF_ITEMS_ARE_READ_BACK),
        ("impl", Self::ONLY_DEF_ITEMS_ARE_READ_BACK),
        ("use", Self::ONLY_DEF_ITEMS_ARE_READ_BACK),
        ("export", Self::ONLY_DEF_ITEMS_ARE_READ_BACK),
        (
            "macro",
            "macro code is not linted, so no `macro` reaches a read-back",
        ),
        (
            "rust",
            "every Rust template prints its type signature, the first deliberate difference",
        ),
    ];

    pub fn load() -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(Self::FIXTURE);
        let text = fs::read_to_string(&path).expect("the fixture is readable");
        let mut sources = SourceMap::default();
        let entry = sources.add(Self::FIXTURE, text);
        let directory = path
            .parent()
            .expect("the fixture has a directory")
            .to_path_buf();
        let program = Expander::new(
            &mut sources,
            ModuleSources::new(directory.clone(), directory),
        )
        .expand(ModulePath::new(Self::MODULE), entry)
        .unwrap_or_else(|_| panic!("the fixture expands"));
        Self { program }
    }

    pub fn disagreements(&mut self) -> Vec<String> {
        let printed = CorePrinter::new(&self.program.bindings).module(self.program.entry_module());
        let printed: Vec<&str> = printed
            .lines()
            .filter(|line| line.starts_with("(def "))
            .collect();
        let read_back: Vec<String> = self.read_back().iter().map(Form::to_string).collect();
        if printed.len() != read_back.len() {
            return vec![format!(
                "the printer writes {} definitions and the read-back {}",
                printed.len(),
                read_back.len()
            )];
        }
        printed
            .iter()
            .zip(&read_back)
            .filter(|(printed, read)| *printed != read)
            .map(|(printed, read)| format!("printed:   {printed}\nread back: {read}"))
            .collect()
    }

    pub fn kernel_heads(&mut self) -> BTreeSet<String> {
        let mut heads = BTreeSet::new();
        let mut pending = self.read_back();
        while let Some(form) = pending.pop() {
            if let Some(items) = form.as_list() {
                heads.extend(items.first().and_then(Self::kernel_head));
                pending.extend(items.iter().cloned());
            }
        }
        heads
    }

    pub fn is_never_held(name: &str) -> bool {
        Self::NEVER_HELD.iter().any(|(form, _)| *form == name)
    }

    fn kernel_head(head: &Form) -> Option<String> {
        let symbol = head.as_symbol()?;
        let names_no_binding = !symbol.scopes().any(|_| true);
        let is_kernel = KernelForm::all().any(|form| form.name() == symbol.name());
        (names_no_binding && is_kernel).then(|| symbol.name().to_string())
    }

    fn read_back(&mut self) -> Vec<Form> {
        let entry = self.program.entry.module;
        let entry_module = self
            .program
            .modules
            .iter()
            .filter(|module| module.id == entry);
        ReadBack::new(&self.program.bindings, &mut self.program.scopes)
            .definitions(entry_module)
            .into_iter()
            .map(|definition| definition.form)
            .collect()
    }
}
