use std::collections::BTreeMap;

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
}
