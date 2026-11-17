use crate::expand::saved_notes::NotesKey;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NotesCheckpoint(usize);

impl NotesCheckpoint {
    pub const START: Self = Self(0);
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct NotesLog {
    added: Vec<NotesKey>,
}

impl NotesLog {
    pub(super) fn record(&mut self, notes_key: NotesKey) {
        self.added.push(notes_key);
    }

    pub fn checkpoint(&self) -> NotesCheckpoint {
        NotesCheckpoint(self.added.len())
    }

    pub(super) fn count_since(&self, checkpoint: NotesCheckpoint, notes_key: &NotesKey) -> usize {
        self.added[checkpoint.0..]
            .iter()
            .filter(|added_key| *added_key == notes_key)
            .count()
    }

    pub(super) fn split_off(&mut self, checkpoint: NotesCheckpoint) -> Vec<NotesKey> {
        self.added.split_off(checkpoint.0)
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.added.is_empty()
    }
}
