use crate::core::ModuleId;
use crate::interpret::TooManyNotes;
use crate::reader::Form;

use super::saved_notes::{NoteTables, NotesKey, NotesTable, SavedNotes};

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct ModuleNotes {
    #[serde(skip)]
    saved: SavedNotes,
    notes: NotesTable,
    added: Vec<NotesKey>,
}

#[derive(Clone, Copy)]
pub struct NotesCheckpoint(usize);

impl ModuleNotes {
    pub const LIMIT: usize = 1 << 20;

    pub fn add(&mut self, module: ModuleId, key: &str, note: Form) -> Result<(), TooManyNotes> {
        if self.added.len() == Self::LIMIT {
            return Err(TooManyNotes { limit: Self::LIMIT });
        }
        let notes_key = (module, key.to_string());
        self.notes.entry(notes_key.clone()).or_default().push(note);
        self.added.push(notes_key);
        Ok(())
    }

    pub fn holds_any(&self, module: ModuleId) -> bool {
        self.notes
            .iter()
            .any(|((noted_module, _), notes)| *noted_module == module && !notes.is_empty())
    }

    pub fn take_saved(&mut self) -> NoteTables {
        let tables = NoteTables::encode(&self.notes);
        *self = Self::default();
        tables
    }

    pub fn adopt(&mut self, saved: SavedNotes) {
        self.saved = saved;
    }

    pub fn of(&self, module: ModuleId, key: &str) -> Vec<Form> {
        self.notes
            .get(&(module, key.to_string()))
            .or_else(|| self.saved.of(module, key))
            .cloned()
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub fn holds_no_unsaved_note(&self) -> bool {
        self.notes.is_empty() && self.added.is_empty()
    }

    #[cfg(test)]
    pub fn decoded_no_saved_note(&self) -> bool {
        self.saved.decoded_nothing()
    }

    pub fn checkpoint(&self) -> NotesCheckpoint {
        NotesCheckpoint(self.added.len())
    }

    pub fn roll_back(&mut self, checkpoint: NotesCheckpoint) {
        for notes_key in self.added.split_off(checkpoint.0).into_iter().rev() {
            self.notes
                .get_mut(&notes_key)
                .expect("every added key holds its notes")
                .pop();
        }
    }
}
