use crate::helper_template::HelperTemplate;
use crate::shape::Shape;

pub struct GeneratedMain {
    lines: Vec<String>,
    helpers: Vec<&'static HelperTemplate>,
}

impl GeneratedMain {
    const FIGURE_SIZE_SOURCE: &'static str = "s";

    const DECLARATIONS: [&'static str; 5] = [
        "(defstruct car :make string :year i64)",
        "(defstruct tag :label (ref str))",
        "(defenum figure (circle i64) (square i64) point)",
        "(deftrait sized\n  (size (fn ((ref self)) i64))\n  (twice (fn ((ref self)) i64) (* 2 (size self))))",
        "(impl sized car\n  (size (c) (:year c)))",
    ];

    pub fn new(lines: Vec<String>, helpers: Vec<&'static HelperTemplate>) -> Self {
        Self { lines, helpers }
    }

    pub fn program_text(&self) -> String {
        let mut sections: Vec<String> = Self::DECLARATIONS
            .iter()
            .map(|text| text.to_string())
            .collect();
        sections.push(Self::figure_impl());
        sections.extend(
            self.helpers
                .iter()
                .map(|helper| helper.definition.to_string()),
        );
        sections.push(format!("(defn main ()\n  {})", self.lines.join("\n  ")));
        sections.join("\n\n") + "\n"
    }

    fn figure_impl() -> String {
        format!(
            "(impl sized figure\n  (size ({}) {}))",
            Self::FIGURE_SIZE_SOURCE,
            Shape::SIZE_BY_VARIANT.filled(Self::FIGURE_SIZE_SOURCE)
        )
    }
}
