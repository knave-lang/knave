use super::name_slots::{NameSlots, NameSlotsBuilder};
use super::saved_key::SavedKey;
use super::saved_words::{SavedWords, SavedWordsBuilder};

#[derive(Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct SavedTexts<const WIDTH: usize> {
    #[serde(borrow)]
    slots: &'static [u8],
    #[serde(borrow)]
    rows: &'static [u8],
    #[serde(borrow)]
    texts: &'static [u8],
}

impl<const WIDTH: usize> SavedTexts<WIDTH> {
    const TEXT_START: usize = 0;
    const TEXT_END: usize = 1;

    pub(super) fn position<'a>(
        &self,
        key: impl SavedKey<'a>,
        holds_rest_of_key: impl Fn([u32; WIDTH]) -> bool,
    ) -> Option<usize> {
        NameSlots::new(self.slots).find(key, |position| {
            let row = self.row(position);
            holds_rest_of_key(row) && self.bytes(row) == key.text().as_bytes()
        })
    }

    pub(super) fn text(&self, position: usize) -> &'static str {
        let row = self.row(position);
        std::str::from_utf8(self.bytes(row)).expect("a saved text is text")
    }

    pub(super) fn row(&self, position: usize) -> [u32; WIDTH] {
        SavedWords::new(self.rows).row(position)
    }

    pub(super) fn count(&self) -> usize {
        SavedWords::<WIDTH>::new(self.rows).rows()
    }

    fn bytes(&self, row: [u32; WIDTH]) -> &'static [u8] {
        &self.texts[row[Self::TEXT_START] as usize..row[Self::TEXT_END] as usize]
    }
}

pub(super) struct SavedTextsBuilder<const WIDTH: usize> {
    slots: NameSlotsBuilder,
    rows: SavedWordsBuilder<WIDTH>,
    texts: Vec<u8>,
    count: usize,
}

impl<const WIDTH: usize> SavedTextsBuilder<WIDTH> {
    pub(super) fn for_count(count: usize) -> Self {
        Self {
            slots: NameSlotsBuilder::for_count(count),
            rows: SavedWordsBuilder::default(),
            texts: Vec::new(),
            count: 0,
        }
    }

    pub(super) fn add<'a>(&mut self, key: impl SavedKey<'a>, mut row: [usize; WIDTH]) {
        row[SavedTexts::<WIDTH>::TEXT_START] = self.texts.len();
        self.texts.extend(key.text().as_bytes());
        row[SavedTexts::<WIDTH>::TEXT_END] = self.texts.len();
        self.rows.push(row);
        self.slots.claim(key, self.count);
        self.count += 1;
    }
}

impl<const WIDTH: usize> serde::Serialize for SavedTextsBuilder<WIDTH> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (&self.slots.words(), &self.rows, &self.texts).serialize(serializer)
    }
}
