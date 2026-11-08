use std::cell::OnceCell;

use super::BindingInfo;

#[derive(Default, serde::Serialize)]
pub struct BindingTables {
    offsets: Vec<u8>,
    infos: Vec<u8>,
}

impl BindingTables {
    const WORD: usize = size_of::<u32>();

    pub fn of<'a>(infos: impl Iterator<Item = &'a BindingInfo>) -> Self {
        let mut tables = Self::default();
        for info in infos {
            tables.push_offset();
            let encoded = bincode::serialize(info).expect("every binding serializes");
            tables.infos.extend(encoded);
        }
        tables.push_offset();
        tables
    }

    fn push_offset(&mut self) {
        let offset = u32::try_from(self.infos.len()).expect("the saved prelude stays under 4 GiB");
        self.offsets.extend(offset.to_le_bytes());
    }
}

#[derive(Default)]
pub struct SavedBindings {
    offsets: &'static [u8],
    infos: &'static [u8],
    decoded: Vec<OnceCell<Box<BindingInfo>>>,
}

impl<'de: 'static> serde::Deserialize<'de> for SavedBindings {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (offsets, infos): (&'static [u8], &'static [u8]) =
            serde::Deserialize::deserialize(deserializer)?;
        let count = (offsets.len() / BindingTables::WORD).saturating_sub(1);
        Ok(Self {
            offsets,
            infos,
            decoded: (0..count).map(|_| OnceCell::new()).collect(),
        })
    }
}

impl SavedBindings {
    pub fn count(&self) -> usize {
        self.decoded.len()
    }

    pub fn get(&self, index: usize) -> &BindingInfo {
        self.decoded[index].get_or_init(|| Box::new(self.decode(index)))
    }

    #[cfg(test)]
    pub fn is_decoded(&self, index: usize) -> bool {
        self.decoded[index].get().is_some()
    }

    fn decode(&self, index: usize) -> BindingInfo {
        let (start, end) = (self.offset(index), self.offset(index + 1));
        bincode::deserialize(&self.infos[start..end]).expect("a saved binding decodes")
    }

    fn offset(&self, index: usize) -> usize {
        let at = index * BindingTables::WORD;
        let word = &self.offsets[at..at + BindingTables::WORD];
        u32::from_le_bytes(word.try_into().expect("a saved offset has four bytes")) as usize
    }
}
