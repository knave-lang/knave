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
