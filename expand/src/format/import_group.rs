use super::block::Block;
use super::layout::LaidOut;
use super::syntax_tree::{Entry, SyntaxNode};
use crate::core::{DefinitionForm, KernelForm};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ImportGroup {
    Use,
    Export,
}

impl ImportGroup {
    pub fn of(entry: &Entry, block: &Block) -> Option<Self> {
        let SyntaxNode::List(list) = &entry.node else {
            return None;
        };
        let head = list.entries.first()?.node.symbol_name()?;
        (block.line_count() == 1).then_some(())?;
        [Self::Use, Self::Export]
            .into_iter()
            .find(|group| group.form_name() == head)
    }

    fn form_name(self) -> &'static str {
        let form = match self {
            Self::Use => DefinitionForm::Use,
            Self::Export => DefinitionForm::Export,
        };
        KernelForm::Definition(form).name()
    }
}
