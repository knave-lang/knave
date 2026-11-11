use crate::reader::Symbol;

pub trait NameComparison {
    fn same_binding(&self, left: &Symbol, right: &Symbol) -> bool;
    fn is_pattern_head(&self, head: &Symbol) -> bool;
}
