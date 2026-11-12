use super::fragment::Code;
use super::preceded_code::PrecededCode;

pub enum Substitution {
    Inline(Vec<Code>),
    Bind(Vec<PrecededCode>),
}
