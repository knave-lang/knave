use super::arguments::Arguments;
use super::host::Host;
use super::host_problems::HostProblems;
use crate::diagnostic::Diagnostic;

pub(super) struct BuiltinHost<'h, 'a> {
    host: &'h mut dyn Host,
    call: &'a Arguments<'a>,
}

impl<'h, 'a> BuiltinHost<'h, 'a> {
    pub(super) fn new(host: &'h mut dyn Host, call: &'a Arguments<'a>) -> Self {
        Self { host, call }
    }

    pub(super) fn granted<Group: ?Sized>(
        self,
        accessor: impl FnOnce(&'h mut dyn Host) -> Option<&'h mut Group>,
    ) -> Result<&'h mut Group, Diagnostic> {
        accessor(self.host).ok_or_else(|| {
            HostProblems::missing_group_of_builtin(self.call.function(), self.call.span())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::BuiltinHost;
    use crate::diagnostic::{Diagnostic, DiagnosticCode};
    use crate::interpret::arguments::{Arguments, BuiltinSite};
    use crate::interpret::partial_host::PartialHost;
    use crate::interpret::{StepBudget, WorkMeter};
    use crate::source::{SourceId, Span};

    struct BuiltinHostFixture;

    impl BuiltinHostFixture {
        fn site() -> BuiltinSite {
            BuiltinSite {
                function: "warn",
                span: Span::new(SourceId::new(0), 3..9),
            }
        }

        fn warnings_granted(mut host: PartialHost) -> Result<(), Diagnostic> {
            let meter = WorkMeter::new(StepBudget::full(), Self::site().span);
            let call = Arguments::new(Self::site(), Vec::new(), &meter);
            BuiltinHost::new(&mut host, &call)
                .granted(|host| host.warnings())
                .map(drop)
        }
    }

    #[test]
    fn a_group_the_host_offers_is_granted() {
        assert!(BuiltinHostFixture::warnings_granted(PartialHost::offering_its_groups()).is_ok());
    }

    #[test]
    fn a_group_the_host_lacks_is_refused_at_the_call() {
        let refusal = BuiltinHostFixture::warnings_granted(PartialHost::offering_nothing())
            .expect_err("the host offers no warnings");
        assert_eq!(refusal.code(), Some(&DiagnosticCode::MISSING_HOST_GROUP));
        assert!(
            refusal
                .labels()
                .map(|label| label.span())
                .eq([BuiltinHostFixture::site().span])
        );
    }

    #[test]
    fn the_refusal_names_the_built_in() {
        let refusal = BuiltinHostFixture::warnings_granted(PartialHost::offering_nothing())
            .expect_err("the host offers no warnings");
        assert!(refusal.title().contains("`warn`"), "{}", refusal.title());
    }
}
