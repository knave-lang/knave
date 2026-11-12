use crate::helper_template::HelperTemplate;

pub struct GeneratedMain {
    lines: Vec<String>,
    helpers: Vec<&'static HelperTemplate>,
}

impl GeneratedMain {
    const DECLARATIONS: [&'static str; 6] = [
        "(defstruct car :make string :year i64)",
        "(defstruct tag :label (ref str))",
        "(defenum shape (circle i64) (square i64) point)",
        "(deftrait sized\n  (size (fn ((ref self)) i64))\n  (twice (fn ((ref self)) i64) (* 2 (size self))))",
        "(impl sized car\n  (size (c) (:year c)))",
        "(impl sized shape\n  (size (s) (match s ((shape/circle r) r) ((shape/square w) w) (shape/point 0))))",
    ];

    pub fn new(lines: Vec<String>, helpers: Vec<&'static HelperTemplate>) -> Self {
        Self { lines, helpers }
    }

    pub fn program_text(&self) -> String {
        let mut sections: Vec<String> = Self::DECLARATIONS
            .iter()
            .map(|text| text.to_string())
            .collect();
        sections.extend(
            self.helpers
                .iter()
                .map(|helper| helper.definition.to_string()),
        );
        sections.push(format!("(defn main ()\n  {})", self.lines.join("\n  ")));
        sections.join("\n\n") + "\n"
    }
}
