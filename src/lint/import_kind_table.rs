use crate::core::{BindingId, BindingKind, Builtin, KernelForm, KernelType, ModuleId, VariantInfo};
use crate::lint::expanded_source::ExpandedSource;

struct ImportKindTable;

struct ReadRow {
    way: &'static str,
    text: String,
}

struct UnreadRow {
    text: String,
    warns_on: Option<&'static str>,
}

impl ImportKindTable {
    const MODULE: &str = "tests/fixtures/lint/modules/every_kind";
    const MAIN: &str = "(defn main () (println 1))";

    fn every_kind() -> Vec<BindingKind> {
        let module = ModuleId::new(0);
        let variant = VariantInfo {
            owner: BindingId::from_index(0),
            index: 0,
            arity: 1,
        };
        vec![
            BindingKind::KernelForm(KernelForm::Macro),
            BindingKind::KernelType(KernelType::Hole),
            BindingKind::Boolean(true),
            BindingKind::CodeBuiltin(Builtin::MakeSymbol),
            BindingKind::Macro(module),
            BindingKind::Global(module),
            BindingKind::Local,
            BindingKind::Struct(module),
            BindingKind::Enum(module),
            BindingKind::Variant(module, variant),
            BindingKind::Trait(module),
            BindingKind::RustType(module),
            BindingKind::TypeVariable,
            BindingKind::AssociatedType,
            BindingKind::Lifetime,
        ]
    }

    fn reads_of(kind: BindingKind) -> Vec<ReadRow> {
        match kind {
            BindingKind::Global(_) | BindingKind::Macro(_) | BindingKind::CodeBuiltin(_) => {
                Self::value_reads(kind)
            }
            BindingKind::KernelForm(_) | BindingKind::KernelType(_) | BindingKind::Boolean(_) => {
                Self::kernel_reads(kind)
            }
            BindingKind::Struct(_)
            | BindingKind::Enum(_)
            | BindingKind::Variant(..)
            | BindingKind::Trait(_)
            | BindingKind::RustType(_) => Self::type_reads(kind),
            BindingKind::Local
            | BindingKind::TypeVariable
            | BindingKind::AssociatedType
            | BindingKind::Lifetime => Vec::new(),
        }
    }

    fn value_reads(kind: BindingKind) -> Vec<ReadRow> {
        match kind {
            BindingKind::Global(_) => vec![
                Self::read("a call", "greet", "(defn main () (println (greet)))"),
                Self::template("greet", "(greet)"),
                Self::uncalled_template("greet", "(greet)"),
                Self::read_from(
                    "a call in an arm whose pattern a variant of its name binds",
                    "(use tests/fixtures/lint/modules/every_kind shape)\n\
                     (use tests/fixtures/lint/modules/dot dot)",
                    "(export pick)\n(defn pick ((s shape)) (match s (dot (dot)) (_ 0)))\n\
                     (defn main () (println 1))",
                ),
                Self::read_from(
                    "a doctest that names the module the use loads",
                    "(use prelude/test doctest)\n(use tests/fixtures/lint/modules/count tally)",
                    "(doctest tests/fixtures/lint/modules/count)\n(defn main () (println 1))",
                ),
                Self::read_from(
                    "a doctest example",
                    "(use prelude/test doctest)\n\
                     (use tests/fixtures/lint/modules/counted counted)\n\
                     (use tests/fixtures/lint/modules/every_kind greet)",
                    "(doctest tests/fixtures/lint/modules/counted)\n(defn main () (println 1))",
                ),
                Self::read(
                    "a quoted name a compile-time def compares",
                    "greet",
                    "(use code same-binding?)\n(export greeted?)\n\
                     (def greet-form? (fn (name) (same-binding? name (quote greet))))\n\
                     (defmacro greeted? (name) (if (greet-form? name) 1 2))\n\
                     (defn main () (println (greeted? greet)))",
                ),
                Self::read_from(
                    "a macro call's argument a code builtin compares",
                    "(use tests/fixtures/lint/modules/greet_checks greeted?)\n\
                     (use tests/fixtures/lint/modules/every_kind greet)",
                    "(defn main () (println (greeted? greet)))",
                ),
            ],
            BindingKind::Macro(_) => vec![
                Self::read("a call", "shout", "(defn main () (println (shout 1)))"),
                Self::template("shout", "(shout 1)"),
                Self::uncalled_template("shout", "(shout 1)"),
            ],
            BindingKind::CodeBuiltin(_) => vec![
                Self::read_from(
                    "a call in a macro body",
                    "(use code value-eq)",
                    "(defmacro same (a b) (if (value-eq a b) 1 2))\n(defn main () (println (same x x)))",
                ),
                Self::read_from(
                    "a template",
                    "(use code value-eq)",
                    "(defmacro same (a b) (template (value-eq (quote a) (quote b))))\n(defn main () (println 1))",
                ),
            ],
            _ => Vec::new(),
        }
    }

    fn kernel_reads(kind: BindingKind) -> Vec<ReadRow> {
        match kind {
            BindingKind::KernelForm(_) => vec![Self::read_from(
                "a kernel form head",
                "(use kernel if)",
                "(defn main () (println (if true 1 2)))",
            )],
            BindingKind::KernelType(_) => vec![Self::read_from(
                "a type position",
                "(use kernel int)",
                "(defn main () (println (the int 1)))",
            )],
            BindingKind::Boolean(_) => vec![Self::read_from(
                "a literal",
                "(use kernel true)",
                "(defn main () (println true))",
            )],
            _ => Vec::new(),
        }
    }

    fn type_reads(kind: BindingKind) -> Vec<ReadRow> {
        match kind {
            BindingKind::Struct(_) => vec![
                Self::type_position("point", "point"),
                Self::implementor_row("point", "point"),
                Self::template("point", "(the point (unquote x))"),
                Self::uncalled_template("point", "(the point (unquote x))"),
            ],
            BindingKind::Enum(_) => vec![
                Self::type_position("shape", "shape"),
                Self::implementor_row("shape", "shape"),
                Self::read(
                    "a call of a variant",
                    "shape",
                    "(export made)\n(defn made () (shape/circle 1))\n(defn main () (println 1))",
                ),
                Self::template("shape", "(shape/circle 1)"),
                Self::uncalled_template("shape", "(shape/circle 1)"),
            ],
            BindingKind::Variant(..) => vec![
                Self::read(
                    "a call",
                    "shape/circle",
                    "(export made)\n(defn made () (shape/circle 1))\n(defn main () (println 1))",
                ),
                Self::template("shape/circle", "(shape/circle 1)"),
                Self::uncalled_template("shape/circle", "(shape/circle 1)"),
            ],
            BindingKind::Trait(_) => vec![
                Self::read(
                    "a call of a method",
                    "sized",
                    "(export measured)\n(defn measured (value) (size value))\n(defn main () (println 1))",
                ),
                Self::template("sized", "(size (unquote x))"),
                Self::uncalled_template("sized", "(size (unquote x))"),
            ],
            BindingKind::RustType(_) => vec![
                Self::type_position("boxed", "(boxed i64)"),
                Self::implementor_row("boxed", "(boxed i64)"),
                Self::implementor_row("boxed", "(where ((a tidy)) (boxed a))"),
                Self::template("boxed", "(the (boxed i64) (unquote x))"),
                Self::uncalled_template("boxed", "(the (boxed i64) (unquote x))"),
            ],
            _ => Vec::new(),
        }
    }

    fn unread_of(kind: BindingKind) -> Option<UnreadRow> {
        match kind {
            BindingKind::Global(_) | BindingKind::Macro(_) | BindingKind::CodeBuiltin(_) => {
                Self::value_unread(kind)
            }
            BindingKind::KernelForm(_) | BindingKind::KernelType(_) | BindingKind::Boolean(_) => {
                Self::kernel_unread(kind)
            }
            BindingKind::Struct(_)
            | BindingKind::Enum(_)
            | BindingKind::Variant(..)
            | BindingKind::Trait(_)
            | BindingKind::RustType(_) => Self::type_unread(kind),
            BindingKind::Local
            | BindingKind::TypeVariable
            | BindingKind::AssociatedType
            | BindingKind::Lifetime => None,
        }
    }

    fn value_unread(kind: BindingKind) -> Option<UnreadRow> {
        match kind {
            BindingKind::Global(_) => Some(Self::unread("greet", "shout", Some("greet"))),
            BindingKind::Macro(_) => Some(Self::unread("shout", "greet", None)),
            BindingKind::CodeBuiltin(_) => Some(UnreadRow {
                text: "(use code value-eq symbol)\n\
                       (defmacro same (a b) (if (value-eq a b) 1 2))\n\
                       (defn main () (println (same x x)))"
                    .to_string(),
                warns_on: Some("symbol"),
            }),
            _ => None,
        }
    }

    fn kernel_unread(kind: BindingKind) -> Option<UnreadRow> {
        match kind {
            BindingKind::KernelForm(_) => Some(UnreadRow {
                text: "(use kernel if int)\n(defn main () (println (the int 1)))".to_string(),
                warns_on: Some("if"),
            }),
            BindingKind::KernelType(_) => Some(UnreadRow {
                text: "(use kernel if int)\n(defn main () (println (if false 1 2)))".to_string(),
                warns_on: Some("int"),
            }),
            BindingKind::Boolean(_) => Some(UnreadRow {
                text: "(use kernel if true)\n(defn main () (println (if false 1 2)))".to_string(),
                warns_on: Some("true"),
            }),
            _ => None,
        }
    }

    fn type_unread(kind: BindingKind) -> Option<UnreadRow> {
        match kind {
            BindingKind::Struct(_) => Some(Self::unread("point", "greet", Some("point"))),
            BindingKind::Enum(_) => Some(Self::unread("shape", "greet", Some("shape"))),
            BindingKind::Variant(..) => {
                Some(Self::unread("shape/circle", "greet", Some("shape/circle")))
            }
            BindingKind::Trait(_) => Some(Self::unread("sized", "greet", Some("sized"))),
            BindingKind::RustType(_) => Some(Self::unread("boxed", "greet", Some("boxed"))),
            _ => None,
        }
    }

    fn read(way: &'static str, name: &str, body: &str) -> ReadRow {
        Self::read_from(way, &format!("(use {} {name})", Self::MODULE), body)
    }

    fn read_from(way: &'static str, import: &str, body: &str) -> ReadRow {
        ReadRow {
            way,
            text: format!("{import}\n{body}"),
        }
    }

    fn type_position(name: &str, written: &str) -> ReadRow {
        let body = format!(
            "(export keep)\n(type keep (fn ({written}) i64))\n(defn keep (_value) 1)\n{}",
            Self::MAIN
        );
        Self::read("a type position", name, &body)
    }

    fn implementor_row(name: &str, row: &str) -> ReadRow {
        let body = format!("(rust-trait tidy \"Tidy\" {row})\n{}", Self::MAIN);
        Self::read("a rust-trait implementor row", name, &body)
    }

    fn template(name: &str, template: &str) -> ReadRow {
        let body = format!(
            "(defmacro written (x) (template {template}))\n\
             (defn main () (println (written 1)))"
        );
        Self::read("a template", name, &body)
    }

    fn uncalled_template(name: &str, template: &str) -> ReadRow {
        let body = format!(
            "(defmacro written (x) (template {template}))\n{}",
            Self::MAIN
        );
        Self::read("a template of a macro nothing calls", name, &body)
    }

    fn unread(name: &str, read: &str, warns_on: Option<&'static str>) -> UnreadRow {
        let body = match read {
            "shout" => "(defn main () (println (shout 1)))",
            _ => "(defn main () (println (greet)))",
        };
        UnreadRow {
            text: format!("(use {} {name} {read})\n{body}", Self::MODULE),
            warns_on,
        }
    }

    fn unused_import_titles(text: &str) -> Vec<String> {
        ExpandedSource::expand(text)
            .warning_titles()
            .into_iter()
            .filter(|title| title.contains("imported but never used") || title.contains("`use`"))
            .collect()
    }
}

#[test]
fn every_importable_kind_has_reads_and_an_unread_row() {
    for kind in ImportKindTable::every_kind() {
        assert_eq!(
            kind.is_importable(),
            !ImportKindTable::reads_of(kind).is_empty(),
            "{kind:?}"
        );
        assert_eq!(
            kind.is_importable(),
            ImportKindTable::unread_of(kind).is_some(),
            "{kind:?}"
        );
    }
}

#[test]
fn a_name_read_in_any_way_its_kind_allows_is_used() {
    let mut warned = Vec::new();
    for kind in ImportKindTable::every_kind() {
        for row in ImportKindTable::reads_of(kind) {
            let titles = ImportKindTable::unused_import_titles(&row.text);
            if !titles.is_empty() {
                warned.push(format!("{kind:?} read by {}: {titles:?}", row.way));
            }
        }
    }
    assert!(warned.is_empty(), "{warned:#?}");
}

#[test]
fn a_name_never_read_warns_by_its_written_spelling() {
    for kind in ImportKindTable::every_kind() {
        let Some(row) = ImportKindTable::unread_of(kind) else {
            continue;
        };
        let expected: Vec<String> = row
            .warns_on
            .map(|name| format!("`{name}` is imported but never used"))
            .into_iter()
            .collect();
        assert_eq!(
            ImportKindTable::unused_import_titles(&row.text),
            expected,
            "{kind:?}"
        );
    }
}
