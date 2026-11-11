use crate::reader::Form;

pub(super) struct BinderForm<'f> {
    pub name: &'f Form,
    pub annotation: Option<&'f Form>,
}

impl<'f> BinderForm<'f> {
    pub(super) fn of(binder: &'f Form) -> Self {
        match binder.as_list() {
            Some([name, annotation]) if name.as_symbol().is_some() => Self {
                name,
                annotation: Some(annotation),
            },
            _ => Self {
                name: binder,
                annotation: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::Reader;
    use crate::source::SourceMap;

    struct BinderFixture {
        sources: SourceMap,
    }

    impl BinderFixture {
        fn new() -> Self {
            Self {
                sources: SourceMap::default(),
            }
        }

        fn form(&mut self, text: &str) -> Form {
            let source = self.sources.add("binder", text);
            let forms = Reader::new(self.sources.file(source)).read();
            let mut forms = forms.unwrap_or_else(|_| panic!("`{text}` reads"));
            forms.remove(0)
        }

        fn parts(form: &Form) -> (String, Option<String>) {
            let binder = BinderForm::of(form);
            (
                binder.name.to_string(),
                binder.annotation.map(ToString::to_string),
            )
        }
    }

    #[test]
    fn a_bare_name_is_the_whole_binder_without_an_annotation() {
        let form = BinderFixture::new().form("x");
        assert_eq!(BinderFixture::parts(&form), ("x".to_string(), None));
    }

    #[test]
    fn a_name_and_a_type_split_into_the_name_and_its_annotation() {
        let form = BinderFixture::new().form("(x i64)");
        assert_eq!(
            BinderFixture::parts(&form),
            ("x".to_string(), Some("i64".to_string()))
        );
    }

    #[test]
    fn a_pair_that_does_not_start_with_a_name_is_kept_whole() {
        let form = BinderFixture::new().form("((x) i64)");
        assert_eq!(BinderFixture::parts(&form), ("((x) i64)".to_string(), None));
    }
}
