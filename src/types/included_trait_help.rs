pub(super) struct IncludedTraitHelp;

impl IncludedTraitHelp {
    pub(super) fn generic_print(included: &str) -> String {
        format!(
            "a generic function prints only `{included}` types: give the parameter a type, or print the value with `prn`"
        )
    }
}
