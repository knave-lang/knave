use std::ops::Range;

pub struct FormBlock {
    pub text: String,
    pub ranges: Vec<Range<usize>>,
}

impl FormBlock {
    const START: &'static str = "(do ";
    const END: &'static str = " (do))";
    const SEPARATOR: &'static str = " ";

    pub fn of(forms: &[String]) -> Self {
        let mut text = String::from(Self::START);
        let mut ranges = Vec::new();
        for form in forms {
            text.push_str(Self::SEPARATOR);
            ranges.push(text.len()..text.len() + form.len());
            text.push_str(form);
        }
        text.push_str(Self::END);
        Self { text, ranges }
    }
}
