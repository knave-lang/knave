pub struct UnlistedLibrary<'t> {
    pub name: &'t str,
    pub lister: Option<&'t str>,
    pub section: &'t str,
}
