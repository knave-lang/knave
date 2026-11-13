use std::fmt;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::SavedValue;

pub trait ImplementorRow {
    fn is_primitive(&self) -> bool;
}

#[derive(Clone)]
pub struct ImplementorRows<R> {
    rows: Rows<R>,
    only_primitives: bool,
}

#[derive(Clone)]
enum Rows<R> {
    Built(Vec<R>),
    Saved(SavedValue<Vec<R>>),
}

impl<R: ImplementorRow + Serialize + DeserializeOwned> ImplementorRows<R> {
    pub fn new(rows: Vec<R>) -> Self {
        let only_primitives = !rows.is_empty() && rows.iter().all(ImplementorRow::is_primitive);
        Self {
            rows: Rows::Built(rows),
            only_primitives,
        }
    }

    pub fn get(&self) -> &[R] {
        match &self.rows {
            Rows::Built(rows) => rows,
            Rows::Saved(saved) => saved.decoded(),
        }
    }

    pub fn only_primitives(&self) -> bool {
        self.only_primitives
    }

    fn encoded(&self) -> Vec<u8> {
        bincode::serialize(self.get()).expect("rows encode")
    }
}

impl<R: ImplementorRow + Serialize + DeserializeOwned + fmt::Debug> fmt::Debug
    for ImplementorRows<R>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.get()).finish()
    }
}

impl<R: ImplementorRow + Serialize + DeserializeOwned> Serialize for ImplementorRows<R> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (self.only_primitives, self.encoded().as_slice()).serialize(serializer)
    }
}

impl<'de: 'static, R: DeserializeOwned> serde::Deserialize<'de> for ImplementorRows<R> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (only_primitives, saved) = <(bool, &'static [u8])>::deserialize(deserializer)?;
        Ok(Self {
            rows: Rows::Saved(SavedValue::borrowing(saved)),
            only_primitives,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ImplementorRow, ImplementorRows};

    #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct NamedRow {
        name: u32,
        primitive: bool,
    }

    impl ImplementorRow for NamedRow {
        fn is_primitive(&self) -> bool {
            self.primitive
        }
    }

    struct RowsFixture;

    impl RowsFixture {
        fn row(name: u32, primitive: bool) -> NamedRow {
            NamedRow { name, primitive }
        }

        fn reloaded(rows: &ImplementorRows<NamedRow>) -> ImplementorRows<NamedRow> {
            let saved = bincode::serialize(rows).expect("the rows encode");
            bincode::deserialize(saved.leak()).expect("the rows decode")
        }
    }

    #[test]
    fn saved_rows_decode_to_the_rows_written() {
        let written = vec![RowsFixture::row(3, false), RowsFixture::row(5, true)];
        let rows = ImplementorRows::new(written.clone());
        assert_eq!(RowsFixture::reloaded(&rows).get(), written.as_slice());
    }

    #[test]
    fn loaded_rows_save_the_same_bytes_again() {
        let rows = ImplementorRows::new(vec![RowsFixture::row(7, false)]);
        let saved = bincode::serialize(&rows).expect("the rows encode");
        let resaved = bincode::serialize(&RowsFixture::reloaded(&rows)).expect("the rows encode");
        assert_eq!(resaved, saved);
    }

    #[test]
    fn rows_of_primitives_only_say_so_without_decoding() {
        let primitives = ImplementorRows::new(vec![RowsFixture::row(1, true)]);
        let mixed =
            ImplementorRows::new(vec![RowsFixture::row(1, true), RowsFixture::row(2, false)]);
        let empty = ImplementorRows::<NamedRow>::new(Vec::new());
        assert!(RowsFixture::reloaded(&primitives).only_primitives());
        assert!(!RowsFixture::reloaded(&mixed).only_primitives());
        assert!(!RowsFixture::reloaded(&empty).only_primitives());
    }
}
