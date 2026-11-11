use super::arguments::BuiltinSite;
use super::host::Host;
use super::host_problems::HostProblems;
use crate::diagnostic::Diagnostic;

pub(super) struct BuiltinHost<'h> {
    host: &'h mut dyn Host,
    site: BuiltinSite,
}

impl<'h> BuiltinHost<'h> {
    pub(super) fn new(host: &'h mut dyn Host, site: BuiltinSite) -> Self {
        Self { host, site }
    }

    pub(super) fn granted<Group: ?Sized>(
        self,
        accessor: impl FnOnce(&'h mut dyn Host) -> Option<&'h mut Group>,
    ) -> Result<&'h mut Group, Diagnostic> {
        accessor(self.host).ok_or_else(|| {
            HostProblems::missing_group_of_builtin(self.site.function, self.site.span)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::BuiltinHost;
    use crate::diagnostic::{Diagnostic, DiagnosticCode};
    use crate::interpret::arguments::BuiltinSite;
    use crate::interpret::partial_host::PartialHost;
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
            BuiltinHost::new(&mut host, Self::site())
                .granted(|host| host.warnings())
                .map(drop)
        }
    }

    #[test]
    fn a_group_the_host_offers_is_granted() {
        assert!(BuiltinHostFixture::warnings_granted(PartialHost::offering_warnings()).is_ok());
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
