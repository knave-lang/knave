pub struct CopyTrait;

impl CopyTrait {
    pub const PATH: &'static str = "Copy";
    pub const BROUGHT_METHODS: [&'static str; 2] = ["clone", "clone_from"];
}

pub struct OrderTrait;

impl OrderTrait {
    pub const PATH: &'static str = "Ord";
}

pub struct EqualityTrait;

impl EqualityTrait {
    pub const PATH: &'static str = "Eq";
}

pub struct HashTrait;

impl HashTrait {
    pub const PATH: &'static str = "std::hash::Hash";
}

pub struct BorrowTrait;

impl BorrowTrait {
    pub const PATH: &'static str = "std::borrow::Borrow";
}

pub struct ConsultedTraits;

impl ConsultedTraits {
    pub const BY_EVERY_PROGRAM: [&'static str; 1] = [CopyTrait::PATH];
    pub const BY_MAPS_AND_DERIVES: [&'static str; 4] = [
        OrderTrait::PATH,
        EqualityTrait::PATH,
        HashTrait::PATH,
        BorrowTrait::PATH,
    ];
}
