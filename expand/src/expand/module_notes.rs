use crate::core::ModuleId;
use crate::interpret::{HeldBytes, NotesTooLarge};
use crate::reader::Form;

use super::saved_notes::{NoteTables, NotesKey, NotesTable, SavedNotes};

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct ModuleNotes {
    #[serde(skip)]
    saved: SavedNotes,
    notes: NotesTable,
    added: Vec<NotesKey>,
    #[serde(skip)]
    held_bytes: usize,
}

#[derive(Clone, Copy)]
pub struct NotesCheckpoint(usize);

impl ModuleNotes {
    pub const BYTE_LIMIT: usize = 1 << 28;

    pub fn add(&mut self, module: ModuleId, key: &str, note: Form) -> Result<(), NotesTooLarge> {
        let held_bytes = self.held_bytes + HeldBytes::of_form(&note);
        if held_bytes > Self::BYTE_LIMIT {
            return Err(NotesTooLarge {
                held_bytes,
                limit: Self::BYTE_LIMIT,
            });
        }
        let notes_key = (module, key.to_string());
        self.notes.entry(notes_key.clone()).or_default().push(note);
        self.added.push(notes_key);
        self.held_bytes = held_bytes;
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
            let removed = self
                .notes
                .get_mut(&notes_key)
                .and_then(Vec::pop)
                .expect("every added key holds its notes");
            self.held_bytes = self.held_bytes.saturating_sub(HeldBytes::of_form(&removed));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ModuleNotes;
    use crate::core::ModuleId;
    use crate::interpret::HeldBytes;
    use crate::reader::{Form, FormKind, Symbol};
    use crate::source::{SourceId, Span};

    struct NoteForms;

    impl NoteForms {
        fn span() -> Span {
            Span::new(SourceId::new(0), 0..1)
        }

        fn leaf() -> Form {
            Form::new(FormKind::Symbol(Symbol::new("leaf")), Self::span())
        }

        fn holding(bytes: usize) -> Form {
            let text = "x".repeat(bytes - HeldBytes::PER_FORM);
            Form::new(FormKind::String(text.into()), Self::span())
        }

        fn add(notes: &mut ModuleNotes, note: Form) -> bool {
            notes.add(ModuleId::new(0), "key", note).is_ok()
        }
    }

    #[test]
    fn notes_up_to_the_byte_limit_are_held() {
        let mut notes = ModuleNotes::default();
        let half = ModuleNotes::BYTE_LIMIT / 2;
        assert!(NoteForms::add(&mut notes, NoteForms::holding(half)));
        assert!(NoteForms::add(&mut notes, NoteForms::holding(half)));
        assert!(!NoteForms::add(&mut notes, NoteForms::leaf()));
    }

    #[test]
    fn rolling_back_a_note_frees_its_bytes() {
        let mut notes = ModuleNotes::default();
        let checkpoint = notes.checkpoint();
        let full = ModuleNotes::BYTE_LIMIT;
        assert!(NoteForms::add(&mut notes, NoteForms::holding(full)));
        notes.roll_back(checkpoint);
        assert!(NoteForms::add(&mut notes, NoteForms::holding(full)));
    }
}
