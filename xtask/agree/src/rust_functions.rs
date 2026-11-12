pub struct RustFunctions {
    starts: Vec<(usize, String)>,
}

impl RustFunctions {
    const FUNCTION_KEYWORD: &str = "fn ";
    const VISIBILITIES: [&str; 3] = ["pub(crate) ", "pub ", ""];
    const QUALIFIERS: [&str; 4] = ["const ", "async ", "unsafe ", ""];

    pub fn of_text(text: &str) -> Self {
        let mut starts = Vec::new();
        let mut offset = 0;
        for line in text.split_inclusive('\n') {
            if let Some(name) = Self::declared_name(line.trim_start()) {
                starts.push((offset, name));
            }
            offset += line.len();
        }
        Self { starts }
    }

    pub fn name_at(&self, offset: usize) -> Option<&str> {
        self.starts
            .iter()
            .take_while(|(start, _)| *start <= offset)
            .last()
            .map(|(_, name)| name.as_str())
    }

    fn declared_name(line: &str) -> Option<String> {
        let declaration = Self::VISIBILITIES.iter().find_map(|visibility| {
            let rest = line.strip_prefix(visibility)?;
            Self::QUALIFIERS.iter().find_map(|qualifier| {
                rest.strip_prefix(qualifier)?
                    .strip_prefix(Self::FUNCTION_KEYWORD)
            })
        })?;
        let name: String = declaration
            .chars()
            .take_while(|character| character.is_alphanumeric() || *character == '_')
            .collect();
        (!name.is_empty()).then_some(name)
    }
}

#[cfg(test)]
mod tests {
    use super::RustFunctions;

    #[test]
    fn names_the_function_whose_body_holds_an_offset() {
        let text = "fn grow(v: &mut Vec<i64>) {\n    v.push(1);\n}\n\npub fn main() {\n    grow(&mut v);\n}\n";
        let functions = RustFunctions::of_text(text);
        assert_eq!(functions.name_at(30), Some("grow"));
        assert_eq!(functions.name_at(text.len() - 3), Some("main"));
    }

    #[test]
    fn names_a_qualified_function() {
        let functions = RustFunctions::of_text("pub const fn seven() -> i64 {\n    7\n}\n");
        assert_eq!(functions.name_at(20), Some("seven"));
    }

    #[test]
    fn names_nothing_before_the_first_function() {
        let functions = RustFunctions::of_text("use std::fmt;\nfn main() {}\n");
        assert_eq!(functions.name_at(2), None);
    }
}
