use super::Expander;
use super::shape::Shape;
use crate::core::{CoreModule, DefinitionForm, Item, ItemKind, KernelForm, MismatchHelp};
use crate::diagnostic::Diagnostic;
use crate::reader::Form;

pub(super) struct MismatchHelpForm<'e, 'x, 'f> {
    expander: &'e mut Expander<'x>,
    form: &'f Form,
}

impl<'e, 'x, 'f> MismatchHelpForm<'e, 'x, 'f> {
    const PARTS: usize = 4;

    pub(super) fn new(expander: &'e mut Expander<'x>, form: &'f Form) -> Self {
        Self { expander, form }
    }

    pub(super) fn lower(self, core: &mut CoreModule) -> Result<(), Diagnostic> {
        let shape = Shape::new(
            KernelForm::Definition(DefinitionForm::MismatchHelp),
            self.form,
        );
        let [_, expected, found, text] = shape.exactly(Self::PARTS)? else {
            unreachable!("the shape holds exactly its parts")
        };
        let help = MismatchHelp {
            expected: self.expander.lower_type(expected)?,
            found: self.expander.lower_type(found)?,
            text: shape.string(text)?.into(),
        };
        core.items.push(Item {
            kind: ItemKind::MismatchHelp(help),
            span: self.form.span(),
            hidden: false,
        });
        Ok(())
    }
}
