mod code_book;
mod failure;
mod joined_parts;
mod leading_lines;
mod lists;
mod numbers;
mod program;
mod step_limit;
mod strings;
mod text;
mod warning;
#[cfg(test)]
mod work_charges;

use std::cmp::Ordering;

use super::arguments::{Arguments, BuiltinSite};
use super::builtin_host::BuiltinHost;
use super::list_size::ListSize;
use super::{BuildLimit, Host, ListNesting, PlannedList, Value, WorkMeter};
use crate::core::{Builtin, Declaration};
use crate::diagnostic::Diagnostic;
use crate::reader::{Form, FormKind, Number, NumberValue};
use crate::source::Span;
use leading_lines::DistinctLeadingLines;
use lists::ListOperations;
use numbers::NumberOperations;
use program::{ModuleOperations, ProgramOperations};
use step_limit::StepLimitSetting;
use strings::StringOperations;
use text::TextOperations;
use warning::WarnCall;

pub struct BuiltinCall<'m> {
    builtin: Builtin,
    arguments: Arguments<'m>,
    call_site: Span,
}

impl<'m> BuiltinCall<'m> {
    const IMPLEMENTATIONS: [(Builtin, Implementation); 73] = [
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
            let group = host.granted(|host| host.name_comparison())?;
            operations.text.same_binding(group)
        }),
        (Builtin::IsPatternHead, |operations, host| {
            let group = host.granted(|host| host.name_comparison())?;
            operations.text.is_pattern_head(group)
        }),
        (Builtin::DefinitionSymbol, |operations, host| {
            let group = host.granted(|host| host.definition_queries())?;
            operations.text.definition_symbol(group)
        }),
        (Builtin::TypeDefinitionSymbol, |operations, host| {
            let group = host.granted(|host| host.definition_queries())?;
            operations.text.type_definition_symbol(group)
        }),
        (Builtin::DefinitionCall, |operations, host| {
            let group = host.granted(|host| host.definition_queries())?;
            operations.text.definition_call(group)
        }),
        (Builtin::TypeDefinitionCall, |operations, host| {
            let group = host.granted(|host| host.definition_queries())?;
            operations.text.type_definition_call(group)
        }),
        (Builtin::ExpandCall, |operations, host| {
            let group = host.granted(|host| host.expansion_callbacks())?;
            operations.text.expand_call(group)
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
        (Builtin::Warn, |operations, host| {
            let group = host.granted(|host| host.warnings())?;
            operations.warning.report(group)
        }),
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
            let group = host.granted(|host| host.module_notes())?;
            operations.text.add_module_note(group)
        }),
        (Builtin::ModuleNotes, |operations, host| {
            let group = host.granted(|host| host.module_notes())?;
            operations.text.module_notes(group)
        }),
        (Builtin::DeclareDefinition, |operations, host| {
            let group = host.granted(|host| host.expansion_callbacks())?;
            operations
                .text
                .declare_definition(group, Declaration::Value)
        }),
        (Builtin::DeclareTypeDefinition, |operations, host| {
            let group = host.granted(|host| host.expansion_callbacks())?;
            operations.text.declare_definition(group, Declaration::Type)
        }),
        (Builtin::DeclareMacroDefinition, |operations, host| {
            let group = host.granted(|host| host.expansion_callbacks())?;
            operations
                .text
                .declare_definition(group, Declaration::Macro)
        }),
        (Builtin::FormLocation, |operations, host| {
            let group = host.granted(|host| host.source_queries())?;
            operations.program.form_location(group)
        }),
        (Builtin::InTasksDirectory, |operations, host| {
            let group = host.granted(|host| host.source_queries())?;
            operations.program.in_tasks_directory(group)
        }),
        (Builtin::PinnedAt, |operations, _| {
            operations.program.pinned_at()
        }),
        (Builtin::ProgramModules, |operations, host| {
            let group = host.granted(|host| host.module_queries())?;
            operations.modules.program_modules(group)
        }),
        (Builtin::ModuleExports, |operations, host| {
            let group = host.granted(|host| host.module_queries())?;
            operations.modules.module_exports(group)
        }),
        (Builtin::ModuleDefinitions, |operations, host| {
            let group = host.granted(|host| host.module_queries())?;
            operations.modules.module_definitions(group)
        }),
        (Builtin::CurrentModule, |operations, host| {
            let group = host.granted(|host| host.module_queries())?;
            operations.modules.current_module(group)
        }),
        (Builtin::ModuleSymbol, |operations, host| {
            let group = host.granted(|host| host.module_queries())?;
            operations
                .modules
                .named(|path, span| group.module_symbol(path, span))
        }),
        (Builtin::RequireModule, |operations, host| {
            let group = host.granted(|host| host.expansion_callbacks())?;
            operations
                .modules
                .named(|path, span| group.require_module(path, span))
        }),
        (Builtin::SetMacroStepLimit, |operations, host| {
            let group = host.granted(|host| host.expansion_callbacks())?;
            operations
                .step_limit
                .apply(|limit| group.set_macro_step_limit(limit))
        }),
        (Builtin::ProjectFileName, |operations, _| {
            operations.program.project_file_name()
        }),
        (Builtin::ReadForm, |operations, host| {
            let group = host.granted(|host| host.source_queries())?;
            operations.program.read_form(group)
        }),
        (Builtin::Located, |operations, _| {
            operations.program.located()
        }),
        (Builtin::ProjectModules, |operations, host| {
            let group = host.granted(|host| host.module_queries())?;
            operations.modules.project_modules(group)
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

    pub fn run(self, host: &mut dyn Host) -> Result<Value, Diagnostic> {
        let operations = Operations::new(&self.arguments, self.call_site);
        Self::implementation(self.builtin)(&operations, BuiltinHost::new(host, &self.arguments))
    }

    fn implementation(builtin: Builtin) -> Implementation {
        let (_, implementation) = Self::IMPLEMENTATIONS[builtin.position()];
        implementation
    }
}

type Implementation =
    for<'a, 'h> fn(&Operations<'a>, BuiltinHost<'h, 'a>) -> Result<Value, Diagnostic>;

struct Operations<'a> {
    lists: ListOperations<'a>,
    modules: ModuleOperations<'a>,
    numbers: NumberOperations<'a>,
    program: ProgramOperations<'a>,
    step_limit: StepLimitSetting<'a>,
    strings: StringOperations<'a>,
    leading_lines: DistinctLeadingLines<'a>,
    text: TextOperations<'a>,
    warning: WarnCall<'a>,
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
            warning: WarnCall::new(arguments),
        }
    }
}

struct Forms;

impl Forms {
    fn list(items: Vec<Form>, span: Span, arguments: &Arguments) -> Result<Value, Diagnostic> {
        arguments.charge(items.len())?;
        BuildLimit::of(arguments.function(), arguments.span())
            .check(PlannedList(ListSize::of_items(&items)))?;
        ListNesting::form_list(items, span).map_err(|deep| deep.at(arguments.span()))
    }

    fn string(text: String, span: Span, arguments: &Arguments) -> Result<Value, Diagnostic> {
        let text = BuildLimit::of(arguments.function(), arguments.span()).check(text)?;
        Ok(Value::Form(Form::new(FormKind::String(text.into()), span)))
    }

    fn grown_string(text: String, span: Span, arguments: &Arguments) -> Result<Value, Diagnostic> {
        arguments.charge(WorkMeter::text_work(text.len()))?;
        Self::string(text, span, arguments)
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
    use crate::interpret::value_limits::ValueLimits;
    use crate::interpret::{StepBudget, WorkMeter};
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
            let leaves = vec![leaf; ValueLimits::LIST_VALUES / Self::COPIES];
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
