pub struct SetAsideInput<'a> {
    pub names: &'a [&'a str],
    pub source: &'a str,
    pub setter: &'a str,
}
