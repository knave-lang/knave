pub struct CopyTrait;

impl CopyTrait {
    pub const PATH: &'static str = "Copy";
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
    pub const PATHS: [&'static str; 5] = [
        CopyTrait::PATH,
        OrderTrait::PATH,
        EqualityTrait::PATH,
        HashTrait::PATH,
        BorrowTrait::PATH,
    ];
}
