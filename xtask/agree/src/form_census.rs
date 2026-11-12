use std::collections::BTreeMap;
use std::fmt;

use knave_expand::core::{ExpressionForm, KernelForm, LeafForm};
use knave_expand::reader::{TokenKind, TokenScan};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormCensus {
    pub kernel_forms: BTreeMap<String, usize>,
    pub prelude_names: BTreeMap<String, usize>,
}

impl FormCensus {
    pub fn of_text(text: &str) -> Self {
        let mut census = Self::default();
        let mut after_open = false;
        for token in TokenScan::new(text).filter(|token| token.kind != TokenKind::Comment) {
            if after_open && token.kind == TokenKind::Symbol {
                census.count(&text[token.range.clone()]);
            }
            after_open = token.kind == TokenKind::Open;
        }
        census
    }

    pub fn add(&mut self, other: &FormCensus) {
        Self::add_counts(&mut self.kernel_forms, &other.kernel_forms);
        Self::add_counts(&mut self.prelude_names, &other.prelude_names);
    }

    pub fn uses_rust_templates(&self) -> bool {
        let template = KernelForm::Expression(ExpressionForm::Leaf(LeafForm::Rust));
        self.kernel_forms.contains_key(template.name())
    }

    fn count(&mut self, head: &str) {
        let counts = if Self::is_kernel_form(head) {
            &mut self.kernel_forms
        } else {
            &mut self.prelude_names
        };
        *counts.entry(head.to_string()).or_default() += 1;
    }

    fn is_kernel_form(head: &str) -> bool {
        KernelForm::all().any(|form| form.name() == head)
    }

    fn add_counts(total: &mut BTreeMap<String, usize>, more: &BTreeMap<String, usize>) {
        for (name, count) in more {
            *total.entry(name.clone()).or_default() += count;
        }
    }
}

impl fmt::Display for FormCensus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kernel: Vec<&str> = self.kernel_forms.keys().map(String::as_str).collect();
        writeln!(formatter, "  kernel forms: {}", kernel.join(" "))?;
        let prelude: Vec<&str> = self.prelude_names.keys().map(String::as_str).collect();
        writeln!(formatter, "  prelude names: {}", prelude.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::FormCensus;

    #[test]
    fn counts_each_list_head_as_a_kernel_form_or_a_prelude_name() {
        let census = FormCensus::of_text("(defn main () (if (< 1 2) (println 1) (do)))");
        assert_eq!(census.kernel_forms.get("if"), Some(&1));
        assert_eq!(census.kernel_forms.get("do"), Some(&1));
        assert_eq!(census.prelude_names.get("defn"), Some(&1));
        assert_eq!(census.prelude_names.get("println"), Some(&1));
        assert_eq!(census.prelude_names.get("main"), None);
    }

    #[test]
    fn skips_comments_and_heads_inside_strings() {
        let census = FormCensus::of_text("; (if)\n(println \"(do)\")");
        assert!(census.kernel_forms.is_empty());
        assert_eq!(census.prelude_names.get("println"), Some(&1));
    }

    #[test]
    fn adds_the_counts_of_another_census() {
        let mut census = FormCensus::of_text("(println 1)");
        census.add(&FormCensus::of_text("(println 2) (if a b c)"));
        assert_eq!(census.prelude_names.get("println"), Some(&2));
        assert_eq!(census.kernel_forms.get("if"), Some(&1));
    }
}
