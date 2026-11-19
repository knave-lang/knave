use std::rc::Rc;

use super::TypeExpr;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MismatchHelp {
    pub expected: TypeExpr,
    pub found: TypeExpr,
    pub text: Rc<str>,
}

impl MismatchHelp {
    pub fn printed(&self, mut type_expr: impl FnMut(&TypeExpr) -> String) -> String {
        let (expected, found) = (type_expr(&self.expected), type_expr(&self.found));
        format!("(mismatch-help {expected} {found} {:?})", self.text)
    }
}
