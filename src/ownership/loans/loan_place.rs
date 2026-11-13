use std::rc::Rc;

use super::loan_names::LoanNames;
use crate::core::BindingId;
use crate::types::TypedExpr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoanPlace {
    root: BindingId,
    fields: Vec<Rc<str>>,
}

impl LoanPlace {
    pub fn of(expr: &TypedExpr) -> Option<Self> {
        let place = expr.field_place()?;
        Some(Self {
            root: place.root,
            fields: place.path.iter().map(|label| label.name.clone()).collect(),
        })
    }

    pub fn written(&self, names: &LoanNames<'_>) -> String {
        let mut written = String::from(names.name(self.root));
        for field in &self.fields {
            written.push('.');
            written.push_str(field);
        }
        written
    }
}

#[cfg(test)]
mod tests {
    use super::LoanPlace;
    use crate::ownership::lambda_source::LambdaSource;
    use crate::ownership::loans::loan_names::LoanNames;
    use crate::source::SourceMap;

    #[test]
    fn a_field_read_is_a_place_written_from_its_root_and_a_call_is_not() {
        let source = LambdaSource::new(
            "(defstruct car :make string :year i64)\n\n(defn main ()\n  (local c (->car :make (str \"a\") :year 1))\n  (local m (:make c))\n  (local n (len (:make c)))\n  (println m n))",
        );
        let field = source.bound("m");
        let place = LoanPlace::of(field).expect("a field read is a place");
        let sources = SourceMap::default();
        let names = LoanNames::new(&source.program().bindings, &sources);
        assert_eq!(place.written(&names), "c.make");
        assert_eq!(LoanPlace::of(source.bound("n")), None);
    }
}
