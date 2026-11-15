pub(super) struct QuotedList;

impl QuotedList {
    pub(super) fn of(words: &[impl AsRef<str>]) -> String {
        let quoted: Vec<String> = words
            .iter()
            .map(|word| format!("`{}`", word.as_ref()))
            .collect();
        let (last, earlier) = quoted
            .split_last()
            .expect("the list names at least one word");
        format!("{} and {last}", earlier.join(", "))
    }
}
