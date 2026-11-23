use rustc_hash::FxHashSet;

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

    pub(super) fn first_free_from(first: char, taken: &FxHashSet<&str>) -> String {
        let start = Self::LETTERS.find(first).unwrap_or_default();
        (start..)
            .map(Self::at)
            .find(|name| !taken.contains(name.as_str()))
            .unwrap_or_default()
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

    #[test]
    fn the_first_free_name_skips_taken_names() {
        let taken = ["r", "s"].into_iter().collect();
        assert_eq!(ParameterNames::first_free_from('r', &taken), "t");
        let late: rustc_hash::FxHashSet<&str> = ["y", "z"].into_iter().collect();
        assert_eq!(ParameterNames::first_free_from('y', &late), "t26");
    }
}
