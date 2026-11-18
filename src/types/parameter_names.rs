pub(super) struct ParameterNames;

impl ParameterNames {
    const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";

    pub(super) fn at(position: usize) -> String {
        let letters: Vec<char> = Self::LETTERS.chars().collect();
        match letters.get(position) {
            Some(letter) => letter.to_string(),
            None => format!("t{position}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ParameterNames;

    #[test]
    fn parameters_past_the_alphabet_are_numbered() {
        assert_eq!(ParameterNames::at(0), "a");
        assert_eq!(ParameterNames::at(26), "t26");
    }
}
