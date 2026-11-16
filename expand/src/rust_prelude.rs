pub struct RustPrelude;

impl RustPrelude {
    const TYPES: [&'static str; 5] = ["Box", "Option", "Result", "String", "Vec"];
    const TRAITS: [(&'static str, &'static str); 31] = [
        ("AsMut", "std::convert::AsMut"),
        ("AsRef", "std::convert::AsRef"),
        ("Clone", "std::clone::Clone"),
        ("Copy", "std::marker::Copy"),
        ("Default", "std::default::Default"),
        ("DoubleEndedIterator", "std::iter::DoubleEndedIterator"),
        ("Drop", "std::ops::Drop"),
        ("Eq", "std::cmp::Eq"),
        ("ExactSizeIterator", "std::iter::ExactSizeIterator"),
        ("Extend", "std::iter::Extend"),
        ("Fn", "std::ops::Fn"),
        ("FnMut", "std::ops::FnMut"),
        ("FnOnce", "std::ops::FnOnce"),
        ("From", "std::convert::From"),
        ("FromIterator", "std::iter::FromIterator"),
        ("Future", "std::future::Future"),
        ("Into", "std::convert::Into"),
        ("IntoFuture", "std::future::IntoFuture"),
        ("IntoIterator", "std::iter::IntoIterator"),
        ("Iterator", "std::iter::Iterator"),
        ("Ord", "std::cmp::Ord"),
        ("PartialEq", "std::cmp::PartialEq"),
        ("PartialOrd", "std::cmp::PartialOrd"),
        ("Send", "std::marker::Send"),
        ("Sized", "std::marker::Sized"),
        ("Sync", "std::marker::Sync"),
        ("ToOwned", "std::borrow::ToOwned"),
        ("ToString", "std::string::ToString"),
        ("TryFrom", "std::convert::TryFrom"),
        ("TryInto", "std::convert::TryInto"),
        ("Unpin", "std::marker::Unpin"),
    ];

    pub fn holds(name: &str) -> bool {
        Self::TYPES.contains(&name) || Self::trait_path(name).is_some()
    }

    pub fn trait_path(name: &str) -> Option<&'static str> {
        Self::TRAITS
            .iter()
            .find(|(trait_name, _)| *trait_name == name)
            .map(|(_, path)| *path)
    }
}
