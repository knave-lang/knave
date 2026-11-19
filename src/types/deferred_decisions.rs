use super::open_set_at::OpenSetAt;
use super::parameter_call_request::ParameterCallRequest;
use super::pending::PendingBorrow;
use super::reopened_parameter::ReopenedParameter;
use super::ty::Type;
use super::{PendingModeCheck, TypedConversion};
use crate::source::Span;

#[derive(Default)]
pub(super) struct DeferredDecisions {
    pub(super) borrows: Vec<PendingBorrow>,
    pub(super) open_set_ats: Vec<OpenSetAt>,
    pub(super) unsettled_collections: Vec<(Type, Span)>,
    pub(super) reopened_parameters: Vec<ReopenedParameter>,
    pub(super) label_requests: Vec<ParameterCallRequest>,
    pub(super) mode_checks: Vec<PendingModeCheck>,
    pub(super) conversions: Vec<TypedConversion>,
}
