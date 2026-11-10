use super::leading_lines::DistinctLeadingLines;
use super::lists::ListOperations;
use super::strings::StringOperations;
use super::text::TextOperations;
use crate::diagnostic::Diagnostic;
use crate::interpret::arguments::{Arguments, BuiltinSite};
use crate::interpret::{CodeList, StepBudget, Value, WorkMeter};
use crate::reader::{Form, FormKind, Symbol};
use crate::source::{SourceId, Span};

struct WorkFixture;

impl WorkFixture {
    const MOST_WORK: u64 = 4_096;
    const FUNCTION: &'static str = "built-in";
    const SIXTEEN_BYTES: &'static str = "abcdefghijklmnop";

    fn span() -> Span {
        Span::new(SourceId::new(0), 0..0)
    }

    fn symbol(name: &str) -> Value {
        Value::Form(Form::new(FormKind::Symbol(Symbol::new(name)), Self::span()))
    }

    fn string(text: &str) -> Value {
        Value::Form(Form::new(FormKind::String(text.into()), Self::span()))
    }

    fn built_list(items: Vec<Value>) -> Value {
        Value::List(CodeList::new(items, Self::span()).expect("within the limit"))
    }

    fn read_list(count: usize) -> Value {
        let leaf = Form::new(FormKind::Symbol(Symbol::new("s")), Self::span());
        Value::Form(Form::new(FormKind::List(vec![leaf; count]), Self::span()))
    }

    fn arguments(values: &[Value], limit: u64) -> Arguments {
        let site = BuiltinSite {
            function: Self::FUNCTION,
            span: Self::span(),
        };
        let budget = StepBudget::full();
        budget.set_limit(limit);
        Arguments::new(site, values.to_vec(), WorkMeter::new(budget, Self::span()))
    }

    fn work(values: &[Value], run: impl Fn(&Arguments) -> Result<Value, Diagnostic>) -> u64 {
        (0..=Self::MOST_WORK)
            .find(|limit| run(&Self::arguments(values, *limit)).is_ok())
            .expect("the work fits the largest limit")
    }

    fn list_work(
        values: &[Value],
        run: impl for<'a> Fn(&ListOperations<'a>) -> Result<Value, Diagnostic>,
    ) -> u64 {
        Self::work(values, |arguments| {
            run(&ListOperations::new(arguments, Self::span()))
        })
    }

    fn text_work(
        values: &[Value],
        run: impl for<'a> Fn(&TextOperations<'a>) -> Result<Value, Diagnostic>,
    ) -> u64 {
        Self::work(values, |arguments| {
            run(&TextOperations::new(arguments, Self::span()))
        })
    }

    fn string_work(
        values: &[Value],
        run: impl for<'a> Fn(&StringOperations<'a>) -> Result<Value, Diagnostic>,
    ) -> u64 {
        Self::work(values, |arguments| {
            run(&StringOperations::new(arguments, Self::span()))
        })
    }

    fn leaves(count: usize) -> Vec<Value> {
        vec![Self::symbol("s"); count]
    }
}

#[test]
fn list_charges_one_step_per_argument() {
    assert_eq!(
        WorkFixture::list_work(&WorkFixture::leaves(3), |lists| lists.list()),
        3
    );
}

#[test]
fn prepend_charges_every_copied_item_and_the_new_one() {
    let values = [WorkFixture::symbol("s"), WorkFixture::read_list(4)];
    assert_eq!(WorkFixture::list_work(&values, |lists| lists.prepend()), 5);
}

#[test]
fn concat_at_the_end_of_a_buffer_charges_only_the_added_items() {
    let head = WorkFixture::built_list(WorkFixture::leaves(10));
    let values = [head, WorkFixture::read_list(3)];
    assert_eq!(WorkFixture::list_work(&values, |lists| lists.concat()), 3);
}

#[test]
fn concat_that_copies_its_head_charges_the_copy() {
    let values = [WorkFixture::read_list(10), WorkFixture::read_list(3)];
    assert_eq!(WorkFixture::list_work(&values, |lists| lists.concat()), 13);
}

#[test]
fn a_string_argument_charges_one_step_per_sixteen_bytes() {
    let text = WorkFixture::SIXTEEN_BYTES.repeat(2);
    assert_eq!(
        WorkFixture::string_work(&[WorkFixture::string(&text)], |strings| strings.length()),
        2
    );
}

#[test]
fn a_string_of_one_byte_more_than_a_step_charges_another() {
    let text = format!("{}x", WorkFixture::SIXTEEN_BYTES);
    assert_eq!(
        WorkFixture::string_work(&[WorkFixture::string(&text)], |strings| strings.length()),
        2
    );
}

#[test]
fn a_symbol_name_charges_by_its_bytes() {
    let name = WorkFixture::SIXTEEN_BYTES.repeat(3);
    assert_eq!(
        WorkFixture::text_work(&[WorkFixture::symbol(&name)], |text| text.symbol_name()),
        3
    );
}

#[test]
fn a_keyword_name_charges_by_its_bytes() {
    let keyword = format!(":{}", WorkFixture::SIXTEEN_BYTES.repeat(2));
    assert_eq!(
        WorkFixture::text_work(&[WorkFixture::symbol(&keyword)], |text| text.keyword_name()),
        2
    );
}

#[test]
fn a_keyword_field_charges_by_its_bytes() {
    let keyword = format!(":{}", WorkFixture::SIXTEEN_BYTES.repeat(2));
    assert_eq!(
        WorkFixture::text_work(&[WorkFixture::symbol(&keyword)], |text| text
            .keyword_field()),
        2
    );
}

#[test]
fn a_field_keyword_charges_by_its_bytes() {
    let field = WorkFixture::SIXTEEN_BYTES.repeat(2);
    let work = WorkFixture::text_work(&[WorkFixture::symbol(&field)], |text| text.field_keyword());
    assert_eq!(work, 3);
}

#[test]
fn joining_parts_charges_each_part() {
    let part = WorkFixture::string(&WorkFixture::SIXTEEN_BYTES.repeat(2));
    let values = [part.clone(), part];
    assert_eq!(
        WorkFixture::text_work(&values, |text| text.make_string()),
        4
    );
}

#[test]
fn value_eq_charges_one_step_per_form_and_the_bytes_of_each_symbol() {
    let values = [WorkFixture::read_list(4), WorkFixture::read_list(4)];
    assert_eq!(
        WorkFixture::text_work(&values, |text| text.value_eq()),
        1 + 4 * 2
    );
}

#[test]
fn value_eq_charges_the_bytes_of_equal_strings() {
    let text = WorkFixture::string(&WorkFixture::SIXTEEN_BYTES.repeat(4));
    let values = [text.clone(), text];
    assert_eq!(
        WorkFixture::text_work(&values, |text| text.value_eq()),
        1 + 4
    );
}

#[test]
fn value_eq_charges_the_forms_of_a_built_list() {
    let values = [
        WorkFixture::built_list(WorkFixture::leaves(3)),
        WorkFixture::symbol("s"),
    ];
    assert_eq!(
        WorkFixture::text_work(&values, |text| text.value_eq()),
        4 + 1
    );
}

#[test]
fn value_eq_of_two_built_lists_charges_both_conversions() {
    let list = WorkFixture::built_list(WorkFixture::leaves(2));
    let values = [list.clone(), list];
    assert_eq!(
        WorkFixture::text_work(&values, |text| text.value_eq()),
        3 + 3 + 1 + 2 * 2
    );
}

#[test]
fn a_form_argument_charges_only_when_converted_from_a_built_list() {
    let read = WorkFixture::read_list(4);
    assert_eq!(
        WorkFixture::work(&[read], |args| args.form(0).map(Value::Form)),
        0
    );
    let built = WorkFixture::built_list(WorkFixture::leaves(4));
    assert_eq!(
        WorkFixture::work(&[built], |args| args.form(0).map(Value::Form)),
        5
    );
}

#[test]
fn sorted_names_charges_forms_bytes_and_the_new_list() {
    let values = [WorkFixture::read_list(3)];
    assert_eq!(
        WorkFixture::text_work(&values, |text| text.sorted_names()),
        3 + 1 + 1
    );
}

#[test]
fn closest_name_charges_the_product_of_the_name_and_candidate_bytes() {
    let values = [WorkFixture::symbol("ab"), WorkFixture::read_list(3)];
    assert_eq!(
        WorkFixture::text_work(&values, |text| text.closest_name()),
        1 + 3 + 2 * 3
    );
}

#[test]
fn distinct_leading_lines_charges_each_text() {
    let text = WorkFixture::string(&format!("{}\nx", WorkFixture::SIXTEEN_BYTES));
    let forms = Value::Form(Form::new(
        FormKind::List(vec![text.as_form().cloned().expect("a form")]),
        WorkFixture::span(),
    ));
    let work = WorkFixture::work(&[forms], |arguments| {
        DistinctLeadingLines::new(arguments, WorkFixture::span()).kept_texts()
    });
    assert_eq!(work, 1 + 3);
}
