mod failure;
mod joined_parts;
mod leading_lines;
mod lists;
mod numbers;
mod program;
mod step_limit;
mod strings;
mod text;
#[cfg(test)]
mod work_charges;

pub use failure::FailureDescription;

use std::cmp::Ordering;

use super::arguments::{Arguments, BuiltinSite};
use super::code_list::ListSize;
use super::{BuildLimit, CompileTimeHost, ListNesting, PlannedList, Value, WorkMeter};
use crate::core::{Builtin, Declaration};
use crate::diagnostic::Diagnostic;
use crate::reader::{Form, FormCount, FormKind, Number, NumberValue};
use crate::source::Span;
use leading_lines::DistinctLeadingLines;
use lists::ListOperations;
use numbers::NumberOperations;
use program::{ModuleOperations, ProgramOperations};
use step_limit::StepLimitSetting;
use strings::StringOperations;
use text::TextOperations;

pub struct BuiltinCall<'m> {
    builtin: Builtin,
    arguments: Arguments<'m>,
    call_site: Span,
}

impl<'m> BuiltinCall<'m> {
    const IMPLEMENTATIONS: [(Builtin, Implementation); 72] = [
        (Builtin::First, |operations, _| operations.lists.first()),
        (Builtin::Rest, |operations, _| operations.lists.rest()),
        (Builtin::List, |operations, _| operations.lists.list()),
        (Builtin::Prepend, |operations, _| operations.lists.prepend()),
        (Builtin::Concat, |operations, _| operations.lists.concat()),
        (Builtin::Len, |operations, _| operations.lists.len()),
        (Builtin::IsEmpty, |operations, _| {
            operations.lists.is_empty()
        }),
        (Builtin::Nth, |operations, _| operations.lists.nth()),
        (Builtin::Take, |operations, _| operations.lists.take()),
        (Builtin::Drop, |operations, _| operations.lists.drop()),
        (Builtin::IsSymbol, |operations, _| {
            operations
                .text
                .is_kind(|kind| matches!(kind, FormKind::Symbol(_)))
        }),
        (Builtin::IsKeyword, |operations, _| {
            operations.text.is_keyword()
        }),
        (Builtin::IsList, |operations, _| operations.text.is_list()),
        (Builtin::IsString, |operations, _| {
            operations
                .text
                .is_kind(|kind| matches!(kind, FormKind::String(_)))
        }),
        (Builtin::IsNumber, |operations, _| {
            operations
                .text
                .is_kind(|kind| matches!(kind, FormKind::Number(_)))
        }),
        (Builtin::SymbolName, |operations, _| {
            operations.text.symbol_name()
        }),
        (Builtin::KeywordName, |operations, _| {
            operations.text.keyword_name()
        }),
        (Builtin::KeywordField, |operations, _| {
            operations.text.keyword_field()
        }),
        (Builtin::FieldKeyword, |operations, _| {
            operations.text.field_keyword()
        }),
        (Builtin::KeywordTrimEnd, |operations, _| {
            operations.text.keyword_trim_end()
        }),
        (Builtin::MakeSymbol, |operations, _| {
            operations.text.make_symbol()
        }),
        (Builtin::MakeString, |operations, _| {
            operations.text.make_string()
        }),
        (Builtin::SameBinding, |operations, host| {
            operations.text.same_binding(host)
        }),
        (Builtin::IsPatternHead, |operations, host| {
            operations.text.is_pattern_head(host)
        }),
        (Builtin::DefinitionSymbol, |operations, host| {
            operations.text.definition_symbol(host)
        }),
        (Builtin::TypeDefinitionSymbol, |operations, host| {
            operations.text.type_definition_symbol(host)
        }),
        (Builtin::DefinitionCall, |operations, host| {
            operations.text.definition_call(host)
        }),
        (Builtin::TypeDefinitionCall, |operations, host| {
            operations.text.type_definition_call(host)
        }),
        (Builtin::ExpandCall, |operations, host| {
            operations.text.expand_call(host)
        }),
        (Builtin::ClosestName, |operations, _| {
            operations.text.closest_name()
        }),
        (Builtin::SortedNames, |operations, _| {
            operations.text.sorted_names()
        }),
        (Builtin::DistinctLeadingLines, |operations, _| {
            operations.leading_lines.kept_texts()
        }),
        (Builtin::Fail, |operations, _| operations.text.fail()),
        (Builtin::NumberAdd, |operations, _| operations.numbers.add()),
        (Builtin::NumberSub, |operations, _| {
            operations.numbers.subtract()
        }),
        (Builtin::NumberLt, |operations, _| {
            operations.numbers.compare(Ordering::is_lt)
        }),
        (Builtin::NumberLe, |operations, _| {
            operations.numbers.compare(Ordering::is_le)
        }),
        (Builtin::NumberGt, |operations, _| {
            operations.numbers.compare(Ordering::is_gt)
        }),
        (Builtin::NumberGe, |operations, _| {
            operations.numbers.compare(Ordering::is_ge)
        }),
        (Builtin::ValueEq, |operations, _| operations.text.value_eq()),
        (Builtin::BoolNot, |operations, _| operations.text.not()),
        (Builtin::EscapeString, |operations, _| {
            operations.text.escape_string()
        }),
        (Builtin::EscapeChar, |operations, _| {
            operations.text.escape_char()
        }),
        (Builtin::EscapeFormat, |operations, _| {
            operations.text.escape_format()
        }),
        (Builtin::StringStripSuffix, |operations, _| {
            operations.text.string_strip_suffix()
        }),
        (Builtin::StringStripPrefix, |operations, _| {
            operations.text.string_strip_prefix()
        }),
        (Builtin::StringLines, |operations, _| {
            operations.strings.lines()
        }),
        (Builtin::StringTrimStart, |operations, _| {
            operations.strings.trim_start()
        }),
        (Builtin::StringTrimEnd, |operations, _| {
            operations.strings.trim_end()
        }),
        (Builtin::StringLength, |operations, _| {
            operations.strings.length()
        }),
        (Builtin::StringDrop, |operations, _| {
            operations.strings.drop()
        }),
        (Builtin::StringLowercase, |operations, _| {
            operations.strings.lowercase()
        }),
        (Builtin::StringContains, |operations, _| {
            operations.strings.contains()
        }),
        (Builtin::AddModuleNote, |operations, host| {
            operations.text.add_module_note(host)
        }),
        (Builtin::ModuleNotes, |operations, host| {
            operations.text.module_notes(host)
        }),
        (Builtin::DeclareDefinition, |operations, host| {
            operations.text.declare_definition(host, Declaration::Value)
        }),
        (Builtin::DeclareTypeDefinition, |operations, host| {
            operations.text.declare_definition(host, Declaration::Type)
        }),
        (Builtin::DeclareMacroDefinition, |operations, host| {
            operations.text.declare_definition(host, Declaration::Macro)
        }),
        (Builtin::FormLocation, |operations, host| {
            operations.program.form_location(host)
        }),
        (Builtin::InTasksDirectory, |operations, host| {
            operations.program.in_tasks_directory(host)
        }),
        (Builtin::PinnedAt, |operations, _| {
            operations.program.pinned_at()
        }),
        (Builtin::ProgramModules, |operations, host| {
            operations.modules.program_modules(host)
        }),
        (Builtin::ModuleExports, |operations, host| {
            operations.modules.module_exports(host)
        }),
        (Builtin::ModuleDefinitions, |operations, host| {
            operations.modules.module_definitions(host)
        }),
        (Builtin::CurrentModule, |operations, host| {
            operations.modules.current_module(host)
        }),
        (Builtin::ModuleSymbol, |operations, host| {
            operations
                .modules
                .named(|path, span| host.module_symbol(path, span))
        }),
        (Builtin::RequireModule, |operations, host| {
            operations
                .modules
                .named(|path, span| host.require_module(path, span))
        }),
        (Builtin::SetMacroStepLimit, |operations, host| {
            operations
                .step_limit
                .apply(|limit| host.set_macro_step_limit(limit))
        }),
        (Builtin::ProjectFileName, |operations, _| {
            operations.program.project_file_name()
        }),
        (Builtin::ReadForm, |operations, host| {
            operations.program.read_form(host)
        }),
        (Builtin::Located, |operations, _| {
            operations.program.located()
        }),
        (Builtin::ProjectModules, |operations, host| {
            operations.modules.project_modules(host)
        }),
    ];

    pub fn new(builtin: Builtin, values: Vec<Value>, span: Span, meter: &'m WorkMeter) -> Self {
        let site = BuiltinSite {
            function: builtin.name(),
            span,
        };
        Self {
            builtin,
            arguments: Arguments::new(site, values, meter),
            call_site: meter.call_site(),
        }
    }

    pub fn run(self, host: &mut dyn CompileTimeHost) -> Result<Value, Diagnostic> {
        let operations = Operations::new(&self.arguments, self.call_site);
        Self::implementation(self.builtin)(&operations, host)
    }

    fn implementation(builtin: Builtin) -> Implementation {
        let (_, implementation) = Self::IMPLEMENTATIONS[builtin.position()];
        implementation
    }
}

type Implementation =
    for<'a> fn(&Operations<'a>, &mut dyn CompileTimeHost) -> Result<Value, Diagnostic>;

struct Operations<'a> {
    lists: ListOperations<'a>,
    modules: ModuleOperations<'a>,
    numbers: NumberOperations<'a>,
    program: ProgramOperations<'a>,
    step_limit: StepLimitSetting<'a>,
    strings: StringOperations<'a>,
    leading_lines: DistinctLeadingLines<'a>,
    text: TextOperations<'a>,
}

impl<'a> Operations<'a> {
    fn new(arguments: &'a Arguments<'a>, call_site: Span) -> Self {
        Self {
            lists: ListOperations::new(arguments, call_site),
            modules: ModuleOperations::new(arguments, call_site),
            numbers: NumberOperations::new(arguments, call_site),
            program: ProgramOperations::new(arguments, call_site),
            step_limit: StepLimitSetting::new(arguments),
            strings: StringOperations::new(arguments, call_site),
            leading_lines: DistinctLeadingLines::new(arguments, call_site),
            text: TextOperations::new(arguments, call_site),
        }
    }
}

struct Forms;

impl Forms {
    fn list(items: Vec<Form>, span: Span, arguments: &Arguments) -> Result<Value, Diagnostic> {
        arguments.charge(items.len())?;
        let forms = items.iter().map(FormCount::stored).sum::<usize>();
        BuildLimit::of(arguments).check(PlannedList(ListSize::EMPTY + forms))?;
        ListNesting::form_list(items, span).map_err(|deep| deep.at(arguments.span()))
    }

    fn count(count: usize, span: Span) -> Value {
        let number = Number::new(count.to_string(), NumberValue::Integer(count as i128));
        Value::Form(Form::new(FormKind::Number(number), span))
    }
}

#[cfg(test)]
mod tests {
    use super::{BuiltinCall, Forms};
    use crate::core::Builtin;
    use crate::diagnostic::DiagnosticCode;
    use crate::interpret::arguments::{Arguments, BuiltinSite};
    use crate::interpret::{BuildLimit, StepBudget, WorkMeter};
    use crate::reader::{Form, FormKind, Symbol};
    use crate::source::{SourceId, Span};

    struct WideForms;

    impl WideForms {
        const COPIES: usize = 16;

        fn span() -> Span {
            Span::new(SourceId::new(0), 0..1)
        }

        fn tree_past_a_sixteenth_of_the_limit() -> Form {
            let leaf = Form::new(FormKind::Symbol(Symbol::new("leaf")), Self::span());
            let leaves = vec![leaf; BuildLimit::LIST_SIZE / Self::COPIES];
            Form::new(FormKind::List(leaves), Self::span())
        }

        fn meter() -> WorkMeter {
            WorkMeter::new(StepBudget::unmetered(), Self::span())
        }

        fn arguments(meter: &WorkMeter) -> Arguments<'_> {
            let site = BuiltinSite {
                function: "module-notes",
                span: Self::span(),
            };
            Arguments::new(site, Vec::new(), meter)
        }
    }

    #[test]
    fn a_form_list_past_the_list_limit_is_refused() {
        let tree = WideForms::tree_past_a_sixteenth_of_the_limit();
        let items = vec![tree; WideForms::COPIES];
        let meter = WideForms::meter();
        let refusal = Forms::list(items, WideForms::span(), &WideForms::arguments(&meter))
            .expect_err("past the list limit");
        assert_eq!(refusal.code(), Some(&DiagnosticCode::MACRO_VALUE_SIZE));
    }

    #[test]
    fn each_builtin_finds_its_own_implementation_row() {
        for builtin in Builtin::all() {
            let (implemented, _) = BuiltinCall::IMPLEMENTATIONS[builtin.position()];
            assert_eq!(implemented, builtin);
        }
    }
}
