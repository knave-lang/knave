mod borrowing;
mod capabilities;
mod conditional;
mod dereference;
mod expression_form;
#[cfg(test)]
mod form_fixtures;
mod if_operands;
mod variant_construction;

pub(super) use borrowing::Borrowing;
pub(super) use capabilities::{
    ChildTyping, ExpectedChildTyping, MacroOrigins, Obligations, PlaceReads, TypeFitting, Unknowns,
    VariantSites,
};
pub(super) use conditional::Conditional;
pub(super) use dereference::Dereference;
pub(super) use expression_form::ExpressionForm;
pub(super) use if_operands::IfOperands;
pub(super) use variant_construction::VariantConstruction;
