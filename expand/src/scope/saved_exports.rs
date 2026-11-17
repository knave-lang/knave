use std::rc::Rc;

use super::name_slots::NameSlots;
use crate::core::{BindingId, Namespace};

pub type ExportParts = (&'static [u8], &'static [u8], &'static [u8]);

#[derive(Clone, Copy)]
struct ExportRow {
    name_start: u32,
    name_end: u32,
    namespace: u32,
    binding: u32,
}

impl ExportRow {
    const WORDS: usize = 4;
    const SIZE: usize = Self::WORDS * NameSlots::WORD;

    fn read(bytes: &[u8]) -> Self {
        let word = |index: usize| NameSlots::word(bytes, index);
        Self {
            name_start: word(0),
            name_end: word(1),
            namespace: word(2),
            binding: word(3),
        }
    }

    fn write(self, bytes: &mut Vec<u8>) {
        let words = [self.name_start, self.name_end, self.namespace, self.binding];
        for word in words {
            bytes.extend(word.to_le_bytes());
        }
    }
}

#[derive(serde::Serialize)]
pub struct ExportBytes {
    slots: Vec<u8>,
    rows: Vec<u8>,
    names: Vec<u8>,
}

impl ExportBytes {
    pub fn of<'a>(exports: impl ExactSizeIterator<Item = (&'a str, Namespace, BindingId)>) -> Self {
        let mut bytes = Self {
            slots: NameSlots::empty(exports.len()),
            rows: Vec::new(),
            names: Vec::new(),
        };
        for (position, (name, namespace, binding)) in exports.enumerate() {
            bytes.add(name, namespace, binding);
            NameSlots::claim(&mut bytes.slots, name, namespace, position);
        }
        bytes
    }

    fn add(&mut self, name: &str, namespace: Namespace, binding: BindingId) {
        let name_start = NameSlots::index(self.names.len());
        self.names.extend(name.as_bytes());
        let row = ExportRow {
            name_start,
            name_end: NameSlots::index(self.names.len()),
            namespace: NameSlots::code(namespace),
            binding: binding.index(),
        };
        row.write(&mut self.rows);
    }
}

#[derive(Clone, Copy)]
pub struct SavedExports {
    slots: &'static [u8],
    rows: &'static [u8],
    names: &'static [u8],
}

impl SavedExports {
    pub fn of((slots, rows, names): ExportParts) -> Self {
        Self { slots, rows, names }
    }

    pub fn get(&self, name: &str, namespace: Namespace) -> Option<BindingId> {
        self.position(name, namespace)
            .map(|position| self.binding(position))
    }

    pub fn entries(&self) -> impl Iterator<Item = (Rc<str>, Namespace, BindingId)> + '_ {
        (0..self.count()).map(|position| {
            let row = self.row(position);
            let namespace = NameSlots::namespace(row.namespace);
            (
                Rc::from(self.name_of(row)),
                namespace,
                self.binding(position),
            )
        })
    }

    pub fn encoded(&self) -> ExportBytes {
        ExportBytes {
            slots: self.slots.to_vec(),
            rows: self.rows.to_vec(),
            names: self.names.to_vec(),
        }
    }

    pub(super) fn count(&self) -> usize {
        self.rows.len() / ExportRow::SIZE
    }

    pub(super) fn position(&self, name: &str, namespace: Namespace) -> Option<usize> {
        NameSlots::find(self.slots, name, namespace, |position| {
            self.holds_key(position, name, namespace)
        })
    }

    pub(super) fn binding(&self, position: usize) -> BindingId {
        BindingId::from_index(self.row(position).binding)
    }

    pub(super) fn key(&self, position: usize) -> (&'static str, Namespace) {
        let row = self.row(position);
        (self.name_of(row), NameSlots::namespace(row.namespace))
    }

    pub(super) fn keys(&self) -> impl Iterator<Item = (&'static str, Namespace)> + '_ {
        (0..self.count()).map(|position| self.key(position))
    }

    fn row(&self, position: usize) -> ExportRow {
        ExportRow::read(&self.rows[position * ExportRow::SIZE..])
    }

    fn holds_key(&self, position: usize, name: &str, namespace: Namespace) -> bool {
        let row = self.row(position);
        row.namespace == NameSlots::code(namespace) && self.name_bytes(row) == name.as_bytes()
    }

    fn name_bytes(&self, row: ExportRow) -> &'static [u8] {
        &self.names[row.name_start as usize..row.name_end as usize]
    }

    fn name_of(&self, row: ExportRow) -> &'static str {
        std::str::from_utf8(self.name_bytes(row)).expect("a saved export name is text")
    }
}
