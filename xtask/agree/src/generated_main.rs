use crate::helper_template::HelperTemplate;

pub struct GeneratedMain {
    lines: Vec<String>,
    helpers: Vec<&'static HelperTemplate>,
}

impl GeneratedMain {
    const STRUCTS: [&'static str; 2] = [
        "(defstruct car :make string :year i64)",
        "(defstruct tag :label (ref str))",
    ];

    pub fn new(lines: Vec<String>, helpers: Vec<&'static HelperTemplate>) -> Self {
        Self { lines, helpers }
    }

    pub fn program_text(&self) -> String {
        let mut sections: Vec<String> = Self::STRUCTS.iter().map(|text| text.to_string()).collect();
        sections.extend(
            self.helpers
                .iter()
                .map(|helper| helper.definition.to_string()),
        );
        sections.push(format!("(defn main ()\n  {})", self.lines.join("\n  ")));
        sections.join("\n\n") + "\n"
    }
}
