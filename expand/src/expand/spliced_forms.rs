use std::collections::VecDeque;

use crate::core::Provenance;
use crate::reader::Form;

#[derive(Default)]
pub struct SplicedForms {
    forms: VecDeque<SplicedForm>,
    taken_splices: usize,
}

pub struct SplicedForm {
    form: Form,
    splices: usize,
    provenance: Provenance,
}

impl SplicedForm {
    pub fn written(form: Form) -> Self {
        Self {
            form,
            splices: 0,
            provenance: Provenance::Written,
        }
    }

    pub fn form(&self) -> &Form {
        &self.form
    }

    pub fn nests_deeper_than(&self, limit: usize) -> bool {
        self.splices > limit
    }

    pub fn expanded_by<Failure>(
        self,
        expand: impl FnOnce(Form, &mut dyn FnMut()) -> Result<Form, Failure>,
    ) -> Result<Self, Failure> {
        let mut provenance = self.provenance;
        let form = expand(self.form, &mut || provenance = Provenance::ThroughMacroCall)?;
        Ok(Self {
            form,
            provenance,
            ..self
        })
    }

    pub fn registered_by<Item, Failure>(
        self,
        register: impl FnOnce(Form) -> Result<Option<Item>, Failure>,
        mark: impl FnOnce(Item, Provenance) -> Item,
    ) -> Result<Option<Item>, Failure> {
        let provenance = self.provenance;
        Ok(register(self.form)?.map(|item| mark(item, provenance)))
    }

    pub fn items(&self) -> &[Form] {
        let items = self.form.as_list().unwrap_or_default();
        items.get(1..).unwrap_or_default()
    }
}

impl SplicedForms {
    pub fn splice_items_of(&mut self, spliced: &SplicedForm) {
        let (splices, provenance) = (self.taken_splices + 1, spliced.provenance);
        for form in spliced.items().iter().rev() {
            let form = form.clone();
            self.forms.push_front(SplicedForm {
                form,
                splices,
                provenance,
            });
        }
    }

    pub fn take_first(&mut self) -> Option<SplicedForm> {
        let first = self.forms.pop_front()?;
        self.taken_splices = first.splices;
        Some(first)
    }
}
