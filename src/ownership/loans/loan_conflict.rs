use super::lending::Lending;
use super::loan_holder::LoanHolder;
use super::place_access::PlaceAccess;
use crate::source::Span;

pub struct LoanConflict<'b> {
    access: &'b PlaceAccess,
    lending: &'b Lending,
    holder: LoanHolder,
    later_use: Option<Span>,
}

impl<'b> LoanConflict<'b> {
    pub fn found(
        access: &'b PlaceAccess,
        lending: &'b Lending,
        holder: LoanHolder,
    ) -> Option<Self> {
        let excludes = lending.is_exclusive() || access.access().excludes_shared_loans();
        let conflict = Self {
            access,
            lending,
            holder,
            later_use: None,
        };
        (excludes && lending.place().overlaps(access.place())).then_some(conflict)
    }

    pub fn used_later_at(self, later_use: Option<Span>) -> Self {
        Self { later_use, ..self }
    }

    pub fn access(&self) -> &'b PlaceAccess {
        self.access
    }

    pub fn lending(&self) -> &'b Lending {
        self.lending
    }

    pub fn holder(&self) -> LoanHolder {
        self.holder
    }

    pub fn later_use(&self) -> Option<Span> {
        self.later_use
    }
}
