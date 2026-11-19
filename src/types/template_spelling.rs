use crate::core::ParsedTemplate;

pub struct TemplateSpelling<'t> {
    text: &'t str,
}

impl<'t> TemplateSpelling<'t> {
    pub fn of(template: &'t ParsedTemplate) -> Self {
        Self {
            text: template.text(),
        }
    }

    pub fn is(&self, wanted: &str) -> bool {
        self.text
            .chars()
            .filter(|character| !character.is_whitespace())
            .eq(wanted
                .chars()
                .filter(|character| !character.is_whitespace()))
    }
}

#[cfg(test)]
mod tests {
    use super::TemplateSpelling;
    use crate::core::ParsedTemplate;

    #[test]
    fn spells_a_template_whatever_its_whitespace() {
        let parsed = ParsedTemplate::parse("HashMap< {0},{1} >").expect("a map template");
        assert!(TemplateSpelling::of(&parsed).is("HashMap<{0}, {1}>"));
        assert!(!TemplateSpelling::of(&parsed).is("BTreeMap<{0}, {1}>"));
    }
}
