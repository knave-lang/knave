use std::cell::OnceCell;

use serde::de::DeserializeOwned;

#[derive(Clone)]
pub struct SavedValue<T> {
    encoded: &'static [u8],
    decoded: OnceCell<T>,
}

impl<T: DeserializeOwned> SavedValue<T> {
    pub fn borrowing(encoded: &'static [u8]) -> Self {
        Self {
            encoded,
            decoded: OnceCell::new(),
        }
    }

    pub fn decoded(&self) -> &T {
        self.decoded
            .get_or_init(|| bincode::deserialize(self.encoded).expect("a saved value decodes"))
    }

    #[cfg(test)]
    pub fn encoded(&self) -> &'static [u8] {
        self.encoded
    }

    #[cfg(test)]
    pub fn is_decoded(&self) -> bool {
        self.decoded.get().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::SavedValue;

    struct SavedFixture;

    impl SavedFixture {
        fn saved(words: &[&str]) -> SavedValue<Vec<String>> {
            let encoded = bincode::serialize(words).expect("the words encode");
            SavedValue::borrowing(encoded.leak())
        }
    }

    #[test]
    fn a_saved_value_decodes_only_when_read() {
        let saved = SavedFixture::saved(&["pear", "fig"]);
        assert!(!saved.is_decoded());
        assert_eq!(
            saved.decoded(),
            &vec!["pear".to_string(), "fig".to_string()]
        );
        assert!(saved.is_decoded());
    }
}
