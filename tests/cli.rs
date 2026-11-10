use std::fs;
use std::io::{BufRead, BufReader, ErrorKind};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use knave::build::EmittedFormatting;
use knave::diagnostic::DiagnosticRenderer;
use knave::language::Language;
use knave::source::SourceMap;
use serde_json::{Map, Value, json};

const GREETING: &str = "(defn main ()\n  (println \"hello from the sandbox\"))\n";
const PRINTS_A: &str = "(defn main () (println \"A\"))\n";
const PRINTS_B: &str = "(defn main () (println \"B\"))\n";
const MARKED_PROGRAM: &str = "\u{feff}(defn main () (println 1))\n";
const FAILED_MAIN: &str = "(defn fails () (the (result i64 string) (err (str \"boom\"))))\n(defn main () (local value (try (fails))) (ok value))\n";
const REPORTING_MAIN: &str = "(defn works () (the (result i64 string) (ok 1)))\n(defn main ()\n  (eprintln \"working\")\n  (local value (try (works)))\n  (ok value))\n";
const ERROR_ONLY_MAIN: &str = "(defn main () (eprintln \"only\"))\n";
const UNWRAPPED_FAILURE: &str = "(defstruct failure :line i64)
(defn main ()
  (println (unwrap-ok (the (result i64 failure) (err (->failure 3))))))
";
const UNBOUND_NAME: &str = "(defn main ()\n  (println greeting))\n";
const ECHO: &str = "(def first-argument (rust \"std::env::args().nth(1).unwrap_or_default()\" string))\n\n(defn main ()\n  (println first-argument))\n";
const EXIT_THREE: &str =
    "(def exit (rust \"std::process::exit({0})\" (fn (i32) unit)))\n\n(defn main ()\n  (exit 3))\n";
const MAIN_ERROR: &str =
    "(defn main ()\n  (let-try (text (slurp \"missing.txt\"))\n    (do (println text) (ok 0))))\n";
const RUSTC_SYNTAX_ERROR: &str = "(def doubled-sign (rust \"{0} + + 1\" (fn (i64) i64)))\n\n(defn main ()\n  (println (doubled-sign 1)))\n";
const CRASHING_RUSTC: &str =
    "#!/bin/sh\necho 'rustc crashed before reporting anything' >&2\nexit 101\n";
const SILENT_RUSTC: &str = "#!/bin/sh\nexit 0\n";
const INTERRUPTED_RUSTC: &str = "#!/bin/sh\nkill -INT $$\n";
const WAITING_RUSTC: &str = "#!/bin/sh\necho compiling > /dev/tty\nexec sleep 30\n";
const WAITING_RUSTFMT: &str = "#!/bin/sh\necho formatting > /dev/tty\nexec sleep 30\n";
const SIGNAL_CATCHING_RUSTC: &str = "#!/bin/sh\nsleep 30 > /dev/null 2>&1 &\nsleeper=$!\ntrap 'kill $sleeper; echo \"error: compilation was interrupted\" >&2; exit 1' INT\necho compiling > /dev/tty\nwait\n";
const SIGNAL_CATCHING_RUSTFMT: &str = "#!/bin/sh\nsleep 30 > /dev/null 2>&1 &\nsleeper=$!\ntrap 'kill $sleeper; exit 1' INT\necho formatting > /dev/tty\nwait\n";
const INTERRUPTED_RUSTFMT: &str = "#!/bin/sh\nkill -INT $$\n";
const UNINTERRUPTED_RUSTFMT: &str =
    "#!/bin/sh\ntrap '' INT\necho formatting > /dev/tty\nsleep 1\nexec cat\n";
const PASSTHROUGH_RUSTFMT: &str =
    "#!/bin/sh\nwhile IFS= read -r line; do printf '%s\\n' \"$line\"; done\n";
const FAILING_RUSTFMT: &str = "#!/bin/sh\nprintf 'rustfmt failed in the sandbox\\n' >&2\nexit 1\n";
const USES_PRELUDE_DIRECTORY: &str =
    "(use prelude/extra good)\n\n(defn main ()\n  (println (good)))\n";
const BROKEN_PRELUDE_DIRECTORY_MODULE: &str =
    "(export good)\n\n(def good (rust \"1\" (fn () i64)))\n\n(def broken (fn () (if 1 2 3)))\n";
const MAIN_IN_PRELUDE: &str = "(def print-it (rust \"println!(\\\"hi\\\")\" (fn () unit)))\n\n(def main (fn () (print-it)))\n";
const BOUND_MODULE: &str = "(export person)\n\n(defstruct person :name string)\n";
const MILLION_LINES: &str = "(defn main ()\n  (for-each (range 0 1000000) (fn (i) (println i))))\n";
const THOUSAND_LINES: &str = "(defn main ()\n  (for-each (range 0 1000) (fn (i) (println i))))\n";
const ABORT: &str =
    "(def abort (rust \"std::process::abort()\" (fn () unit)))\n\n(defn main ()\n  (abort))\n";
const PROMPT_THEN_READ_LINE: &str = "(defn main ()\n  (print \"who? \")\n  (prn (read-line)))\n";
const PROMPT_THEN_SLURP: &str = "(defn main ()\n  (print \"who? \")\n  (prn (slurp stdin)))\n";
const PROMPT_THEN_SLURP_ANY_SOURCE: &str = "(type load (where ((s source)) (fn ((ref s)) (result string io-error))))\n(defn load (input) (slurp input))\n\n(defn main ()\n  (print \"who? \")\n  (prn (load stdin)))\n";
const COUNT_INPUT_LINES: &str = "(defn main ()\n  (println (count (take (line-seq stdin) 2))))\n";
const STRUCTS: &str = "(defstruct person :name string :age i64)\n(defstruct ticket :id u32)\n(derive ticket debug)\n\n(defn main () (do))\n";
const BOUND_ENTRY: &str = "(use other/person person)\n\n(rust-trait same \"std::ops::Add\" i64 (with i64 person))\n\n(type matches (where ((a (same person))) (fn (a) bool)))\n(defn matches (x) true)\n\n(defn main () (println (matches 1)))\n";

const TRAIT_SHARED_VECTOR: &str = "(struct odometer (offset i64))\n\n(trait mileage\n  (first-reading (fn ((ref self) (ref (vec i64))) i64)))\n\n(impl mileage odometer\n  (first-reading (o readings) (+ (field o offset) (at readings 0))))\n\n(defn main ()\n  (println (first-reading (new odometer (offset 10)) (vec 1200))))\n";
const DOCUMENTED_TWICE: &str = "(defn twice (x)\n  \"Doubles a number.\n\n  Handy for widgets.\"\n  (* 2 x))\n\n(defn main () (println (twice 2)))\n";
const DOCUMENTED_PROJECT: &str = "(project demo)\n";
const DOCUMENTED_MAIN: &str =
    "(module \"Prints a greeting.\")\n\n(export main)\n\n(defn main () (println \"hi\"))\n";
const DOCUMENTED_AREA: &str = "(export area)\n\n(defn area (width height)\n  \"Multiplies two sides, as [option] does not.\"\n  (* width height))\n";
const AREA_SECTION: &str = "<a id=\"area\"></a>\n\n## `area`\n\n```lisp\n(defn area (width height))\n(type area (where ((a number)) (fn (a a) a)))\n```\n\nMultiplies two sides, as [option](https://knave-lang.org/docs/prelude/types/#option) does not.\n";
const UNTYPED_SHAPES: &str = "(export point twice)\n\n(defstruct point \"A point.\" :x i64)\n\n(defrule twice \"Writes a form twice.\" ((x) (do x x)))\n";
const UNTYPED_SHAPES_PAGE: &str = "# shapes\n\n<a id=\"point\"></a>\n\n## `point`\n\n```lisp\n(defstruct point)\n```\n\nA point.\n\n<a id=\"twice\"></a>\n\n## `twice`\n\n```lisp\n(defrule twice)\n```\n\nWrites a form twice.\n";
const PRELUDE_PAGES: [&str; 23] = [
    "assoc",
    "definers",
    "documentation",
    "io",
    "iterator",
    "logic",
    "macros",
    "map",
    "number",
    "option",
    "ownership",
    "pair",
    "place",
    "project-file",
    "result",
    "sequence",
    "struct",
    "syntax",
    "task",
    "test",
    "text",
    "types",
    "vec",
];
const UNWRAP_OR_SECTION: &str = "<a id=\"unwrap-or\"></a>\n\n## `unwrap-or`\n\n```lisp\n(unwrap-or option default)\n(type unwrap-or (where (a) (fn ((option a) a) a)))\n```\n";
const PRELUDE_REEXPORT: &str = "## Re-exported names\n\n- [`*`](prelude/number.md#%2A)\n";
const GEOMETRY: &str = "(export area)\n\n(defn area (width height) \"Multiplies a width by a height.\" (* width height))\n";
const GEOMETRY_ENTRY: &str = "(use geometry area)\n\n(defn main () (println (area 3 4)))\n";
const DOCUMENTED_GEOMETRY: &str = "(module \"Shapes on a grid.\")\n\n(export area perimeter)\n\n(defn area (width height)\n  \"Multiplies two sides.\"\n  (* width height))\n\n(defn perimeter (width height)\n  (* 2 (+ width height)))\n\n(defn helper (x)\n  \"Not exported.\"\n  x)\n";
const GEOMETRY_PAGE: &str = "(module geometry)\n\nShapes on a grid.\n\n(defn area (width height))\n    Multiplies two sides.\n(defn perimeter (width height))\n";
const DOCUMENTED_ENTRY: &str =
    "(module \"Prints an area.\")\n\n(export main)\n\n(defn main ()\n  (println 6))\n";
const MODULE_BINDING_ENTRY: &str =
    "(use geometry area)\n\n(def module 1)\n\n(defn main ()\n  (println (area module 3)))\n";
const SHADOWING_GEOMETRY_ENTRY: &str =
    "(use geometry area)\n\n(def geometry 1)\n\n(defn main ()\n  (println (area geometry 3)))\n";
const UNDOCUMENTED_ENTRY: &str = "(defn twice (x) (* 2 x))\n\n(defn main () (println (twice 2)))\n";
const SHADOWING_PAGE_BODY: &str = "(defrule doc-body ((term) 0))\n\n(defn main () (do))\n";
const SHADOWING_HEADER: &str = "(defrule doc-header ((term) 0))\n\n(defn main () (do))\n";
const TRUE_PAGE: &str = "true\n\nIs the `bool` value true.\n\nIt is a kernel constant, so macro code and program code see the same\nbooleans.\n\n(if true 1 2)\n=> 1\n";
const IF_PAGE: &str = "(if test then else)\n\nEvaluates `then` when the test is true, and `else` otherwise.\n\nIt emits a Rust `if` with an `else`.\n\n(if (> 2 1) \"yes\" \"no\")\n=> \"yes\"\n";
#[cfg(target_os = "linux")]
const DEEP_NESTING: usize = 1000;
const MACRO_STRUCT_BESIDE_A_DEF: &str = "(defrule my-struct ((n) (do (defstruct n \"A documented struct.\" :x i64))))\n(my-struct point)\n(def point 1)\n(defn main () (do))\n";
const THREADING_HEADER: &str = "(-> value form ...)";
const THREAD_LAST_HEADER: &str = "(->> value form ...)";
const MINUS_HEADER: &str = "(- number more ...)";
const FLAG_NAMED_DEF: &str = "(def --verbose 1)\n\n(defn main () (println --verbose))\n";
const FLAG_NAMED_DEF_PAGE: &str = "(def --verbose)\n\nNo documentation.\n";
const DEFN_HEADER: &str = "(defmacro defn (name parameters body ...))";
const SEPARATOR_TIP: &str = "write `knave doc -- <term>`";
const UNKNOWN_FLAG_ERROR: &str = "error: unexpected argument '--typo' found";
const ARITHMETIC: &str = include_str!("fixtures/emit/arithmetic.knv");
const EMPTY_MAIN: &str = "(def main (fn () (do)))\n";
const ONE_BRANCH: &str = "(def main (fn () (if true (do) (do))))\n";

const STAGE_WORK_KEYS: [&str; 4] = [
    "expand.lowered_forms",
    "check.inferred_expressions",
    "emit.emitted_expressions",
    "build.scanned_tokens",
];
const TYPED_PROGRAM: &str = "(defn twice (x) \"Doubles a number.\" (* 2 x))\n(defn same (x) x)\n(defstruct point :x i64 :y i64)\n(def limit 10)\n(defn area (:width i64 :height i64) (* width height))\n(defn main () (println (twice limit)))\n";
const DOCUMENTED_STRUCT_BESIDE_A_FUNCTION: &str = "(defstruct point \"A point on a grid.\" :x i64 :y i64)\n(defn point () (->point 1 2))\n(defn main () (do))\n";
const DOCUMENTED_FUNCTION_BESIDE_A_STRUCT: &str = "(defstruct point :x i64 :y i64)\n(defn point () \"Makes the origin.\" (->point 0 0))\n(defn main () (do))\n";
const UNDOCUMENTED_DEF_BESIDE_A_STRUCT: &str =
    "(defstruct point :x i64 :y i64)\n(def point 3)\n(defn main () (do))\n";
const UNDOCUMENTED_FUNCTION_BESIDE_A_STRUCT: &str =
    "(defstruct point :x i64 :y i64)\n(defn point () (->point 1 2))\n(defn main () (do))\n";
const OTHER_DEFINITION_FAILS: &str = "(defn twice (x) \"Doubles a number.\" (* 2 x))\n(defn bad () (+ 1 \"a\"))\n(defn main () (println (twice 2)))\n";
const OWN_DEFINITION_FAILS: &str =
    "(defn twice (x) \"Doubles a number.\" (+ x \"a\"))\n(defn main () (do))\n";
const PAGE_PROGRAM: &str = "(defn twice (x) \"Doubles a number.\" (* 2 x))\n(def page (doc-page twice))\n(defn main () (println page))\n";
const SHADOWING_SUM: &str =
    "(defn sum (left right) (+ left right))\n(defn main () (println (sum 1 2)))\n";
const NUMBERS: &str = "(export twice)\n\n(defn twice (x) \"Doubles a number.\" (* 2 x))\n";
const NUMBERS_ENTRY: &str = "(use lib/numbers twice)\n\n(defn main () (println (twice 21)))\n";
const TWICE_TYPE: &str = "(type twice (fn (i64) i64))";
const TWICE_PAGE: &str = "(defn twice (x))\n\nDoubles a number.\n";
const POINT_TYPE: &str = "(type point (fn () point))";

const SESSION: &str = "(defn square (n) (* n n))\n(defstruct point :x i64 :y i64)\n(defn main () (println \"written main\"))\n";
const EVALUATED_GEOMETRY: &str = "(defn area (w h) (* w h))\n(area 2 3)\n(defn report ()\n  (println (area 4 5)))\n(do (defn twice (n) (* 2 n)))\n";
const GEOMETRY_DEFINITIONS: &str = "(defn area (w h) (* w h))\n(defn report ()\n  (println (area 4 5)))\n(do (defn twice (n) (* 2 n)))\n";
const AREA_OF_A_STRING: &str = "(defn area (w h) (* w h))\n(defn report () (area 2 \"3\"))\n";
const TOP_LEVEL_AREA_OF_A_STRING: &str = "(defn area (w h) (* w h))\n(area 2 \"3\")\n";
const SCRATCH: &str = "(defn f (n) (* 2 n))\n\n(comment (f 1) (exit 3))\n";
const EXITS: &str = "(def exit (rust \"std::process::exit({0})\" (fn (i32) unit)))\n(def abort (rust \"std::process::abort()\" (fn () unit)))\n";
const TOP_LEVEL_DO_OF_EXPRESSIONS: &str = "(defn square (n) (* n n))\n(do (prn (square 3)))\n";
const UNREACHABLE_ARM_IN_A_BODY: &str =
    "(defn square (n) (* n n))\n(defn pick ()\n  (match 3 (_ 1) (_ 2)))\n";
const MIXED_TOP_LEVEL_DO: &str =
    "(defn square (n) (* n n))\n(do (defn cube (n) (* n (square n))) (prn 7))\n";
const UNREACHABLE_ARM: &str = "(defn pick (x) (match x (_ 1) (_ 2)))\n";
const SHAPES: &str = "(export circle)\n(defn circle (r) (* 3 r r))\n";
const SHAPES_USER: &str = "(use shapes circle)\n";
const SHAPES_OF_TWO: &str =
    "(export circle square)\n(defn circle (r) (* 3 r r))\n(defn square (s) (* s s))\n";
const POINTS: &str = "(defstruct point :x i64 :y i64)\n";
const PROMPT_DEFINITIONS: &str = "(defn square (n) (* n n))\n(defstruct point :x i64 :y i64)\n(->point (square 2) 3)\n(println \"side \" (square 3))\n";
const PROMPT_DOC: &str =
    "(defn twice (x) \"Doubles a number.\" (* 2 x))\n(doc twice)\n(doc-page twice)\n";
const PROMPT_RUNAWAY_MACRO: &str =
    "(defmacro spin () ((fn () (recur))))\n(with-macro-steps 1000 (spin))\n(+ 1 2)\n";
const PROMPT_INFER: &str = "(defn identity (x) x)\n(infer (vec 1 2))\n(infer identity)\n(infer none)\n(def text (infer identity))\ntext\n";
const PROMPT_REDEFINITION: &str = "(defn square (n) (* n n))\n(defn cube (n) (* n (square n)))\n(cube 3)\n(defn square (n) (+ n n))\n(cube 3)\n";
const PROMPT_DROPPING_REPLACEMENT: &str = "(defstruct point :x i64 :y i64)
(defn ->point (x y) (vec x y))
(->point 1 2)
";
const PROMPT_REPLACEMENT_OF_TWO: &str = "(defn circle (r) r)\n(defn square (s) s)\n(do (defn circle (r) (+ r 1)) (defn square (s) (+ s 2)))\n(+ (circle 1) (square 1))\n";
const PROMPT_FAILING_DEFINITION: &str =
    "(defn twice (x) (* 2 x))\n(defn twice (x) (+ x \"a\"))\n(twice 4)\n";
const PROMPT_FORMS_ON_ONE_LINE: &str = "(defn twice (x) (* 2 x)) (twice 4) (twice 5)\n";
const PROMPT_MULTI_LINE: &str =
    "(defn greet (name)\n  (println \"hello, \"\n    name))\n(greet \"line\nbreak\")\n";
const PROMPT_USES: &str =
    "(use shapes circle square)\n(circle 2)\n(use shapes circle)\n(square 3)\n";
const PROMPT_PANIC: &str = "(defn twice (x) (* 2 x))\n(at (vec 1 2) 5)\n(twice 4)\n";
const PROMPT_ABORT: &str =
    "(def abort (rust \"std::process::abort()\" (fn () unit)))\n(abort)\n(+ 1 2)\n";
const PROMPT_EXIT: &str =
    "(def exit (rust \"std::process::exit({0})\" (fn (i32) unit)))\n(exit 3)\n(exit 0)\n";
const PROMPT_UNCLOSED_END: &str = "(+ 1 2)\n(defn twice (x)\n  (* 2 x)\n";
const PROMPT_NOT_UTF8: &[u8] = b"(+ 1 1)\n\xff\n";
const PROMPT_REDEFINED_STRUCT: &str = "(defstruct user :name str)\n(def karl (->user :name \"karl\"))\n(defstruct user :name str :email str)\n(karl)\n";
const PROMPT_FAILED_REDEFINED_STRUCT: &str = "(defstruct user :name str)\n(def karl (->user :name \"karl\"))\n(defstruct user (name str))\n(:name karl)\n";
const PROMPT_CASCADE: &str = "(defstruct user :name str)\n(def karl (->user :name \"karl\"))\n(defn greet () (str \"hi \" (:name karl)))\n(defstruct user :name str :email str)\n(greet)\n";
const PROMPT_BROKEN_MACRO_CALLED: &str =
    "(defmacro bad () (template (missing-helper 1)))\n(+ 1 2)\n(defn f () (bad))\n(f)\n";
const PROMPT_BROKEN_MACRO_REDEFINED: &str = "(defmacro bad () (template 1))\n(defn f () (bad))\n(defmacro bad () (template (missing-helpr 1)))\n(f)\n";
const PROMPT_STOPS_BINDING: &str = "(defn pick (a b) a)\n(defmacro mk (a b) (template (defn (unquote (pick a b)) () 1)))\n(mk foo bar)\n(defn pick (a b) b)\n(foo)\n(bar)\n";
const PROMPT_TWO_BINDERS: &str = "(defn pick (a b) a)\n(defmacro mk (a b) (template (defn (unquote (pick a b)) () 1)))\n(mk foo bar)\n(defn bar () 2)\n(defn pick (a b) b)\n(bar)\n";
const PROMPT_WOULD_SET_ASIDE: &str = "(defstruct user :name str)\n(def karl (->user :name \"karl\"))\n(do (defstruct user :name str :email str) (defn show () (:name karl)))\n(:name karl)\n";
const PROMPT_RE_ENTERED_TYPE: &str = "(defstruct point :x i64 :y i64)\n(defn ->point (x y) (vec x y))\n(defstruct point :x i64 :y i64)\n(use geo area)\n";
const GEO_WITHOUT_POINT: &str = "(export area)\n(defn area (:p point) 1)\n";
const PROMPT_EXPRESSION_CALLING_A_BROKEN_MACRO: &str =
    "(defmacro bad () (template (missing-helper 1)))\n(bad)\n";
const PROMPT_TWO_ERRORS_IN_ONE_INPUT: &str = "(defstruct user :name str)\n(def karl (->user :name \"karl\"))\n(defn greet () (str (:name karl) (:name karl)))\n(defstruct user :name str :email str)\n";
const SHAPES_WITH_A_TYPO: &str =
    "(write-file \"shapes.knv\" \"(export circle)\\n(defn circle (r) (nope r))\\n\")\n";
const WRITE_FILE: &str = "(def write-file (rust \"std::fs::write({0}, {1}).unwrap()\" (fn ((ref str) (ref str)) unit)))\n";
const SHAPES_WITHOUT_SQUARE: &str =
    "(write-file \"shapes.knv\" \"(export circle)\\n(defn circle (r) r)\\n\")\n";
const PROMPT_EXIT_ON_A_TERMINAL: &[Keys] = &[
    Keys::AtPrompt("(def exit (rust \"std::process::exit({0})\" (fn (i32) unit)))\r"),
    Keys::AtPrompt("(exit 3)\r"),
    Keys::AtPrompt(Keys::END_OF_INPUT),
];
const PROMPT_EXTRA_CLOSE: &str = "(+ 1 2))\n(+ 3 4)\n(+ 3 \"a\")\n";
const PROMPT_ENUM_REDEFINITION: &str = "(enum shape circle square)\n(enum shape circle square triangle)\n(match shape/triangle (triangle 3) (_ 0))\n";
const PROMPT_WARNING: &str = "(defn pick (x) (match x (_ 1) (_ 2)))\n(pick 1)\n(defn other () 1)\n(defn another () 2)\n(pick 2)\n";

const SHADOWED_PRINTLN: &str = "(defn println (x) x)\n(defn square (n) (* n n))\n";
const COUNT_THEN_FAIL: &str = "(def exit (rust \"std::process::exit({0})\" (fn (i32) unit)))\n(type count-up (fn (i64) unit))\n(defn count-up (i)\n  (println i)\n  (if (< i 1000000) (recur (+ i 1)) (exit 7)))\n";

const UNFORMATTED: &str = "(defn twice (x)\n      (* 2 x))   ; doubles\n(def limit 10)\n";
const FORMATTED: &str = "(defn twice (x) (* 2 x)) ; doubles\n\n(def limit 10)\n";
const UNCLOSED: &str = "(defn main ()\n  (println 1)\n";
const LOOP_SHAPE: &str = "(use prelude/macros header-shape)\n\n(export dotimes)\n\n(defrule dotimes ((binding body ...) (do body ...)))\n\n(header-shape (dotimes binding))\n";
const LOOP_CALL: &str = "(use shapes/loops dotimes)\n\n(dotimes (index 10) (println \"first line of the body\") (println index))\n";
const LOOP_CALL_SHAPED: &str = "(use shapes/loops dotimes)\n\n(dotimes (index 10)\n  (println \"first line of the body\")\n  (println index))\n";

const UNBOUND_NAMES_PAST_THE_CAP: usize = 25;

struct Sandbox {
    directory: PathBuf,
}

impl Sandbox {
    const COMPILER: &str = env!("CARGO_BIN_EXE_knave");
    const BUILD_PARENT: &str = "target";

    fn new(test_name: &str) -> Self {
        let directory = std::env::temp_dir().join(format!(
            "{}-cli-{}-{test_name}",
            Language::NAME,
            std::process::id()
        ));
        fs::create_dir_all(&directory).expect("the sandbox directory is writable");
        Self { directory }
    }

    const EXECUTABLE: u32 = 0o755;
    const PIPE_OVERFLOWING_GREETING_REPEATS: usize = 1 << 18;
    const PIPE_OVERFLOWING_INPUT_BYTES: usize = 1 << 20;
    const READ_ONLY: u32 = 0o444;
    const PRIVATE_FILE: u32 = 0o600;
    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    const CACHE_HOME_VARIABLE: &str = "XDG_CACHE_HOME";
    const CACHE_DIRECTORY: &str = "cache";
    const CARGO_TARGET_VARIABLE: &str = "CARGO_TARGET_DIR";
    const CARGO_BUILD_TARGET_VARIABLE: &str = "CARGO_BUILD_TARGET_DIR";
    const GIT_MESSAGE: &str = "git says:";
    const SANDBOX_MASK: &str = "<sandbox>";
    const TRANSLATING_CARGO: &str = r#"#!/bin/sh
printf '\n[dependencies]\nfake_json = "1.0.0"\n' >> Cargo.toml
"#;
    const VERSIONLESS_CARGO: &str = r#"#!/bin/sh
printf '\n[dependencies]\nfake-json = { path = "elsewhere" }\n' >> Cargo.toml
"#;
    const WAITING_CARGO: &str = r#"#!/bin/sh
echo "    Blocking waiting for file lock on build directory" >&2
exit 101
"#;
    const BIN_NAME_PREFIX: &str = "name = \"app-";
    const LINKED_LIBRARY: &str = "linked-lib";
    const LINKED_MODULE: &str = "linked-module";
    const OUTSIDE_FROM_STORE: &str = "../../../../../keymap/src/lib.knv";
    const FIXTURE_COMMIT_DATE: &str = "2026-01-01T00:00:00Z";
    const FAKE_GIT: &str = "#!/bin/sh\necho \"fatal: prompt=$GIT_TERMINAL_PROMPT schemes=$GIT_ALLOW_PROTOCOL ssh=$GIT_SSH_COMMAND\" >&2\nexit 1\n";
    const MASKED_GIT_MESSAGE: &str = "  = note: git says: <message>";
    const UNWRITABLE_DIRECTORY: u32 = 0o555;
    const NO_ACCESS: u32 = 0o000;
    const TERMINAL_RECORDER: &str = "script";
    const INPUT_PATIENCE: Duration = Duration::from_secs(10);
    const STATE_HOME_VARIABLE: &str = "XDG_STATE_HOME";
    const STATE_DIRECTORY: &str = "state";
    const COLOR_VARIABLES: [&str; 5] = ["NO_COLOR", "CLICOLOR_FORCE", "CLICOLOR", "TERM", "CI"];
    const TEMPORARY_DIRECTORY_VARIABLE: &str = "TMPDIR";
    #[cfg(target_os = "linux")]
    const FULL_DEVICE: &str = "/dev/full";

    fn with_file(self, name: &str, source: &str) -> Self {
        let path = self.directory.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("the sandbox accepts directories");
        }
        fs::write(path, source).expect("the sandbox accepts files");
        self
    }

    fn without_file(self, name: &str) -> Self {
        fs::remove_file(self.directory.join(name)).expect("the sandbox removes files");
        self
    }

    fn files_under(&self, directory: &str) -> Vec<String> {
        let root = self.directory.join(directory);
        let mut pending = vec![root.clone()];
        let mut files = Vec::new();
        while let Some(folder) = pending.pop() {
            for entry in fs::read_dir(&folder).expect("the sandbox folder is readable") {
                let path = entry.expect("the sandbox folder lists").path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    let relative = path.strip_prefix(&root).expect("the file is inside");
                    files.push(relative.display().to_string());
                }
            }
        }
        files.sort();
        files
    }

    fn contents(&self, name: &str) -> String {
        fs::read_to_string(self.directory.join(name)).expect("the sandbox file is readable")
    }

    fn with_documented_project(test_name: &str) -> Self {
        Self::new(test_name)
            .with_file(&Language::project_file_name(), DOCUMENTED_PROJECT)
            .with_file("src/main.knv", DOCUMENTED_MAIN)
            .with_file("src/geometry/area.knv", DOCUMENTED_AREA)
    }

    fn with_unbound_names(self, name: &str, count: usize) -> Self {
        let body: String = (0..count)
            .map(|index| format!("    missing{index}\n"))
            .collect();
        self.with_file(name, &format!("(defn main ()\n  (do\n{body}    1))\n"))
    }

    fn pipe_overflowing_greeting() -> String {
        let greeting = "hello ".repeat(Self::PIPE_OVERFLOWING_GREETING_REPEATS);
        format!("(defn main () (println \"{greeting}\"))\n")
    }

    fn with_executable(self, name: &str, script: &str) -> Self {
        let sandbox = self.with_file(name, script);
        let permissions = fs::Permissions::from_mode(Self::EXECUTABLE);
        fs::set_permissions(sandbox.directory.join(name), permissions)
            .expect("the sandbox marks files executable");
        sandbox
    }

    fn compiler(&self, arguments: &[&str]) -> Outcome {
        Outcome::of(&mut self.command(Path::new(Self::COMPILER), arguments))
    }

    fn typed_page(&self, arguments: &[&str]) -> String {
        self.typed_lookup(arguments).assert_exit_code(0).stdout()
    }

    fn typed_lookup(&self, arguments: &[&str]) -> Outcome {
        let typed: Vec<&str> = ["doc", "--types"]
            .iter()
            .chain(arguments)
            .copied()
            .collect();
        self.compiler(&typed)
    }

    fn compiler_reading(&self, input: impl AsRef<[u8]>, arguments: &[&str]) -> Outcome {
        Self::reading(
            &mut self.command(Path::new(Self::COMPILER), arguments),
            input,
        )
    }

    fn compiler_on_a_terminal_reading(
        &self,
        typed: &[Keys],
        environment: &[(&str, &str)],
        arguments: &[&str],
    ) -> Outcome {
        let recorded = Self::recorded_arguments(arguments);
        let recorded: Vec<&str> = recorded.iter().map(String::as_str).collect();
        let state = self.directory.join(Self::STATE_DIRECTORY);
        let state = state.to_str().expect("the sandbox path is UTF-8");
        let environment: Vec<(&str, &str)> = [(Self::STATE_HOME_VARIABLE, state)]
            .into_iter()
            .chain(environment.iter().copied())
            .collect();
        let mut command =
            self.color_command(Path::new(Self::TERMINAL_RECORDER), &environment, &recorded);
        Self::typing_until_the_end(&mut command, typed)
    }

    fn typing_until_the_end(command: &mut Command, typed: &[Keys]) -> Outcome {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the terminal recorder starts");
        let mut keyboard = child.stdin.take().expect("stdin is piped");
        let mut stdout = child.stdout.take().expect("stdout is piped");
        let mut stderr = child.stderr.take().expect("stderr is piped");
        let (stdout, stderr, status) = std::thread::scope(|scope| {
            let (sender, updates) = mpsc::channel();
            let stdout = scope.spawn(move || TerminalScreen::forward(&mut stdout, &sender));
            let stderr = scope.spawn(move || Self::read_all(&mut stderr));
            let mut screen = TerminalScreen::new(updates);
            for keys in typed {
                if let Err(missing) = screen.type_keys(keys, &mut keyboard) {
                    child.kill().ok();
                    panic!("{missing}");
                }
            }
            let status = child.wait().expect("the terminal recorder finishes");
            (
                stdout.join().expect("stdout is read"),
                stderr.join().expect("stderr is read"),
                status,
            )
        });
        drop(keyboard);
        Outcome {
            output: Output {
                status,
                stdout,
                stderr,
            },
        }
    }

    fn read_all(stream: &mut impl std::io::Read) -> Vec<u8> {
        let mut bytes = Vec::new();
        stream
            .read_to_end(&mut bytes)
            .expect("the stream is readable");
        bytes
    }

    fn reading(command: &mut Command, input: impl AsRef<[u8]>) -> Outcome {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the compiler starts");
        Self::offer_input(&mut child, input.as_ref());
        Outcome {
            output: child.wait_with_output().expect("the compiler finishes"),
        }
    }

    fn compiler_writing_to_a_closed_pipe(&self, arguments: &[&str]) -> Outcome {
        self.compiler_writing_to(Self::closed_pipe(), arguments)
    }

    fn compiler_with_a_closed_standard_error(&self, arguments: &[&str]) -> Outcome {
        self.compiler_reporting_to(Self::closed_pipe(), arguments)
    }

    fn assert_exit_code_with_a_closed_error_pipe(&self, arguments: &[&str], expected: i32) {
        self.compiler(arguments).assert_exit_code(expected);
        self.compiler_with_a_closed_standard_error(arguments)
            .assert_exit_code(expected);
    }

    fn closed_pipe() -> Stdio {
        let (reader, writer) = std::io::pipe().expect("the test opens a pipe");
        drop(reader);
        Stdio::from(writer)
    }

    #[cfg(target_os = "linux")]
    fn compiler_writing_to_a_full_device(&self, arguments: &[&str]) -> Outcome {
        let full_device = fs::OpenOptions::new()
            .write(true)
            .open(Self::FULL_DEVICE)
            .expect("the full device opens");
        self.compiler_writing_to(Stdio::from(full_device), arguments)
    }

    fn compiler_writing_to(&self, standard_output: Stdio, arguments: &[&str]) -> Outcome {
        let mut command = self.command(Path::new(Self::COMPILER), arguments);
        command.stdout(standard_output);
        Outcome::of(&mut command)
    }

    fn compiler_blocking_sigabrt(&self, arguments: &[&str]) -> Outcome {
        let mut command = self.command(Path::new(Self::COMPILER), arguments);
        unsafe { command.pre_exec(Self::block_sigabrt) };
        Outcome::of(&mut command)
    }

    fn block_sigabrt() -> std::io::Result<()> {
        unsafe {
            let mut only_sigabrt: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut only_sigabrt);
            libc::sigaddset(&mut only_sigabrt, libc::SIGABRT);
            libc::pthread_sigmask(libc::SIG_BLOCK, &only_sigabrt, std::ptr::null_mut());
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn compiler_on_a_short_stack(&self, arguments: &[&str]) -> Outcome {
        let mut command = self.command(Path::new(Self::COMPILER), arguments);
        unsafe { command.pre_exec(Self::shorten_the_stack) };
        Outcome::of(&mut command)
    }

    #[cfg(target_os = "linux")]
    fn shorten_the_stack() -> std::io::Result<()> {
        const SHORT_STACK_BYTES: libc::rlim_t = 1024 * 1024;
        let short = libc::rlimit {
            rlim_cur: SHORT_STACK_BYTES,
            rlim_max: SHORT_STACK_BYTES,
        };
        match unsafe { libc::setrlimit(libc::RLIMIT_STACK, &short) } {
            0 => Ok(()),
            _ => Err(std::io::Error::last_os_error()),
        }
    }

    fn compiler_reporting_to(&self, standard_error: Stdio, arguments: &[&str]) -> Outcome {
        let mut command = self.command(Path::new(Self::COMPILER), arguments);
        command.stderr(standard_error);
        Outcome::of(&mut command)
    }

    fn compiler_in(&self, working_directory: &str, arguments: &[&str]) -> Outcome {
        let mut command = self.command(Path::new(Self::COMPILER), arguments);
        command.current_dir(self.directory.join(working_directory));
        Outcome::of(&mut command)
    }

    fn compiler_reading_in(
        &self,
        working_directory: &str,
        input: impl AsRef<[u8]>,
        arguments: &[&str],
    ) -> Outcome {
        let mut command = self.command(Path::new(Self::COMPILER), arguments);
        command.current_dir(self.directory.join(working_directory));
        Self::reading(&mut command, input)
    }

    fn with_fixture_tree(self, name: &str) -> Self {
        Self::copy_tree(&Path::new(Self::FIXTURES).join(name), &self.directory);
        self
    }

    fn copy_tree(from: &Path, to: &Path) {
        fs::create_dir_all(to).expect("the sandbox accepts directories");
        for entry in fs::read_dir(from).expect("the fixture tree is readable") {
            let entry = entry.expect("the fixture tree lists its entries");
            let target = to.join(entry.file_name());
            if entry.path().is_dir() {
                Self::copy_tree(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).expect("the sandbox accepts files");
            }
        }
    }

    fn compiler_in_fixture(directory: &str, arguments: &[&str]) -> Outcome {
        let mut command = Command::new(Self::COMPILER);
        command
            .args(arguments)
            .current_dir(Path::new(Self::FIXTURES).join(directory))
            .stdin(Stdio::null());
        Self::isolated(&mut command, &Self::unsandboxed_cache());
        Outcome::of(&mut command)
    }

    fn compared_run(
        &self,
        scenario: &str,
        profile: Option<&str>,
        with_crate: bool,
    ) -> (i32, String, String) {
        let dependencies = if with_crate {
            " (dependencies (answer :path \"../answer\"))"
        } else {
            ""
        };
        let project = format!("(project compare :entry \"src/{scenario}.knv\"{dependencies})\n");
        fs::write(
            self.directory
                .join("compare")
                .join(Language::project_file_name()),
            project,
        )
        .expect("the sandbox accepts files");
        let mut arguments = vec!["build", "-o", "program"];
        arguments.extend(profile);
        self.compiler_in("compare", &arguments).assert_exit_code(0);
        let output = Command::new(self.directory.join("compare/program"))
            .output()
            .expect("the built program starts");
        let stderr = String::from_utf8_lossy(&output.stderr)
            .lines()
            .map(Self::without_panic_location)
            .collect::<Vec<_>>()
            .join("\n");
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr,
        )
    }

    fn without_panic_location(line: &str) -> String {
        match line.split_once(" panicked at ") {
            Some((_, location)) => {
                let position = location
                    .rsplit_once("main.rs:")
                    .map_or(location, |(_, at)| at);
                format!("thread 'main' panicked at main.rs:{position}")
            }
            None => line.to_string(),
        }
    }

    fn with_fake_registry(self) -> Self {
        Self::copy_tree(
            &Path::new(Self::FIXTURES).join("registry"),
            &self.directory.join("registry"),
        );
        let config = format!(
            "[source.crates-io]\nreplace-with = \"fixtures\"\n\n[source.fixtures]\ndirectory = \"{}\"\n",
            self.directory.join("registry").display()
        );
        self.with_file("cargo-home/config.toml", &config)
            .with_file(&Language::project_file_name(), "(project demo)\n")
            .with_file(
                &format!("src/main.{}", Language::EXTENSION),
                "(defn main ()\n  (println \"hi\"))\n",
            )
    }

    fn compiler_with_registry(&self, arguments: &[&str]) -> Outcome {
        let mut command = self.command(Path::new(Self::COMPILER), arguments);
        command.env("CARGO_HOME", self.directory.join("cargo-home"));
        Outcome::of(&mut command)
    }

    fn compiler_with_translating_cargo(&self, arguments: &[&str]) -> Outcome {
        self.compiler_with_fake_cargo(Self::TRANSLATING_CARGO, arguments)
    }

    fn compiler_with_fake_cargo(&self, script: &str, arguments: &[&str]) -> Outcome {
        let fake_cargo = self.directory.join("translating-bin/cargo");
        fs::create_dir_all(fake_cargo.parent().expect("the fake cargo has a directory"))
            .expect("the sandbox accepts directories");
        fs::write(&fake_cargo, script).expect("the sandbox accepts files");
        fs::set_permissions(&fake_cargo, fs::Permissions::from_mode(Self::EXECUTABLE))
            .expect("the sandbox sets permissions");
        let mut command = self.command(Path::new(Self::COMPILER), arguments);
        command.env("PATH", self.directory.join("translating-bin"));
        Outcome::of(&mut command)
    }

    fn project_file_text(&self) -> String {
        fs::read_to_string(self.directory.join(Language::project_file_name()))
            .expect("the project file is readable")
    }

    fn check_fixture(directory: &str) -> Outcome {
        let mut command = Command::new(Self::COMPILER);
        command
            .arg("check")
            .current_dir(Path::new(Self::FIXTURES).join(directory));
        Self::isolated(&mut command, &Self::unsandboxed_cache());
        Outcome::of(&mut command)
    }

    fn git(&self, working_directory: &str, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(self.directory.join(working_directory))
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Fixture Author")
            .env("GIT_AUTHOR_EMAIL", "author@example.com")
            .env("GIT_COMMITTER_NAME", "Fixture Author")
            .env("GIT_COMMITTER_EMAIL", "author@example.com")
            .env("GIT_AUTHOR_DATE", Self::FIXTURE_COMMIT_DATE)
            .env("GIT_COMMITTER_DATE", Self::FIXTURE_COMMIT_DATE)
            .output()
            .expect("git starts");
        assert!(
            output.status.success(),
            "git {arguments:?} fails: {output:?}"
        );
        String::from_utf8(output.stdout)
            .expect("git prints text")
            .trim()
            .to_string()
    }

    fn with_git_library(self, commit_override: Option<&str>) -> (Self, String) {
        let sandbox = self.with_fixture_tree("libraries");
        let commit = sandbox.git_repository("keymap");
        let project = format!(
            "(project app (libraries (keymap :git \"../keymap.git\" :rev \"{}\")))\n",
            commit_override.unwrap_or(&commit)
        );
        let sandbox = sandbox.with_file(
            &format!("git-app/{}", Language::project_file_name()),
            &project,
        );
        (sandbox, commit)
    }

    fn git_repository(&self, library: &str) -> String {
        let source = format!("{library}-source");
        Self::copy_tree(&self.directory.join(library), &self.directory.join(&source));
        if library == Self::LINKED_LIBRARY {
            std::os::unix::fs::symlink("..", self.directory.join(&source).join("outer"))
                .expect("the sandbox accepts symbolic links");
        }
        if library == Self::LINKED_MODULE {
            std::os::unix::fs::symlink(
                Self::OUTSIDE_FROM_STORE,
                self.directory.join(&source).join("src/other.knv"),
            )
            .expect("the sandbox accepts symbolic links");
        }
        self.git(&source, &["init", "--quiet"]);
        self.git(&source, &["add", "--all"]);
        self.git(
            &source,
            &["commit", "--quiet", "--message", "Add the library"],
        );
        let commit = self.git(&source, &["rev-parse", "HEAD"]);
        let bare = format!("{library}.git");
        self.git(".", &["clone", "--quiet", "--bare", &source, &bare]);
        commit
    }

    fn with_one_git_library(self, library: &str, url: &str) -> Self {
        self.with_fixture_tree("libraries")
            .with_one_git_library_from_tree(library, url)
    }

    fn with_one_git_library_from_tree(self, library: &str, url: &str) -> Self {
        let commit = self.git_repository(library);
        let project =
            format!("(project app (libraries ({library} :git \"{url}\" :rev \"{commit}\")))\n");
        self.with_file(
            &format!("git-app/{}", Language::project_file_name()),
            &project,
        )
    }

    fn masked(&self, text: &str) -> String {
        let canonical = fs::canonicalize(&self.directory).expect("the sandbox resolves");
        [canonical, self.directory.clone()]
            .iter()
            .fold(text.to_string(), |masked, spelling| {
                masked.replace(&spelling.display().to_string(), Self::SANDBOX_MASK)
            })
    }

    fn compiler_with_fake_git(&self, ssh_command: Option<&str>) -> Outcome {
        let fake_git = self.directory.join("fake-bin/git");
        fs::create_dir_all(fake_git.parent().expect("the fake git has a directory"))
            .expect("the sandbox accepts directories");
        fs::write(&fake_git, Self::FAKE_GIT).expect("the sandbox accepts files");
        fs::set_permissions(&fake_git, fs::Permissions::from_mode(Self::EXECUTABLE))
            .expect("the sandbox sets permissions");
        let mut command = self.command(Path::new(Self::COMPILER), &["check"]);
        command
            .current_dir(self.directory.join("git-app"))
            .env("PATH", self.directory.join("fake-bin"))
            .env_remove("GIT_SSH_COMMAND");
        if let Some(ssh_command) = ssh_command {
            command.env("GIT_SSH_COMMAND", ssh_command);
        }
        Outcome::of(&mut command)
    }

    fn compiler_with_path(&self, path_directory: &str, arguments: &[&str]) -> Outcome {
        let mut command = self.command(Path::new(Self::COMPILER), arguments);
        command.env("PATH", self.directory.join(path_directory));
        Outcome::of(&mut command)
    }

    fn compiler_with_temporary_directory(&self, directory: &str, arguments: &[&str]) -> Outcome {
        let mut command = self.command(Path::new(Self::COMPILER), arguments);
        command.env(
            Self::TEMPORARY_DIRECTORY_VARIABLE,
            self.directory.join(directory),
        );
        Outcome::of(&mut command)
    }

    fn compiler_with_rustfmt_time_limit(
        &self,
        time_limit: Duration,
        arguments: &[&str],
    ) -> Outcome {
        self.compiler_with_rustfmt_time_limit_value(&time_limit.as_millis().to_string(), arguments)
    }

    fn compiler_with_rustfmt_time_limit_value(&self, value: &str, arguments: &[&str]) -> Outcome {
        let mut command = self.command(Path::new(Self::COMPILER), arguments);
        command.env(EmittedFormatting::time_limit_variable(), value);
        Outcome::of(&mut command)
    }

    fn built_path(program: &str) -> PathBuf {
        Self::build_directory().join(program)
    }

    fn build_directory() -> PathBuf {
        Path::new(Self::BUILD_PARENT).join(Language::NAME)
    }

    fn built_program(&self, relative_path: impl AsRef<Path>) -> Outcome {
        Outcome::of(&mut self.command(&self.directory.join(relative_path), &[]))
    }

    fn outside_build_path(&self, working_directory: &str, built_path: &str) -> PathBuf {
        let sandbox = fs::canonicalize(&self.directory).expect("the sandbox directory resolves");
        let sandbox_without_root: PathBuf = sandbox
            .components()
            .filter(|component| matches!(component, Component::Normal(_)))
            .collect();
        Path::new(working_directory)
            .join(Self::build_directory())
            .join("outside")
            .join(sandbox_without_root)
            .join(built_path)
    }

    fn built_program_read_for_one_line(&self, relative_path: impl AsRef<Path>) -> Outcome {
        let mut child = self
            .command(&self.directory.join(relative_path), &[])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the built program starts");
        let mut first_line = String::new();
        BufReader::new(child.stdout.take().expect("stdout is piped"))
            .read_line(&mut first_line)
            .expect("the program prints a line");
        let output = child
            .wait_with_output()
            .expect("the program finishes after its reader closes");
        Outcome {
            output: Output {
                stdout: first_line.into_bytes(),
                ..output
            },
        }
    }

    fn built_program_output_before_its_input(
        &self,
        relative_path: impl AsRef<Path>,
        input: &str,
    ) -> String {
        let mut child = self
            .command(&self.directory.join(relative_path), &[])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("the built program starts");
        let mut stdout = child.stdout.take().expect("stdout is piped");
        let (sender, shown) = mpsc::channel();
        let reader = std::thread::spawn(move || TerminalScreen::forward(&mut stdout, &sender));
        let before_input = shown.recv_timeout(Self::INPUT_PATIENCE).unwrap_or_default();
        Self::offer_input(&mut child, input.as_bytes());
        child.wait().expect("the program finishes");
        reader.join().expect("stdout is read");
        String::from_utf8(before_input).expect("the program prints UTF-8")
    }

    fn offer_input(child: &mut Child, input: &[u8]) {
        let mut stdin = child.stdin.take().expect("stdin is piped");
        std::io::Write::write_all(&mut stdin, input)
            .or_else(Self::allow_exit_before_reading)
            .expect("the child accepts its input");
    }

    fn allow_exit_before_reading(error: std::io::Error) -> std::io::Result<()> {
        match error.kind() {
            ErrorKind::BrokenPipe => Ok(()),
            _ => Err(error),
        }
    }

    fn built_program_reading_its_directory(&self, relative_path: impl AsRef<Path>) -> Outcome {
        let directory = fs::File::open(&self.directory).expect("the sandbox directory opens");
        Outcome::of(
            self.command(&self.directory.join(relative_path), &[])
                .stdin(Stdio::from(directory)),
        )
    }

    fn compiler_with_colors(&self, environment: &[(&str, &str)], arguments: &[&str]) -> Outcome {
        Outcome::of(&mut self.color_command(Path::new(Self::COMPILER), environment, arguments))
    }

    fn compiler_on_a_terminal(&self, environment: &[(&str, &str)], arguments: &[&str]) -> Outcome {
        let recorded = Self::recorded_arguments(arguments);
        let recorded: Vec<&str> = recorded.iter().map(String::as_str).collect();
        Outcome::of(&mut self.color_command(
            Path::new(Self::TERMINAL_RECORDER),
            environment,
            &recorded,
        ))
    }

    #[cfg(target_os = "linux")]
    fn recorded_arguments(arguments: &[&str]) -> Vec<String> {
        let command_line = Self::compiler_command_line(arguments);
        vec!["-qec".to_string(), command_line, "/dev/null".to_string()]
    }

    #[cfg(target_os = "macos")]
    fn recorded_arguments(arguments: &[&str]) -> Vec<String> {
        let command_line = Self::compiler_command_line(arguments);
        ["-q", "/dev/null", "sh", "-c", &command_line]
            .iter()
            .map(|argument| argument.to_string())
            .collect()
    }

    fn compiler_command_line(arguments: &[&str]) -> String {
        let words: Vec<&str> = [Self::COMPILER].iter().chain(arguments).copied().collect();
        words.join(" ")
    }

    fn written_file(&self, name: &str) -> String {
        fs::read_to_string(self.directory.join(name)).expect("the command wrote the file")
    }

    fn color_command(
        &self,
        program: &Path,
        environment: &[(&str, &str)],
        arguments: &[&str],
    ) -> Command {
        let mut command = self.command(program, arguments);
        for variable in Self::COLOR_VARIABLES {
            command.env_remove(variable);
        }
        command.envs(environment.iter().copied());
        command
    }

    fn command(&self, program: &Path, arguments: &[&str]) -> Command {
        let mut command = Command::new(program);
        command.args(arguments).current_dir(&self.directory);
        Self::isolated(&mut command, &self.directory.join(Self::CACHE_DIRECTORY));
        command
    }

    fn isolated(command: &mut Command, cache: &Path) {
        command
            .env(Self::CACHE_HOME_VARIABLE, cache)
            .env_remove(Self::CARGO_TARGET_VARIABLE)
            .env_remove(Self::CARGO_BUILD_TARGET_VARIABLE);
    }

    fn unsandboxed_cache() -> PathBuf {
        std::env::temp_dir().join(format!(
            "{}-cli-{}-unsandboxed-cache",
            Language::NAME,
            std::process::id()
        ))
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.directory).ok();
    }
}

struct Outcome {
    output: Output,
}

enum Keys<'a> {
    AtPrompt(&'a str),
    AtPromptAfter(&'a str, &'a str),
    AfterOutput(&'a str, &'a str),
    BytesAtPrompt(&'a [u8]),
}

impl Keys<'_> {
    const END_OF_INPUT: &'static str = "\u{4}";
    const INTERRUPT: &'static str = "\u{3}";
    const LEFT: &'static str = "\u{1b}[D";
    const PASTE_START: &'static str = "\u{1b}[200~";
    const PASTE_END: &'static str = "\u{1b}[201~";
}

struct RenderedScreen {
    rows: Vec<Vec<char>>,
    row: usize,
    column: usize,
}

impl RenderedScreen {
    const ESCAPE: char = '\u{1b}';

    fn of(session: &str) -> String {
        let mut screen = Self {
            rows: vec![Vec::new()],
            row: 0,
            column: 0,
        };
        let mut characters = session.chars();
        while let Some(character) = characters.next() {
            screen.apply(character, &mut characters);
        }
        screen.text()
    }

    fn apply(&mut self, character: char, rest: &mut std::str::Chars) {
        match character {
            Self::ESCAPE => self.apply_sequence(rest),
            '\r' => self.column = 0,
            '\n' => self.move_down(),
            _ => self.write(character),
        }
    }

    fn apply_sequence(&mut self, rest: &mut std::str::Chars) {
        rest.next();
        let mut parameters = String::new();
        for character in rest.by_ref() {
            if character.is_ascii_alphabetic() {
                return self.apply_final(character, parameters.parse().unwrap_or(1));
            }
            parameters.push(character);
        }
    }

    fn apply_final(&mut self, last: char, count: usize) {
        match last {
            'A' => self.row = self.row.saturating_sub(count),
            'C' => self.column += count,
            'D' => self.column = self.column.saturating_sub(count),
            'K' => self.rows[self.row].truncate(self.column),
            _ => {}
        }
    }

    fn move_down(&mut self) {
        self.row += 1;
        if self.row == self.rows.len() {
            self.rows.push(Vec::new());
        }
    }

    fn write(&mut self, character: char) {
        let row = &mut self.rows[self.row];
        if row.len() <= self.column {
            row.resize(self.column, ' ');
            row.push(character);
        } else {
            row[self.column] = character;
        }
        self.column += 1;
    }

    fn text(&self) -> String {
        let rows: Vec<String> = self.rows.iter().map(|row| row.iter().collect()).collect();
        rows.join("\n")
    }
}

struct TerminalScreen {
    updates: Receiver<Vec<u8>>,
    shown: Vec<u8>,
    seen: usize,
}

impl TerminalScreen {
    const PROMPT_READY: &str = "\u{1b}[?2004h";
    const PATIENCE: Duration = Duration::from_secs(60);
    const CHUNK: usize = 4096;

    fn new(updates: Receiver<Vec<u8>>) -> Self {
        Self {
            updates,
            shown: Vec::new(),
            seen: 0,
        }
    }

    fn forward(stream: &mut impl std::io::Read, sender: &Sender<Vec<u8>>) -> Vec<u8> {
        let mut everything = Vec::new();
        let mut chunk = [0; Self::CHUNK];
        while let Ok(read @ 1..) = stream.read(&mut chunk) {
            everything.extend_from_slice(&chunk[..read]);
            sender.send(chunk[..read].to_vec()).ok();
        }
        everything
    }

    fn type_keys(&mut self, keys: &Keys, keyboard: &mut impl std::io::Write) -> Result<(), String> {
        let keys = match keys {
            Keys::AtPrompt(keys) => keys.as_bytes(),
            Keys::AtPromptAfter(output, keys) => {
                self.wait_for(output)?;
                keys.as_bytes()
            }
            Keys::AfterOutput(output, keys) => {
                self.wait_for(output)?;
                Self::press(keys.as_bytes(), keyboard);
                return Ok(());
            }
            Keys::BytesAtPrompt(bytes) => bytes,
        };
        self.wait_for(Self::PROMPT_READY)?;
        Self::press(keys, keyboard);
        Ok(())
    }

    fn press(keys: &[u8], keyboard: &mut impl std::io::Write) {
        keyboard
            .write_all(keys)
            .and_then(|()| keyboard.flush())
            .expect("the terminal recorder reads its input");
    }

    fn wait_for(&mut self, awaited: &str) -> Result<(), String> {
        while self.end_of_next(awaited).is_none() {
            let update = self.updates.recv_timeout(Self::PATIENCE).map_err(|_| {
                format!(
                    "the terminal never showed {awaited:?}: {:?}",
                    String::from_utf8_lossy(&self.shown)
                )
            })?;
            self.shown.extend(update);
        }
        self.seen = self
            .end_of_next(awaited)
            .expect("the awaited text was shown");
        Ok(())
    }

    fn end_of_next(&self, awaited: &str) -> Option<usize> {
        let awaited = awaited.as_bytes();
        self.shown[self.seen..]
            .windows(awaited.len())
            .position(|window| window == awaited)
            .map(|start| self.seen + start + awaited.len())
    }
}

impl Outcome {
    const PANIC_EXIT_CODE: i32 = 101;
    const PANIC_TEXT: &str = "panicked";

    const SIGABRT: i32 = 6;
    const SIGPIPE: i32 = 13;
    const ESCAPE: &str = "\u{1b}[";

    fn of(command: &mut Command) -> Self {
        let output = command
            .output()
            .unwrap_or_else(|error| panic!("cannot start {command:?}: {error}"));
        Self { output }
    }

    fn assert_no_panic(&self) {
        assert_ne!(self.output.status.code(), Some(Self::PANIC_EXIT_CODE));
        assert!(!self.stderr().contains(Self::PANIC_TEXT));
    }

    fn assert_exit_code(&self, expected: i32) -> &Self {
        assert_eq!(
            self.output.status.code(),
            Some(expected),
            "stdout:\n{}\nstderr:\n{}",
            self.stdout(),
            self.stderr()
        );
        self
    }

    fn assert_killed_by(&self, signal: i32) -> &Self {
        assert_eq!(
            self.output.status.signal(),
            Some(signal),
            "status: {}\nstdout:\n{}\nstderr:\n{}",
            self.output.status,
            self.stdout(),
            self.stderr()
        );
        self
    }

    fn assert_quiet_stop(&self) -> &Self {
        let status = self.output.status;
        assert!(
            status.code() == Some(0) || status.signal() == Some(Self::SIGPIPE),
            "status: {status}\nstderr:\n{}",
            self.stderr()
        );
        assert!(!self.stderr().contains("panicked"), "{}", self.stderr());
        self
    }

    fn assert_silent_success(&self) -> &Self {
        self.assert_exit_code(0);
        assert_eq!(self.stderr(), "");
        self
    }

    fn transcript(&self) -> String {
        format!(
            "exit code: {:?}\n--- stdout\n{}--- stderr\n{}",
            self.output.status.code(),
            self.stdout(),
            self.stderr()
        )
    }

    fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).into_owned()
    }

    fn stdout_is_colored(&self) -> bool {
        self.stdout().contains(Self::ESCAPE)
    }

    fn stderr_is_colored(&self) -> bool {
        self.stderr().contains(Self::ESCAPE)
    }

    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).into_owned()
    }

    fn counter_report(&self) -> Map<String, Value> {
        let stderr = self.stderr();
        let last_line = stderr
            .lines()
            .last()
            .expect("the counters follow the command");
        serde_json::from_str(last_line).expect("the last line is one JSON object")
    }

    fn json_replies(&self) -> Vec<Value> {
        self.stdout()
            .lines()
            .map(|line| serde_json::from_str(line).expect("each line is one JSON object"))
            .collect()
    }

    fn stage_work(&self) -> Map<String, Value> {
        self.counter_report()
            .into_iter()
            .filter(|(key, _)| !key.starts_with("allocator."))
            .collect()
    }

    fn work_beyond(&self, baseline: &Outcome) -> Value {
        let baseline = baseline.stage_work();
        self.stage_work()
            .into_iter()
            .map(|(key, count)| {
                let added = Self::count_of(&count) - Self::count_of(&baseline[key.as_str()]);
                (key, Value::from(added))
            })
            .collect()
    }

    fn count_of(count: &Value) -> i64 {
        count.as_i64().expect("each work count is a whole number")
    }
}

#[test]
fn help_lists_every_command() {
    let sandbox = Sandbox::new("help");
    let help = sandbox.compiler(&["--help"]);
    insta::assert_snapshot!(help.assert_exit_code(0).stdout());
}

#[test]
fn no_command_runs_piped_inputs_as_repl_does() {
    let sandbox = Sandbox::new("no_command");
    let bare = sandbox.compiler_reading(PROMPT_DEFINITIONS, &[]);
    let repl = sandbox.compiler_reading(PROMPT_DEFINITIONS, &["repl"]);
    assert_eq!(bare.transcript(), repl.transcript());
}

#[test]
fn no_command_takes_the_global_color_argument() {
    let sandbox = Sandbox::new("no_command_color");
    let bare = sandbox.compiler_reading(PROMPT_DEFINITIONS, &["--color", "never"]);
    let repl = sandbox.compiler_reading(PROMPT_DEFINITIONS, &["repl"]);
    assert_eq!(bare.transcript(), repl.transcript());
}

#[test]
fn counters_flag_prints_one_json_object_after_the_command() {
    let sandbox = Sandbox::new("counters").with_file("arithmetic.knv", ARITHMETIC);
    let plain = sandbox.compiler(&["emit", "arithmetic.knv"]);
    let counted = sandbox.compiler(&["--counters", "emit", "arithmetic.knv"]);
    assert_eq!(counted.assert_exit_code(0).stdout(), plain.stdout());
    let stderr = counted.stderr();
    let last_line = stderr
        .lines()
        .last()
        .expect("the counters follow the command");
    let report: serde_json::Value = serde_json::from_str(last_line).expect("the last line is JSON");
    assert!(report.is_object(), "{last_line}");
    #[cfg(not(feature = "counters"))]
    assert!(
        report
            .as_object()
            .is_some_and(|counters| !counters.keys().any(|key| key.starts_with("allocator."))),
        "{last_line}"
    );
}

#[test]
fn counters_flag_stays_out_of_the_help() {
    let sandbox = Sandbox::new("counters-help");
    let help = sandbox.compiler(&["--help"]);
    assert!(!help.assert_exit_code(0).stdout().contains("--counters"));
}

#[test]
fn counters_flag_counts_a_program_small_enough_to_count_by_hand() {
    let sandbox = Sandbox::new("counters-by-hand")
        .with_file("empty.knv", EMPTY_MAIN)
        .with_file("branch.knv", ONE_BRANCH);
    let empty = sandbox.compiler(&["--counters", "emit", "empty.knv"]);
    let branch = sandbox.compiler(&["--counters", "emit", "branch.knv"]);
    assert_eq!(
        branch
            .assert_exit_code(0)
            .work_beyond(empty.assert_exit_code(0)),
        json!({
            "expand.lowered_forms": 3,
            "check.inferred_expressions": 3,
            "emit.emitted_expressions": 2,
            "build.scanned_tokens": 8,
        })
    );
}

#[test]
fn counters_flag_prints_no_build_work_when_rustfmt_fails() {
    let sandbox = Sandbox::new("counters-unformatted")
        .with_file("branch.knv", ONE_BRANCH)
        .with_executable("tools/rustfmt", FAILING_RUSTFMT);
    let emit = sandbox.compiler_with_path("tools", &["--counters", "emit", "branch.knv"]);
    let work = emit.assert_exit_code(0).stage_work();
    assert_eq!(
        work.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "check.inferred_expressions",
            "emit.emitted_expressions",
            "expand.lowered_forms"
        ]
    );
}

#[test]
fn counters_flag_prints_the_work_of_each_stage_that_ran() {
    let sandbox = Sandbox::new("counters-stages").with_file("arithmetic.knv", ARITHMETIC);
    let emit = ["--counters", "emit", "arithmetic.knv"];
    let emitted = sandbox.compiler(&emit).assert_exit_code(0).stage_work();
    for key in STAGE_WORK_KEYS {
        assert!(
            emitted[key].as_u64().is_some_and(|count| count > 0),
            "{key} in {emitted:?}"
        );
    }
    let checked = sandbox
        .compiler(&["--counters", "check", "arithmetic.knv"])
        .assert_exit_code(0)
        .stage_work();
    assert_eq!(
        checked.keys().map(String::as_str).collect::<Vec<_>>(),
        ["check.inferred_expressions", "expand.lowered_forms"]
    );
    assert_eq!(sandbox.compiler(&emit).stage_work(), emitted);
}

#[test]
fn unknown_command_is_a_usage_error() {
    let sandbox = Sandbox::new("unknown_command");
    sandbox.compiler(&["compile"]).assert_exit_code(2);
}

#[test]
fn unreadable_file_is_a_usage_error() {
    let sandbox = Sandbox::new("unreadable_file");
    let check = sandbox.compiler(&["check", "missing.knv"]);
    assert!(
        check
            .assert_exit_code(2)
            .stderr()
            .starts_with("error: cannot read `missing.knv`")
    );
}

#[test]
fn fmt_rewrites_a_file_in_place_and_prints_nothing() {
    let sandbox = Sandbox::new("fmt_rewrites").with_file("twice.knv", UNFORMATTED);
    let formatted = sandbox.compiler(&["fmt", "twice.knv"]);
    formatted.assert_silent_success();
    assert_eq!(
        (formatted.stdout(), sandbox.contents("twice.knv")),
        (String::new(), FORMATTED.to_string())
    );
}

#[test]
fn fmt_check_prints_each_file_that_would_change_and_writes_nothing() {
    let sandbox = Sandbox::new("fmt_check_changes")
        .with_file("twice.knv", UNFORMATTED)
        .with_file("done.knv", FORMATTED);
    let check = sandbox.compiler(&["fmt", "--check", "twice.knv", "done.knv"]);
    check.assert_exit_code(1);
    assert_eq!(
        (check.stdout(), sandbox.contents("twice.knv")),
        ("twice.knv\n".to_string(), UNFORMATTED.to_string())
    );
}

#[test]
fn fmt_check_exits_0_when_every_file_is_formatted() {
    let sandbox = Sandbox::new("fmt_check_formatted").with_file("done.knv", FORMATTED);
    let check = sandbox.compiler(&["fmt", "--check", "done.knv"]);
    check.assert_silent_success();
    assert_eq!(check.stdout(), "");
}

#[test]
fn fmt_reads_standard_input_and_writes_the_formatted_text() {
    let sandbox = Sandbox::new("fmt_standard_input");
    let formatted = sandbox.compiler_reading(UNFORMATTED, &["fmt", "-"]);
    formatted.assert_silent_success();
    assert_eq!(formatted.stdout(), FORMATTED);
}

#[test]
fn fmt_of_standard_input_reads_shapes_from_modules_beside_its_stdin_path() {
    let sandbox =
        Sandbox::new("fmt_standard_input_shapes").with_file("shapes/loops.knv", LOOP_SHAPE);
    let formatted = sandbox.compiler_reading(
        LOOP_CALL,
        &["fmt", "--stdin-path", "programs/main.knv", "-"],
    );
    formatted.assert_silent_success();
    assert_eq!(formatted.stdout(), LOOP_CALL_SHAPED);
}

#[test]
fn fmt_formats_every_source_file_under_a_directory() {
    let sandbox = Sandbox::new("fmt_directory")
        .with_file("project/twice.knv", UNFORMATTED)
        .with_file("project/nested/limit.knv", UNFORMATTED)
        .with_file("project/notes.txt", UNFORMATTED);
    sandbox
        .compiler(&["fmt", "project"])
        .assert_silent_success();
    assert_eq!(
        [
            sandbox.contents("project/twice.knv"),
            sandbox.contents("project/nested/limit.knv"),
            sandbox.contents("project/notes.txt"),
        ],
        [FORMATTED, FORMATTED, UNFORMATTED].map(String::from)
    );
}

#[test]
fn fmt_check_of_standard_input_that_would_change_prints_its_name_and_exits_1() {
    let sandbox = Sandbox::new("fmt_check_standard_input_changes");
    let check = sandbox.compiler_reading(UNFORMATTED, &["fmt", "--check", "-"]);
    check.assert_exit_code(1);
    assert_eq!(check.stdout(), "<stdin>\n");
}

#[test]
fn fmt_check_of_formatted_standard_input_prints_nothing() {
    let sandbox = Sandbox::new("fmt_check_standard_input_formatted");
    let check = sandbox.compiler_reading(FORMATTED, &["fmt", "--check", "-"]);
    check.assert_silent_success();
    assert_eq!(check.stdout(), "");
}

#[test]
fn fmt_exits_2_when_an_unreadable_file_follows_a_reader_error() {
    let sandbox =
        Sandbox::new("fmt_unreadable_after_reader_error").with_file("unclosed.knv", UNCLOSED);
    sandbox
        .compiler(&["fmt", "unclosed.knv", "missing.knv"])
        .assert_exit_code(2);
}

#[test]
fn fmt_skips_a_symlinked_directory() {
    let sandbox = Sandbox::new("fmt_symlinked_directory")
        .with_file("project/twice.knv", UNFORMATTED)
        .with_file("elsewhere/limit.knv", UNFORMATTED);
    std::os::unix::fs::symlink(
        sandbox.directory.join("elsewhere"),
        sandbox.directory.join("project/linked"),
    )
    .expect("the sandbox accepts a symlink");
    sandbox
        .compiler(&["fmt", "project"])
        .assert_silent_success();
    assert_eq!(
        [
            sandbox.contents("project/twice.knv"),
            sandbox.contents("elsewhere/limit.knv"),
        ],
        [FORMATTED, UNFORMATTED].map(String::from)
    );
}

#[test]
fn fmt_skips_hidden_directories_and_the_project_build_directory() {
    let sandbox = Sandbox::new("fmt_skipped_directories")
        .with_file(&Language::project_file_name(), PROJECT)
        .with_file("twice.knv", UNFORMATTED)
        .with_file(".direnv/limit.knv", UNFORMATTED)
        .with_file("target/limit.knv", UNFORMATTED)
        .with_file("src/target/limit.knv", UNFORMATTED)
        .with_file("tests/target/limit.knv", UNFORMATTED);
    sandbox.compiler(&["fmt", "."]).assert_silent_success();
    assert_eq!(
        [
            "twice.knv",
            ".direnv/limit.knv",
            "target/limit.knv",
            "src/target/limit.knv",
            "tests/target/limit.knv",
        ]
        .map(|file| sandbox.contents(file)),
        [FORMATTED, UNFORMATTED, UNFORMATTED, FORMATTED, FORMATTED].map(String::from)
    );
}

#[test]
fn fmt_in_a_project_walks_a_target_directory_below_the_root() {
    let sandbox = Sandbox::new("fmt_project_inner_target")
        .with_file(&Language::project_file_name(), PROJECT)
        .with_file("src/target/limit.knv", UNFORMATTED);
    sandbox.compiler(&["fmt", "src"]).assert_silent_success();
    assert_eq!(sandbox.contents("src/target/limit.knv"), FORMATTED);
}

#[test]
fn fmt_outside_a_project_skips_only_the_target_directory_of_its_walk() {
    let sandbox = Sandbox::new("fmt_skipped_walk_target")
        .with_file("loose/target/limit.knv", UNFORMATTED)
        .with_file("loose/src/target/limit.knv", UNFORMATTED);
    sandbox.compiler(&["fmt", "loose"]).assert_silent_success();
    assert_eq!(
        [
            sandbox.contents("loose/target/limit.knv"),
            sandbox.contents("loose/src/target/limit.knv"),
        ],
        [UNFORMATTED, FORMATTED].map(String::from)
    );
}

#[test]
fn fmt_reports_a_reader_error_and_leaves_the_file_as_it_was() {
    let sandbox = Sandbox::new("fmt_reader_error")
        .with_file("unclosed.knv", UNCLOSED)
        .with_file("twice.knv", UNFORMATTED);
    let formatted = sandbox.compiler(&["fmt", "unclosed.knv", "twice.knv"]);
    formatted.assert_exit_code(1);
    assert!(
        formatted.stderr().contains("error[L0101]"),
        "{}",
        formatted.stderr()
    );
    assert_eq!(
        (
            sandbox.contents("unclosed.knv"),
            sandbox.contents("twice.knv")
        ),
        (UNCLOSED.to_string(), FORMATTED.to_string())
    );
}

#[test]
fn fmt_of_a_missing_file_is_a_usage_error() {
    let sandbox = Sandbox::new("fmt_missing_file");
    let formatted = sandbox.compiler(&["fmt", "missing.knv"]);
    assert!(
        formatted
            .assert_exit_code(2)
            .stderr()
            .starts_with("error: cannot read `missing.knv`")
    );
}

#[test]
fn fmt_without_a_path_outside_a_project_is_a_usage_error() {
    let fmt = Sandbox::new("fmt_without_path").compiler(&["fmt"]);
    insta::assert_snapshot!(fmt.assert_exit_code(2).stderr());
}

#[test]
fn fmt_of_standard_input_with_another_path_is_a_usage_error() {
    let sandbox = Sandbox::new("fmt_standard_input_and_path").with_file("done.knv", FORMATTED);
    let formatted = sandbox.compiler_reading(FORMATTED, &["fmt", "-", "done.knv"]);
    assert!(
        formatted
            .assert_exit_code(2)
            .stderr()
            .starts_with("error: `-` reads standard input")
    );
}

#[test]
fn fmt_of_unread_standard_input_with_another_path_is_a_usage_error() {
    let sandbox = Sandbox::new("fmt_unread_standard_input").with_file("done.knv", FORMATTED);
    let unread = vec![b' '; Sandbox::PIPE_OVERFLOWING_INPUT_BYTES];
    let formatted = sandbox.compiler_reading(unread, &["fmt", "-", "done.knv"]);
    assert!(
        formatted
            .assert_exit_code(2)
            .stderr()
            .starts_with("error: `-` reads standard input")
    );
}

#[test]
fn a_child_that_exits_before_reading_its_input_is_allowed() {
    let closed_pipe = std::io::Error::from(ErrorKind::BrokenPipe);
    assert!(Sandbox::allow_exit_before_reading(closed_pipe).is_ok());
}

#[test]
fn any_other_failure_to_give_a_child_its_input_stays_an_error() {
    let refused = std::io::Error::from(ErrorKind::PermissionDenied);
    assert!(Sandbox::allow_exit_before_reading(refused).is_err());
}

#[test]
fn fmt_with_a_stdin_path_but_no_standard_input_is_a_usage_error() {
    let sandbox = Sandbox::new("fmt_stdin_path_alone").with_file("done.knv", FORMATTED);
    sandbox
        .compiler(&["fmt", "--stdin-path", "done.knv", "done.knv"])
        .assert_exit_code(2);
}

#[test]
fn check_is_silent_on_a_valid_program() {
    let sandbox = Sandbox::new("check_valid").with_file("greeting.knv", GREETING);
    let check = sandbox.compiler(&["check", "greeting.knv"]);
    check.assert_exit_code(0);
    assert_eq!(
        (check.stdout(), check.stderr()),
        (String::new(), String::new())
    );
}

#[test]
fn check_ignores_a_byte_order_mark() {
    let sandbox = Sandbox::new("check_byte_order_mark").with_file("marked.knv", MARKED_PROGRAM);
    sandbox
        .compiler(&["check", "marked.knv"])
        .assert_exit_code(0);
}

#[test]
fn types_prints_each_function() {
    let sandbox = Sandbox::new("types").with_file("greeting.knv", GREETING);
    let types = sandbox.compiler(&["types", "greeting.knv"]);
    insta::assert_snapshot!(types.assert_exit_code(0).stdout());
}

#[test]
fn types_prints_inferred_struct_derives() {
    let sandbox = Sandbox::new("types_structs").with_file("structs.knv", STRUCTS);
    let types = sandbox.compiler(&["types", "structs.knv"]);
    insta::assert_snapshot!(types.assert_exit_code(0).stdout());
}

#[test]
fn signatures_is_an_unknown_command() {
    let sandbox = Sandbox::new("signatures_unknown").with_file("greeting.knv", GREETING);
    let signatures = sandbox.compiler(&["signatures", "greeting.knv"]);
    let stderr = signatures.assert_exit_code(2).stderr();
    assert_eq!(
        stderr.lines().next(),
        Some("error: unrecognized subcommand 'signatures'")
    );
}

#[test]
fn doc_ends_quietly_when_its_reader_closes() {
    let sandbox = Sandbox::new("doc_closed_pipe");
    sandbox
        .compiler_writing_to_a_closed_pipe(&["doc", "if"])
        .assert_silent_success();
}

#[test]
#[cfg(target_os = "linux")]
fn doc_reports_a_standard_output_it_cannot_write() {
    let sandbox = Sandbox::new("doc_unwritable");
    let doc = sandbox.compiler_writing_to_a_full_device(&["doc", "if"]);
    assert!(
        doc.assert_exit_code(2)
            .stderr()
            .starts_with("error: cannot write standard output: "),
        "{}",
        doc.stderr()
    );
}

#[test]
fn doc_keeps_its_exit_code_when_its_error_reader_closes() {
    let sandbox = Sandbox::new("doc_closed_error_pipe").with_file("unbound.knv", UNBOUND_NAME);
    let arguments = ["doc", "--file", "unbound.knv", "main"];
    sandbox.assert_exit_code_with_a_closed_error_pipe(&arguments, 1);
}

#[test]
fn check_keeps_its_exit_code_when_its_error_reader_closes() {
    let sandbox = Sandbox::new("check_closed_error_pipe").with_file("unbound.knv", UNBOUND_NAME);
    let arguments = ["check", "unbound.knv"];
    sandbox.assert_exit_code_with_a_closed_error_pipe(&arguments, 1);
}

#[test]
fn run_keeps_its_exit_code_when_its_error_reader_closes() {
    let sandbox = Sandbox::new("run_closed_error_pipe").with_file("unbound.knv", UNBOUND_NAME);
    let arguments = ["run", "unbound.knv"];
    sandbox.assert_exit_code_with_a_closed_error_pipe(&arguments, 1);
}

#[test]
fn emit_ends_quietly_when_its_reader_closes() {
    let sandbox = Sandbox::new("emit_closed_pipe").with_file("greeting.knv", GREETING);
    sandbox
        .compiler_writing_to_a_closed_pipe(&["emit", "greeting.knv"])
        .assert_silent_success();
}

#[test]
fn expand_ends_quietly_when_its_reader_closes() {
    let sandbox = Sandbox::new("expand_closed_pipe").with_file("greeting.knv", GREETING);
    sandbox
        .compiler_writing_to_a_closed_pipe(&["expand", "greeting.knv"])
        .assert_silent_success();
}

#[test]
fn types_ends_quietly_when_its_reader_closes() {
    let sandbox = Sandbox::new("types_closed_pipe").with_file("greeting.knv", GREETING);
    sandbox
        .compiler_writing_to_a_closed_pipe(&["types", "greeting.knv"])
        .assert_silent_success();
}

#[test]
fn doc_prints_the_page_of_a_prelude_term() {
    let sandbox = Sandbox::new("doc_prelude");
    let doc = sandbox.compiler(&["doc", "defn"]);
    assert_eq!(
        doc.assert_exit_code(0).stdout(),
        "(defmacro defn (name parameters body ...))\n\nDefines a function: (defn name (parameter ...) body ...).\n\nA string before at least one more body form is the docstring.\n\n```\n(defn twice (x) \"Doubles a number.\" (* 2 x))\n(twice 4)\n=> 8\n```\n"
    );
}

#[test]
fn doc_prints_the_header_that_documents_a_prelude_term() {
    let sandbox = Sandbox::new("doc_document_header");
    let doc = sandbox.compiler(&["doc", "str"]);
    assert_eq!(
        doc.assert_exit_code(0).stdout(),
        "(str part ...)\n\nBuilds a `string` from its parts.\n\nA string literal part is literal text, and any other part prints with\n`display`. `(str x)` emits `x.to_string()`, and `str` as a value is that\none-part function, so `(map (iter years) str)` turns each year into a\n`string`.\n\n(str \"year \" 2026)\n=> \"year 2026\"\n\n(collect (map (iter (vec 1 2)) str))\n=> (vec (str 1) (str 2))\n"
    );
}

#[test]
fn doc_prints_the_page_of_a_kernel_form() {
    let sandbox = Sandbox::new("doc_kernel_form");
    let doc = sandbox.compiler(&["doc", "match"]);
    assert_eq!(
        doc.assert_exit_code(0).stdout(),
        "(match value (pattern body ...) ...)\n\nEvaluates the body of the first arm whose pattern matches the value.\n\nAn arm's body is a `do`, so its last form is the arm's value.\n\nA pattern is `_`, a literal, a bare name, `(variant pattern ...)`,\n`(variant :field pattern ...)`, `(type-name pattern ...)` or\n`(type-name :field pattern ...)`. It emits a Rust `match`.\n\n(match 3\n  (0 \"zero\")\n  (_ \"other\"))\n=> \"other\"\n\n(match (pair 1 2)\n  ((pair left _) left))\n=> 1\n"
    );
}

#[test]
fn doc_prints_the_page_of_a_kernel_type() {
    let sandbox = Sandbox::new("doc_kernel_type");
    let doc = sandbox.compiler(&["doc", "i64"]);
    assert_eq!(
        doc.assert_exit_code(0).stdout(),
        "i64\n\nHolds a signed 64-bit integer, as Rust's `i64` does.\n\nAn integer literal whose type stays open defaults to `i64`.\n\n(the i64 3)\n=> 3\n\n(infer 1)\n=> \"i64\"\n"
    );
}

#[test]
fn doc_prints_the_page_of_a_kernel_constant() {
    let sandbox = Sandbox::new("doc_kernel_constant");
    let doc = sandbox.compiler(&["doc", "true"]);
    assert_eq!(doc.assert_exit_code(0).stdout(), TRUE_PAGE);
}

#[test]
fn doc_prints_the_call_form_of_template() {
    let sandbox = Sandbox::new("doc_template");
    let doc = sandbox.compiler(&["doc", "template"]);
    assert_eq!(
        doc.assert_exit_code(0).stdout(),
        "(template form)\n\nBuilds a form from a quoted form with holes in it.\n\n`(unquote x)` puts the value of `x` in place, and `(splice xs)` puts each\nform of the list `xs` in place, as in\n`(template (def (unquote name) (splice body)))`.\n\n```\n(defmacro twice (x) (template (* 2 (unquote x))))\n(twice 4)\n=> 8\n```\n"
    );
}

#[test]
fn doc_takes_a_term_that_starts_with_a_hyphen() {
    let sandbox = Sandbox::new("doc_hyphen");
    let doc = sandbox.compiler(&["doc", "->"]);
    let page = doc.assert_exit_code(0).stdout();
    assert_eq!(page.lines().next(), Some(THREADING_HEADER));
}

#[test]
fn doc_apropos_lists_the_prelude_names_that_hold_the_text() {
    let sandbox = Sandbox::new("doc_apropos");
    let doc = sandbox.compiler(&["doc", "(apropos sort)"]);
    assert_eq!(
        doc.assert_exit_code(0).stdout(),
        "(apropos sort)\n\ninto-sorted-map sort! sort-by! sorted-map\n"
    );
}

#[test]
fn doc_apropos_ignores_case() {
    let sandbox = Sandbox::new("doc_apropos_case");
    let doc = sandbox.compiler(&["doc", "(apropos SORT)"]);
    assert_eq!(
        doc.assert_exit_code(0).stdout(),
        "(apropos SORT)\n\ninto-sorted-map sort! sort-by! sorted-map\n"
    );
}

#[test]
fn doc_apropos_without_a_match_points_to_find_doc() {
    let sandbox = Sandbox::new("doc_apropos_none");
    let doc = sandbox.compiler(&["doc", "(apropos zzqx)"]);
    assert_eq!(
        doc.assert_exit_code(0).stdout(),
        "(apropos zzqx)\n\nNo matches. Try (find-doc zzqx) to search docstrings.\n"
    );
}

#[test]
fn doc_find_doc_without_a_match_succeeds() {
    let sandbox = Sandbox::new("doc_find_doc_none");
    let doc = sandbox.compiler(&["doc", "(find-doc zzqx)"]);
    assert_eq!(
        doc.assert_exit_code(0).stdout(),
        "(find-doc zzqx)\n\nNo matches.\n"
    );
}

#[test]
fn doc_finds_the_page_of_assert_imported_from_prelude_test() {
    let sandbox = Sandbox::new("doc_imported_assert").with_file("main.knv", PADDING_TESTS);
    let doc = sandbox.compiler(&["doc", "--file", "main.knv", "assert"]);
    let page = doc.assert_exit_code(0).stdout();
    assert!(
        page.starts_with("(assert form)\n\nEnds the test with a failure unless `form` is true.")
    );
}

#[test]
fn doc_find_doc_finds_a_term_through_the_body_of_its_docstring() {
    let sandbox = Sandbox::new("doc_find_doc_body").with_file("main.knv", DOCUMENTED_TWICE);
    let doc = sandbox.compiler(&["doc", "--file", "main.knv", "(find-doc widgets)"]);
    assert_eq!(
        doc.assert_exit_code(0).stdout(),
        "(find-doc widgets)\n\n(defn twice (x))\n    Doubles a number.\n"
    );
}

#[test]
fn doc_takes_a_search_text_that_starts_with_hyphens() {
    let sandbox = Sandbox::new("doc_search_hyphens");
    let doc = sandbox.compiler(&["doc", "(find-doc --zzqx)"]);
    assert_eq!(
        doc.assert_exit_code(0).stdout(),
        "(find-doc --zzqx)\n\nNo matches.\n"
    );
}

#[test]
fn doc_dir_lists_the_exports_of_a_module() {
    let sandbox = Sandbox::new("doc_dir_module")
        .with_file("geometry.knv", GEOMETRY)
        .with_file("main.knv", GEOMETRY_ENTRY);
    let doc = sandbox.compiler(&["doc", "--file", "main.knv", "(dir geometry)"]);
    assert_eq!(doc.assert_exit_code(0).stdout(), "(dir geometry)\n\narea\n");
}

#[test]
fn doc_dir_lists_the_names_of_the_entry_module() {
    let sandbox = Sandbox::new("doc_dir_entry").with_file("main.knv", DOCUMENTED_TWICE);
    let doc = sandbox.compiler(&["doc", "--file", "main.knv", "(dir)"]);
    assert_eq!(doc.assert_exit_code(0).stdout(), "(dir)\n\nmain twice\n");
}

#[test]
fn doc_write_writes_a_page_per_module_and_the_marker() {
    let sandbox = Sandbox::with_documented_project("doc_write_pages");
    sandbox
        .compiler(&["doc", "--write"])
        .assert_silent_success();
    assert_eq!(
        sandbox.files_under("doc"),
        [".knave-doc", "geometry/area.md", "main.md"]
    );
    assert_eq!(
        sandbox.contents("doc/geometry/area.md"),
        format!("# geometry/area\n\n{AREA_SECTION}")
    );
}

#[test]
fn doc_write_removes_the_page_of_a_deleted_module() {
    let sandbox = Sandbox::with_documented_project("doc_write_deleted");
    sandbox
        .compiler(&["doc", "--write"])
        .assert_silent_success();
    let sandbox = sandbox.without_file("src/geometry/area.knv");
    sandbox
        .compiler(&["doc", "--write"])
        .assert_silent_success();
    assert_eq!(sandbox.files_under("doc"), [".knave-doc", "main.md"]);
}

#[test]
fn doc_write_refuses_a_folder_that_holds_other_files() {
    let sandbox = Sandbox::with_documented_project("doc_write_foreign")
        .with_file("site/content/install.md", "kept\n");
    let write = sandbox.compiler(&["doc", "--write", "--output", "site/content"]);
    assert_eq!(
        write.assert_exit_code(2).stderr(),
        "error: `site/content` holds files that `knave doc --write` did not write\nhelp: name an empty folder with `--output`, or empty `site/content`\n"
    );
    assert_eq!(sandbox.files_under("site/content"), ["install.md"]);
    assert_eq!(sandbox.contents("site/content/install.md"), "kept\n");
}

#[test]
fn doc_write_writes_into_an_empty_output_folder() {
    let sandbox = Sandbox::with_documented_project("doc_write_empty_output");
    fs::create_dir_all(sandbox.directory.join("pages")).expect("the sandbox makes folders");
    sandbox
        .compiler(&["doc", "--write", "--output", "pages"])
        .assert_silent_success();
    assert_eq!(
        sandbox.files_under("pages"),
        [".knave-doc", "geometry/area.md", "main.md"]
    );
}

#[test]
fn doc_write_reports_an_output_that_is_a_file() {
    let sandbox = Sandbox::with_documented_project("doc_write_file_output");
    let write = sandbox.compiler(&["doc", "--write", "--output", "project.knv"]);
    assert!(
        write
            .assert_exit_code(2)
            .stderr()
            .starts_with("error: cannot write the pages into `project.knv`: "),
        "{}",
        write.stderr()
    );
}

#[test]
fn doc_write_reports_a_folder_it_cannot_write() {
    let sandbox = Sandbox::with_documented_project("doc_write_read_only");
    let folder = sandbox.directory.join("pages");
    fs::create_dir_all(&folder).expect("the sandbox makes folders");
    fs::set_permissions(&folder, fs::Permissions::from_mode(Sandbox::READ_ONLY))
        .expect("the sandbox marks folders read-only");
    let write = sandbox.compiler(&["doc", "--write", "--output", "pages"]);
    fs::set_permissions(&folder, fs::Permissions::from_mode(Sandbox::EXECUTABLE))
        .expect("the sandbox restores the folder");
    assert!(
        write
            .assert_exit_code(2)
            .stderr()
            .starts_with("error: cannot write the pages into `pages`: "),
        "{}",
        write.stderr()
    );
}

#[test]
fn doc_write_shows_no_type_for_a_macro_or_a_struct() {
    let sandbox = Sandbox::with_documented_project("doc_write_untyped")
        .with_file("src/shapes.knv", UNTYPED_SHAPES);
    sandbox
        .compiler(&["doc", "--write"])
        .assert_silent_success();
    assert_eq!(sandbox.contents("doc/shapes.md"), UNTYPED_SHAPES_PAGE);
}

#[test]
fn doc_write_prelude_writes_a_page_per_public_prelude_module() {
    let sandbox = Sandbox::new("doc_write_prelude");
    sandbox
        .compiler(&["doc", "--write", "--prelude"])
        .assert_silent_success();
    let pages: Vec<String> = PRELUDE_PAGES
        .iter()
        .map(|page| format!("prelude/{page}.md"))
        .collect();
    let mut expected = [
        vec![".knave-doc".to_string(), "prelude.md".to_string()],
        pages,
    ]
    .concat();
    expected.sort();
    assert_eq!(sandbox.files_under("doc"), expected);
    assert!(
        sandbox
            .contents("doc/prelude/option.md")
            .contains(UNWRAP_OR_SECTION)
    );
    assert!(
        sandbox
            .contents("doc/prelude.md")
            .contains(PRELUDE_REEXPORT)
    );
}

#[test]
fn doc_write_reads_a_project_whose_entry_sits_at_its_root() {
    let sandbox = Sandbox::new("doc_write_root_entry")
        .with_file(&Language::project_file_name(), ROOT_ENTRY_PROJECT)
        .with_file("main.knv", ROOT_ENTRY);
    sandbox
        .compiler(&["doc", "--write"])
        .assert_silent_success();
    assert_eq!(sandbox.files_under("doc"), [".knave-doc", "main.md"]);
}

#[test]
fn doc_write_leaves_out_the_tasks_and_tests_folders() {
    let sandbox = Sandbox::new("doc_write_tasks_and_tests")
        .with_file(&Language::project_file_name(), ROOT_ENTRY_PROJECT)
        .with_file("main.knv", ROOT_ENTRY)
        .with_file("tasks/build.knv", HELPER_TASK)
        .with_file("tests/build.knv", HELPER_TASK_TEST);
    sandbox
        .compiler(&["doc", "--write"])
        .assert_silent_success();
    assert_eq!(sandbox.files_under("doc"), [".knave-doc", "main.md"]);
}

#[test]
fn doc_write_leaves_out_a_library_inside_the_project() {
    let sandbox = Sandbox::new("doc_write_inner_library")
        .with_file(&Language::project_file_name(), ROOT_ENTRY_PROJECT)
        .with_file("main.knv", ROOT_ENTRY)
        .with_file("vendor/hints/project.knv", "(project hints)\n")
        .with_file(
            "vendor/hints/src/lib.knv",
            "(use labels label)\n\n(export hint)\n\n(def hint label)\n",
        )
        .with_file(
            "vendor/hints/src/labels.knv",
            "(export label)\n\n(def label 1)\n",
        );
    sandbox
        .compiler(&["doc", "--write"])
        .assert_silent_success();
    assert_eq!(sandbox.files_under("doc"), [".knave-doc", "main.md"]);
}

#[test]
fn doc_write_stays_inside_the_source_root() {
    let sandbox = Sandbox::with_documented_project("doc_write_outside_link")
        .with_file("elsewhere/far.knv", "(export far)\n\n(def far 1)\n");
    std::os::unix::fs::symlink(
        sandbox.directory.join("elsewhere"),
        sandbox.directory.join("src/linked"),
    )
    .expect("the sandbox makes links");
    sandbox
        .compiler(&["doc", "--write"])
        .assert_silent_success();
    assert_eq!(
        sandbox.files_under("doc"),
        [".knave-doc", "geometry/area.md", "main.md"]
    );
}

#[test]
fn doc_write_reads_only_source_files_outside_hidden_folders() {
    let sandbox = Sandbox::with_documented_project("doc_write_hidden")
        .with_file("src/.cache/old.knv", "(export old)\n\n(def old 1)\n")
        .with_file("src/notes.txt", "(export notes)\n");
    sandbox
        .compiler(&["doc", "--write"])
        .assert_silent_success();
    assert_eq!(
        sandbox.files_under("doc"),
        [".knave-doc", "geometry/area.md", "main.md"]
    );
}

#[test]
fn doc_write_refuses_a_file_that_cannot_be_a_module() {
    let sandbox =
        Sandbox::with_documented_project("doc_write_unnamable").with_file("src/odd name.knv", "");
    let write = sandbox.compiler(&["doc", "--write"]);
    assert_eq!(
        write.assert_exit_code(2).stderr(),
        "error: `src/odd name.knv` cannot be a module, because a module path cannot hold a space, a parenthesis, a quote or a semicolon\nhelp: rename the file\n"
    );
}

#[test]
fn doc_write_refuses_a_file_that_takes_a_compiler_module() {
    let sandbox = Sandbox::with_documented_project("doc_write_compiler_module")
        .with_file("src/prelude/extra.knv", "");
    let write = sandbox.compiler(&["doc", "--write"]);
    assert!(
        write.assert_exit_code(1).stderr().contains("error[L0257]"),
        "{}",
        write.stderr()
    );
    assert!(!sandbox.directory.join("doc").exists());
}

#[test]
fn doc_write_gives_each_page_the_type_of_its_own_name() {
    let sandbox = Sandbox::new("doc_write_same_name")
        .with_file(&Language::project_file_name(), DOCUMENTED_PROJECT)
        .with_file(
            "src/a.knv",
            "(export size)\n\n(defn size () \"Counts.\" 1)\n",
        )
        .with_file(
            "src/b.knv",
            "(export size)\n\n(defn size () \"Names.\" \"big\")\n",
        );
    sandbox
        .compiler(&["doc", "--write"])
        .assert_silent_success();
    assert!(
        sandbox
            .contents("doc/a.md")
            .contains("\n(type size (fn () i64))\n```")
    );
    assert!(
        sandbox
            .contents("doc/b.md")
            .contains("\n(type size (fn () (ref str)))\n```")
    );
}

#[test]
fn doc_write_shows_the_type_a_module_gives_its_own_name() {
    let sandbox = Sandbox::new("doc_write_own_type")
        .with_file(&Language::project_file_name(), DOCUMENTED_PROJECT)
        .with_file(
            "src/main.knv",
            "(use geometry/area square-area)\n\n(defn main () (println (square-area 2)))\n",
        )
        .with_file(
            "src/geometry/area.knv",
            "(export square-area)\n\n(defn square-area (side) \"Squares a side.\" (* side side))\n",
        );
    sandbox
        .compiler(&["doc", "--write"])
        .assert_silent_success();
    assert!(
        sandbox
            .contents("doc/geometry/area.md")
            .contains("\n(type square-area (where ((a number)) (fn (a) a)))\n```")
    );
}

#[test]
fn doc_write_prelude_ignores_a_broken_project_file() {
    let sandbox = Sandbox::new("doc_write_prelude_broken_project")
        .with_file(&Language::project_file_name(), "(project\n")
        .with_file("src/notes.knv", "");
    sandbox
        .compiler_in("src", &["doc", "--write", "--prelude"])
        .assert_silent_success();
    assert!(sandbox.directory.join("doc/prelude/option.md").is_file());
}

#[test]
fn doc_write_types_a_module_from_the_whole_program_when_its_own_check_fails() {
    let sandbox = Sandbox::new("doc_write_whole_program_type")
        .with_file(&Language::project_file_name(), DOCUMENTED_PROJECT)
        .with_file("src/lib.knv", "(export nothing)\n\n(def nothing none)\n")
        .with_file("src/other.knv", "(export nobody)\n\n(def nobody none)\n")
        .with_file(
            "src/main.knv",
            "(use lib nothing)\n(use other nobody)\n\n(defn main ()\n  (println (unwrap-or nothing 3))\n  (println (unwrap-or nobody \"x\")))\n",
        );
    sandbox
        .compiler(&["doc", "--write"])
        .assert_silent_success();
    assert!(
        sandbox
            .contents("doc/lib.md")
            .contains("\n(def nothing)\n(type nothing (option i64))\n```")
    );
    assert!(
        sandbox
            .contents("doc/other.md")
            .contains("\n(def nobody)\n(type nobody (option (ref str)))\n```")
    );
}

#[test]
fn doc_write_refuses_a_fallback_type_that_only_a_module_outside_the_entry_program_fixes() {
    let sandbox = Sandbox::new("doc_write_fallback_outside_entry")
        .with_file(&Language::project_file_name(), DOCUMENTED_PROJECT)
        .with_file("src/lib.knv", "(export nothing)\n(def nothing none)\n")
        .with_file(
            "src/main.knv",
            "(use lib nothing)\n(defn main () (println \"hi\"))\n",
        )
        .with_file(
            "src/extra.knv",
            "(use lib nothing)\n(export three)\n(defn three () (unwrap-or nothing 3))\n",
        );
    let write = sandbox.compiler(&["doc", "--write"]);
    let error = write.assert_exit_code(1).stderr();
    assert!(error.contains("error[L0407]"), "{error}");
    assert!(error.contains("src/lib.knv:2:14"), "{error}");
    assert!(!sandbox.directory.join("doc").exists());
}

#[test]
fn doc_write_refuses_a_module_the_entry_program_does_not_load() {
    let sandbox = Sandbox::new("doc_write_unloaded_fallback")
        .with_file(&Language::project_file_name(), DOCUMENTED_PROJECT)
        .with_file("src/unused.knv", "(export nothing)\n(def nothing none)\n")
        .with_file("src/main.knv", "(defn main () (println \"hi\"))\n");
    let write = sandbox.compiler(&["doc", "--write"]);
    let error = write.assert_exit_code(1).stderr();
    assert!(error.contains("error[L0407]"), "{error}");
    assert!(error.contains("src/unused.knv:2:14"), "{error}");
    assert!(!sandbox.directory.join("doc").exists());
}

#[test]
fn doc_write_falls_back_for_a_module_whose_exports_have_no_type() {
    let sandbox = Sandbox::new("doc_write_untyped_fallback")
        .with_file(&Language::project_file_name(), DOCUMENTED_PROJECT)
        .with_file(
            "src/lib.knv",
            "(export get-nothing)\n(def nothing none)\n(defmacro get-nothing () (template nothing))\n",
        )
        .with_file(
            "src/main.knv",
            "(use lib get-nothing)\n(defn main () (println (unwrap-or (get-nothing) 3)))\n",
        );
    sandbox
        .compiler(&["doc", "--write"])
        .assert_silent_success();
    assert!(
        sandbox
            .contents("doc/lib.md")
            .contains("(defmacro get-nothing ())")
    );
}

#[test]
fn doc_write_reports_the_error_that_fails_the_entry_program() {
    let sandbox = Sandbox::new("doc_write_failing_entry_program")
        .with_file(&Language::project_file_name(), DOCUMENTED_PROJECT)
        .with_file("src/lib.knv", "(export nothing)\n(def nothing none)\n")
        .with_file(
            "src/main.knv",
            "(use lib nothing)\n\n(defn main ()\n  (println (+ 1 \"x\"))\n  (println (unwrap-or nothing 3)))\n",
        );
    let write = sandbox.compiler(&["doc", "--write"]);
    let error = write.assert_exit_code(1).stderr();
    assert!(error.contains("error[L0401]"), "{error}");
    assert!(error.contains("src/main.knv:4:17"), "{error}");
    assert!(!error.contains("L0407"), "{error}");
}

#[test]
fn doc_write_outside_a_project_is_a_usage_error() {
    let sandbox = Sandbox::new("doc_write_no_project");
    sandbox.compiler(&["doc", "--write"]).assert_exit_code(2);
}

#[test]
fn doc_write_takes_no_term() {
    let sandbox = Sandbox::new("doc_write_term");
    let write = sandbox.compiler(&["doc", "--write", "defn"]);
    assert_eq!(
        write.assert_exit_code(2).stderr().lines().next(),
        Some("error: the argument '--write' cannot be used with '[TERM]'")
    );
}

#[test]
fn doc_takes_the_thread_last_term() {
    let sandbox = Sandbox::new("doc_thread_last");
    let doc = sandbox.compiler(&["doc", "->>"]);
    let page = doc.assert_exit_code(0).stdout();
    assert_eq!(page.lines().next(), Some(THREAD_LAST_HEADER));
}

#[test]
fn doc_takes_the_minus_term() {
    let sandbox = Sandbox::new("doc_minus");
    let doc = sandbox.compiler(&["doc", "-"]);
    let page = doc.assert_exit_code(0).stdout();
    assert_eq!(page.lines().next(), Some(MINUS_HEADER));
}

#[test]
fn doc_takes_a_program_term_that_starts_with_two_hyphens_after_the_separator() {
    let sandbox = Sandbox::new("doc_separated_program_term").with_file("main.knv", FLAG_NAMED_DEF);
    let doc = sandbox.compiler(&["doc", "--file", "main.knv", "--", "--verbose"]);
    assert_eq!(doc.assert_exit_code(0).stdout(), FLAG_NAMED_DEF_PAGE);
}

#[test]
fn doc_takes_a_prelude_term_after_the_separator() {
    let sandbox = Sandbox::new("doc_separated_prelude_term");
    let doc = sandbox.compiler(&["doc", "--", "defn"]);
    let page = doc.assert_exit_code(0).stdout();
    assert_eq!(page.lines().next(), Some(DEFN_HEADER));
}

#[test]
fn doc_reports_a_term_before_and_after_the_separator() {
    let sandbox = Sandbox::new("doc_term_on_both_sides");
    sandbox
        .compiler(&["doc", "defn", "--", "if"])
        .assert_exit_code(2);
}

#[test]
fn doc_reports_an_unknown_flag_before_the_term() {
    let sandbox = Sandbox::new("doc_unknown_flag_before_term");
    let doc = sandbox.compiler(&["doc", "--typo", "twice"]);
    let stderr = doc.assert_exit_code(2).stderr();
    assert_eq!(stderr.lines().next(), Some(UNKNOWN_FLAG_ERROR));
    assert!(stderr.contains(SEPARATOR_TIP), "{stderr}");
}

#[test]
fn doc_reports_an_unknown_flag_in_place_of_the_term() {
    let sandbox = Sandbox::new("doc_unknown_flag_as_term");
    let doc = sandbox.compiler(&["doc", "--typo"]);
    let stderr = doc.assert_exit_code(2).stderr();
    assert_eq!(stderr.lines().next(), Some(UNKNOWN_FLAG_ERROR));
}

#[test]
fn doc_reports_a_second_term() {
    let sandbox = Sandbox::new("doc_second_term");
    let doc = sandbox.compiler(&["doc", "defn", "defmacro"]);
    let stderr = doc.assert_exit_code(2).stderr();
    assert_eq!(
        stderr.lines().next(),
        Some("error: unexpected argument 'defmacro' found")
    );
}

#[test]
fn doc_prints_the_page_of_an_imported_term() {
    let sandbox = Sandbox::new("doc_imported")
        .with_file("geometry.knv", GEOMETRY)
        .with_file("main.knv", GEOMETRY_ENTRY);
    let doc = sandbox.compiler(&["doc", "--file", "main.knv", "area"]);
    assert_eq!(
        doc.assert_exit_code(0).stdout(),
        "(defn area (width height))\n\nMultiplies a width by a height.\n"
    );
}

#[test]
fn doc_prints_the_page_of_an_imported_module() {
    let sandbox = Sandbox::new("doc_module")
        .with_file("geometry.knv", DOCUMENTED_GEOMETRY)
        .with_file("main.knv", GEOMETRY_ENTRY);
    let doc = sandbox.compiler(&["doc", "--file", "main.knv", "(module geometry)"]);
    assert_eq!(doc.assert_exit_code(0).stdout(), GEOMETRY_PAGE);
}

#[test]
fn doc_prints_the_page_of_a_prelude_module() {
    let sandbox = Sandbox::new("doc_prelude_module");
    let doc = sandbox.compiler(&["doc", "(module prelude/vec)"]);
    let page = doc.assert_exit_code(0).stdout();
    let lines: Vec<&str> = page.lines().take(6).collect();
    assert_eq!(
        lines,
        [
            "(module prelude/vec)",
            "",
            "Builds, reads and reorders vectors.",
            "",
            "(at items index)",
            "    Returns a borrow of the item at an index."
        ]
    );
}

#[test]
fn doc_prints_a_module_page_without_a_type_line() {
    let sandbox = Sandbox::new("doc_module_types")
        .with_file("geometry.knv", DOCUMENTED_GEOMETRY)
        .with_file("main.knv", GEOMETRY_ENTRY);
    let doc = sandbox.compiler(&["doc", "--types", "--file", "main.knv", "(module geometry)"]);
    assert_eq!(doc.assert_exit_code(0).stdout(), GEOMETRY_PAGE);
}

#[test]
fn doc_lists_the_exports_of_the_entry_module() {
    let sandbox = Sandbox::new("doc_entry_module").with_file("main.knv", DOCUMENTED_ENTRY);
    let doc = sandbox.compiler(&["doc", "--file", "main.knv", "(module main)"]);
    assert_eq!(
        doc.assert_exit_code(0).stdout(),
        "(module main)\n\nPrints an area.\n\n(defn main ())\n"
    );
}

#[test]
fn doc_keeps_the_page_of_a_binding_named_like_a_module() {
    let sandbox = Sandbox::new("doc_module_binding")
        .with_file("geometry.knv", DOCUMENTED_GEOMETRY)
        .with_file("main.knv", SHADOWING_GEOMETRY_ENTRY);
    let doc = sandbox.compiler(&["doc", "--file", "main.knv", "geometry"]);
    assert_eq!(
        doc.assert_exit_code(0).stdout(),
        "(def geometry)\n\nNo documentation.\n"
    );
}

#[test]
fn doc_reports_an_unknown_module_at_its_path_in_the_term() {
    let sandbox = Sandbox::new("doc_unknown_module");
    let doc = sandbox.compiler(&["doc", "(module nowhere)"]);
    assert!(
        doc.assert_exit_code(1)
            .stderr()
            .starts_with("error[L0204]: cannot find module `nowhere`\n --> <term>:1:9\n")
    );
}

#[test]
fn doc_finds_a_module_when_the_program_binds_the_name_module() {
    let sandbox = Sandbox::new("doc_module_bound_head")
        .with_file("geometry.knv", DOCUMENTED_GEOMETRY)
        .with_file("main.knv", MODULE_BINDING_ENTRY);
    let doc = sandbox.compiler(&["doc", "--file", "main.knv", "(module geometry)"]);
    assert_eq!(doc.assert_exit_code(0).stdout(), GEOMETRY_PAGE);
}

#[test]
fn doc_reports_a_term_that_is_no_name_or_module_as_a_program_error() {
    for term in ["(f x)", "5"] {
        let sandbox = Sandbox::new("doc_malformed_term");
        let doc = sandbox.compiler(&["doc", term]);
        assert!(
            doc.assert_exit_code(1).stderr().starts_with(
                "error[L0301]: a documentation lookup takes a name, a module or a listing\n"
            ),
            "{term}"
        );
    }
}

#[test]
fn doc_prints_the_page_of_an_undocumented_term() {
    let sandbox = Sandbox::new("doc_undocumented").with_file("main.knv", UNDOCUMENTED_ENTRY);
    let doc = sandbox.compiler(&["doc", "--file", "main.knv", "twice"]);
    assert_eq!(
        doc.assert_exit_code(0).stdout(),
        "(defn twice (x))\n\nNo documentation.\n"
    );
}

#[test]
fn doc_suggests_a_close_name_for_an_unknown_term() {
    let sandbox = Sandbox::new("doc_unknown");
    let doc = sandbox.compiler(&["doc", "defm"]);
    insta::assert_snapshot!(doc.assert_exit_code(1).stderr());
}

#[test]
fn doc_rejects_a_term_that_is_not_one_term() {
    let sandbox = Sandbox::new("doc_not_one_name");
    let doc = sandbox.compiler(&["doc", "x y"]);
    assert_eq!(
        doc.assert_exit_code(2).stderr(),
        "error: `x y` is not one term\n"
    );
}

#[test]
fn doc_rejects_a_term_that_does_not_read() {
    let sandbox = Sandbox::new("doc_unreadable_term");
    let doc = sandbox.compiler(&["doc", "("]);
    assert!(doc.assert_exit_code(2).stderr().starts_with("error[L0101]"));
}

#[test]
fn doc_reports_an_unreadable_file_as_a_usage_error() {
    let sandbox = Sandbox::new("doc_unreadable_file");
    let doc = sandbox.compiler(&["doc", "--file", "missing.knv", "defn"]);
    assert!(
        doc.assert_exit_code(2)
            .stderr()
            .starts_with("error: cannot read `missing.knv`")
    );
}

#[test]
fn doc_rejects_a_term_that_is_not_one_term_before_reading_the_file() {
    let sandbox = Sandbox::new("doc_term_before_file");
    let doc = sandbox.compiler(&["doc", "--file", "missing.knv", "x y"]);
    assert_eq!(
        doc.assert_exit_code(2).stderr(),
        "error: `x y` is not one term\n"
    );
}

#[test]
fn doc_rejects_a_shadowing_page_body_that_returns_no_string() {
    let sandbox = Sandbox::new("doc_shadowed").with_file("main.knv", SHADOWING_PAGE_BODY);
    let doc = sandbox.compiler(&["doc", "--file", "main.knv", "main"]);
    assert_eq!(
        doc.assert_exit_code(1).stderr(),
        "error: `doc-body` did not expand to a string\n"
    );
}

#[test]
fn doc_rejects_a_shadowing_header_that_returns_no_string() {
    let sandbox = Sandbox::new("doc_shadowed_header").with_file("main.knv", SHADOWING_HEADER);
    let doc = sandbox.compiler(&["doc", "--file", "main.knv", "main"]);
    assert_eq!(
        doc.assert_exit_code(1).stderr(),
        "error: `doc-header` did not expand to a string\n"
    );
}

#[test]
fn doc_types_prints_the_inferred_type_above_the_page() {
    let sandbox = Sandbox::new("doc_types").with_file("prog.knv", TYPED_PROGRAM);
    assert_eq!(
        sandbox.typed_page(&["--file", "prog.knv", "twice"]),
        format!("{TWICE_TYPE}\n{TWICE_PAGE}")
    );
}

#[test]
fn doc_types_takes_the_flag_after_the_term() {
    let sandbox = Sandbox::new("doc_types_after_term").with_file("prog.knv", TYPED_PROGRAM);
    let doc = sandbox.compiler(&["doc", "twice", "--file", "prog.knv", "--types"]);
    assert_eq!(
        doc.assert_exit_code(0).stdout(),
        format!("{TWICE_TYPE}\n{TWICE_PAGE}")
    );
}

#[test]
fn doc_types_prints_the_generic_type_of_an_uncalled_function() {
    let sandbox = Sandbox::new("doc_types_generic").with_file("prog.knv", TYPED_PROGRAM);
    assert_eq!(
        sandbox.typed_page(&["--file", "prog.knv", "same"]),
        "(type same (where (a) (fn (a) a)))\n(defn same (x))\n\nNo documentation.\n"
    );
}

#[test]
fn doc_types_prints_no_type_for_a_macro() {
    let sandbox = Sandbox::new("doc_types_macro");
    assert_eq!(
        sandbox.typed_page(&["when"]),
        "(when test body ...)\n\nRuns the body when the test is true.\n\nIt returns unit when the test is false. The body must return unit, and so\ndoes the call.\n\n(local total 0)\n(when (> 2 1) (set! total 5))\n(= total 5)\n=> true\n\n(when (> 1 2) (set! total 9))\n(= total 5)\n=> true\n"
    );
}

#[test]
fn doc_types_prints_no_type_for_a_kernel_form() {
    let sandbox = Sandbox::new("doc_types_kernel_form");
    assert_eq!(sandbox.typed_page(&["if"]), IF_PAGE);
}

#[test]
fn doc_types_prints_the_type_of_a_prelude_function() {
    let sandbox = Sandbox::new("doc_types_prelude");
    assert_eq!(
        sandbox.typed_page(&["sum"]),
        "(type sum (where ((n number)) (fn ((iterator n)) n)))\n(defn sum (values))\n\nAdds the items of an iterator from left to right.\n\nAn empty iterator gives 0. An overflow panics in a debug build, as `+`\ndoes. A vector passes and yields copies of its numbers. `(iter items)` is an\nerror, as its items are borrows, and so is a map.\n\n(sum (vec 1 2 3))\n=> 6\n\n(sum (range 0 0))\n=> 0\n"
    );
}

#[test]
fn check_reports_an_unused_broken_definition_in_a_user_prelude_directory() {
    let sandbox = Sandbox::new("user_prelude_directory")
        .with_file("prog.knv", USES_PRELUDE_DIRECTORY)
        .with_file("prelude/extra.knv", BROKEN_PRELUDE_DIRECTORY_MODULE);
    let check = sandbox.compiler(&["check", "prog.knv"]);
    check.assert_exit_code(1);
    assert!(check.stderr().contains("L0401"), "{}", check.stderr());
}

#[test]
fn run_finds_main_in_an_entry_named_prelude() {
    let sandbox = Sandbox::new("entry_named_prelude").with_file("prelude.knv", MAIN_IN_PRELUDE);
    let run = sandbox.compiler(&["run", "prelude.knv"]);
    run.assert_exit_code(0);
    assert_eq!(run.stdout(), "hi\n");
}

#[test]
fn doc_types_prints_no_type_for_a_struct() {
    let sandbox = Sandbox::new("doc_types_struct").with_file("prog.knv", TYPED_PROGRAM);
    assert_eq!(
        sandbox.typed_page(&["--file", "prog.knv", "point"]),
        "(defstruct point)\n\nNo documentation.\n"
    );
}

#[test]
fn doc_types_prints_the_type_of_a_struct_constructor() {
    let sandbox = Sandbox::new("doc_types_constructor").with_file("prog.knv", TYPED_PROGRAM);
    assert_eq!(
        sandbox.typed_page(&["--file", "prog.knv", "->point"]),
        "(type ->point (fn (i64 i64) point))\n(defstruct point)\n\nNo documentation.\n"
    );
}

#[test]
fn doc_types_prints_the_type_of_a_function_with_keyword_parameters() {
    let sandbox = Sandbox::new("doc_types_keyword").with_file("prog.knv", TYPED_PROGRAM);
    assert_eq!(
        sandbox.typed_page(&["--file", "prog.knv", "area"]),
        "(type area (fn (i64 i64) i64))\n(defn area (:width i64 :height i64))\n\nNo documentation.\n"
    );
}

#[test]
fn doc_types_prints_the_type_of_a_plain_definition() {
    let sandbox = Sandbox::new("doc_types_plain").with_file("prog.knv", TYPED_PROGRAM);
    assert_eq!(
        sandbox.typed_page(&["--file", "prog.knv", "limit"]),
        "(type limit i64)\n(def limit)\n\nNo documentation.\n"
    );
}

#[test]
fn doc_types_prints_no_type_for_a_value_that_no_definition_binds() {
    let sandbox = Sandbox::new("doc_types_literal");
    assert_eq!(sandbox.typed_page(&["true"]), TRUE_PAGE);
}

#[test]
fn doc_types_prints_no_type_above_a_struct_page_whose_header_matches_the_value() {
    let sandbox =
        Sandbox::new("doc_types_same_header").with_file("prog.knv", MACRO_STRUCT_BESIDE_A_DEF);
    assert_eq!(
        sandbox.typed_page(&["--file", "prog.knv", "point"]),
        "point\n\nA documented struct.\n"
    );
}

#[test]
fn doc_types_prints_no_type_above_a_documented_struct_beside_a_function() {
    let sandbox = Sandbox::new("doc_types_documented_struct")
        .with_file("prog.knv", DOCUMENTED_STRUCT_BESIDE_A_FUNCTION);
    assert_eq!(
        sandbox.typed_page(&["--file", "prog.knv", "point"]),
        "(defstruct point)\n\nA point on a grid.\n"
    );
}

#[test]
fn doc_types_prints_the_type_of_a_documented_function_beside_a_struct() {
    let sandbox = Sandbox::new("doc_types_documented_function")
        .with_file("prog.knv", DOCUMENTED_FUNCTION_BESIDE_A_STRUCT);
    assert_eq!(
        sandbox.typed_page(&["--file", "prog.knv", "point"]),
        format!("{POINT_TYPE}\n(defn point ())\n\nMakes the origin.\n")
    );
}

#[test]
fn doc_types_prints_the_type_of_an_undocumented_function_beside_a_struct() {
    let sandbox = Sandbox::new("doc_types_undocumented_function")
        .with_file("prog.knv", UNDOCUMENTED_FUNCTION_BESIDE_A_STRUCT);
    assert_eq!(
        sandbox.typed_page(&["--file", "prog.knv", "point"]),
        format!("{POINT_TYPE}\n(defn point ())\n\nNo documentation.\n")
    );
}

#[test]
fn doc_types_prints_the_type_of_an_undocumented_def_beside_a_struct() {
    let sandbox = Sandbox::new("doc_types_undocumented_def")
        .with_file("prog.knv", UNDOCUMENTED_DEF_BESIDE_A_STRUCT);
    assert_eq!(
        sandbox.typed_page(&["--file", "prog.knv", "point"]),
        "(type point i64)\n(def point)\n\nNo documentation.\n"
    );
}

#[test]
fn doc_types_prints_the_page_alone_when_another_definition_fails_to_type_check() {
    let sandbox =
        Sandbox::new("doc_types_other_fails").with_file("prog.knv", OTHER_DEFINITION_FAILS);
    let doc = sandbox.typed_lookup(&["--file", "prog.knv", "twice"]);
    assert!(doc.assert_exit_code(1).stderr().starts_with("error[L0401]"));
    assert_eq!(doc.stdout(), TWICE_PAGE);
}

#[test]
fn doc_types_prints_the_page_alone_when_its_own_definition_fails_to_type_check() {
    let sandbox = Sandbox::new("doc_types_own_fails").with_file("prog.knv", OWN_DEFINITION_FAILS);
    let doc = sandbox.typed_lookup(&["--file", "prog.knv", "twice"]);
    assert!(doc.assert_exit_code(1).stderr().starts_with("error[L0404]"));
    assert_eq!(doc.stdout(), TWICE_PAGE);
}

#[test]
fn doc_types_prints_the_type_of_an_imported_term() {
    let sandbox = Sandbox::new("doc_types_imported")
        .with_file("lib/numbers.knv", NUMBERS)
        .with_file("main.knv", NUMBERS_ENTRY);
    assert_eq!(
        sandbox.typed_page(&["--file", "main.knv", "twice"]),
        format!("{TWICE_TYPE}\n{TWICE_PAGE}")
    );
}

#[test]
fn doc_types_prints_the_type_of_the_definition_that_shadows_a_prelude_name() {
    let sandbox = Sandbox::new("doc_types_shadowing").with_file("main.knv", SHADOWING_SUM);
    assert_eq!(
        sandbox.typed_page(&["--file", "main.knv", "sum"]),
        "(type sum (fn (i64 i64) i64))\n(defn sum (left right))\n\nNo documentation.\n"
    );
}

#[test]
fn doc_types_leaves_a_page_in_the_program_untyped() {
    let sandbox = Sandbox::new("doc_types_in_program").with_file("prog.knv", PAGE_PROGRAM);
    let expanded = sandbox.compiler(&["expand", "prog.knv"]);
    assert!(
        expanded
            .assert_exit_code(0)
            .stdout()
            .contains(r#"(def page "(defn twice (x))\n\nDoubles a number.")"#)
    );
    assert_eq!(
        sandbox.typed_page(&["--file", "prog.knv", "page"]),
        "(type page (ref str))\n(def page)\n\nNo documentation.\n"
    );
}

#[test]
fn emit_qualifies_a_struct_named_in_a_bound() {
    let sandbox = Sandbox::new("bound_import")
        .with_file("bound.knv", BOUND_ENTRY)
        .with_file("other/person.knv", BOUND_MODULE);
    let emitted = sandbox.compiler(&["emit", "bound.knv"]);
    let stdout = emitted.assert_exit_code(0).stdout();
    assert!(stdout.contains("std::ops::Add<crate::other::person::Person>"));
    assert!(stdout.contains("struct Person"));
}

#[test]
fn run_keeps_a_trait_method_vector_parameter_as_written() {
    let sandbox = Sandbox::new("trait_vector").with_file("mileage.knv", TRAIT_SHARED_VECTOR);
    let run = sandbox.compiler(&["run", "mileage.knv"]);
    assert_eq!(run.assert_exit_code(0).stdout(), "1210\n");
}

#[test]
fn a_failed_main_stops_without_a_panic_when_standard_error_is_a_dead_pipe() {
    let sandbox = Sandbox::new("dead_pipe_main_error").with_file("failure.knv", FAILED_MAIN);
    sandbox
        .compiler_with_a_closed_standard_error(&["run", "failure.knv"])
        .assert_no_panic();
}

#[test]
fn eprintln_in_a_result_main_stops_without_a_panic_when_standard_error_is_a_dead_pipe() {
    let sandbox = Sandbox::new("dead_pipe_eprintln").with_file("report.knv", REPORTING_MAIN);
    sandbox
        .compiler_with_a_closed_standard_error(&["run", "report.knv"])
        .assert_no_panic();
}

#[test]
fn eprintln_alone_stops_without_a_panic_when_standard_error_is_a_dead_pipe() {
    let sandbox = Sandbox::new("dead_pipe_error_only").with_file("report.knv", ERROR_ONLY_MAIN);
    sandbox
        .compiler_with_a_closed_standard_error(&["run", "report.knv"])
        .assert_no_panic();
}

#[test]
fn run_panics_with_the_printed_form_of_an_unwrapped_error() {
    let sandbox = Sandbox::new("unwrapped_failure").with_file("failure.knv", UNWRAPPED_FAILURE);
    let run = sandbox.compiler(&["run", "failure.knv"]);
    assert!(
        run.assert_exit_code(101)
            .stderr()
            .contains("called unwrap-ok on (err (->failure :line 3))")
    );
}

#[test]
fn check_reports_errors_without_terminal_styling() {
    let sandbox = Sandbox::new("check_errors").with_file("unbound.knv", UNBOUND_NAME);
    let check = sandbox.compiler(&["check", "unbound.knv"]);
    insta::assert_snapshot!(check.assert_exit_code(1).stderr());
}

#[test]
fn check_caps_the_printed_diagnostics_and_counts_the_rest() {
    let sandbox =
        Sandbox::new("check_capped").with_unbound_names("flood.knv", UNBOUND_NAMES_PAST_THE_CAP);
    let check = sandbox.compiler(&["check", "flood.knv"]);
    insta::assert_snapshot!(check.assert_exit_code(1).stderr());
}

#[test]
fn expand_prints_kernel_forms() {
    let sandbox = Sandbox::new("expand").with_file("greeting.knv", GREETING);
    let expand = sandbox.compiler(&["expand", "greeting.knv"]);
    assert_eq!(
        expand.assert_exit_code(0).stdout(),
        "(def main (fn () ((rust \"println!(\\\"hello from the sandbox\\\")\" (where () (fn () unit))))))\n"
    );
}

#[test]
fn emit_prints_the_generated_rust() {
    let sandbox = Sandbox::new("emit").with_file("greeting.knv", GREETING);
    let emit = sandbox.compiler(&["emit", "greeting.knv"]);
    assert!(emit.assert_exit_code(0).stdout().contains("fn main() {"));
}

#[test]
fn build_writes_the_program_under_the_build_directory() {
    let sandbox = Sandbox::new("build_default").with_file("greeting.knv", GREETING);
    sandbox
        .compiler(&["build", "greeting.knv"])
        .assert_exit_code(0);
    let greeting = sandbox.built_program(Sandbox::built_path("greeting/greeting"));
    assert_eq!(
        greeting.assert_exit_code(0).stdout(),
        "hello from the sandbox\n"
    );
}

#[test]
fn build_keeps_entries_with_the_same_stem_apart() {
    let sandbox = Sandbox::new("build_same_stem")
        .with_file("a/program.knv", PRINTS_A)
        .with_file("b/program.knv", PRINTS_B);
    sandbox
        .compiler(&["build", "a/program.knv"])
        .assert_exit_code(0);
    sandbox
        .compiler(&["build", "b/program.knv"])
        .assert_exit_code(0);
    let first = sandbox.built_program(Sandbox::built_path("a/program/program"));
    let second = sandbox.built_program(Sandbox::built_path("b/program/program"));
    assert_eq!(first.assert_exit_code(0).stdout(), "A\n");
    assert_eq!(second.assert_exit_code(0).stdout(), "B\n");
}

#[test]
fn build_mirrors_an_entry_outside_the_working_directory() {
    let sandbox = Sandbox::new("build_outside")
        .with_file("shared/program.knv", PRINTS_A)
        .with_file("work/empty", "");
    sandbox
        .compiler_in("work", &["build", "../shared/program.knv"])
        .assert_exit_code(0);
    let program =
        sandbox.built_program(sandbox.outside_build_path("work", "shared/program/program"));
    assert_eq!(program.assert_exit_code(0).stdout(), "A\n");
}

#[test]
fn built_program_stops_quietly_when_its_reader_closes_stdout() {
    let sandbox = Sandbox::new("closed_stdout").with_file("count.knv", MILLION_LINES);
    sandbox
        .compiler(&["build", "count.knv"])
        .assert_exit_code(0);
    let count = sandbox.built_program_read_for_one_line(Sandbox::built_path("count/count"));
    assert_eq!(count.assert_quiet_stop().stdout(), "0\n");
}

#[test]
fn built_program_shows_a_prompt_before_it_reads_a_line() {
    let sandbox = Sandbox::new("prompt_before_read").with_file("who.knv", PROMPT_THEN_READ_LINE);
    sandbox.compiler(&["build", "who.knv"]).assert_exit_code(0);
    let shown =
        sandbox.built_program_output_before_its_input(Sandbox::built_path("who/who"), "Ada\n");
    assert_eq!(shown, "who? ");
}

#[test]
fn built_program_shows_a_prompt_before_it_slurps_its_input() {
    let sandbox = Sandbox::new("prompt_before_slurp").with_file("who.knv", PROMPT_THEN_SLURP);
    sandbox.compiler(&["build", "who.knv"]).assert_exit_code(0);
    let shown =
        sandbox.built_program_output_before_its_input(Sandbox::built_path("who/who"), "Ada\n");
    assert_eq!(shown, "who? ");
}

#[test]
fn built_program_shows_a_prompt_before_it_slurps_any_source() {
    let sandbox =
        Sandbox::new("prompt_before_source").with_file("who.knv", PROMPT_THEN_SLURP_ANY_SOURCE);
    sandbox.compiler(&["build", "who.knv"]).assert_exit_code(0);
    let shown =
        sandbox.built_program_output_before_its_input(Sandbox::built_path("who/who"), "Ada\n");
    assert_eq!(shown, "who? ");
}

#[test]
fn built_program_ends_its_input_lines_after_a_failed_read() {
    let sandbox = Sandbox::new("failed_read").with_file("lines.knv", COUNT_INPUT_LINES);
    sandbox
        .compiler(&["build", "lines.knv"])
        .assert_exit_code(0);
    let lines = sandbox.built_program_reading_its_directory(Sandbox::built_path("lines/lines"));
    assert_eq!(lines.assert_exit_code(0).stdout(), "1\n");
}

#[test]
fn build_writes_the_program_to_the_output_path() {
    let sandbox = Sandbox::new("build_output").with_file("greeting.knv", GREETING);
    sandbox
        .compiler(&["build", "greeting.knv", "-o", "bin/hello"])
        .assert_exit_code(0);
    let greeting = sandbox.built_program("bin/hello");
    assert_eq!(
        greeting.assert_exit_code(0).stdout(),
        "hello from the sandbox\n"
    );
}

#[test]
fn emit_without_rustfmt_prints_one_note_and_writes_unformatted_rust() {
    let sandbox = Sandbox::new("emit_without_rustfmt").with_file("greeting.knv", GREETING);
    let emit = sandbox.compiler_with_path("no-tools", &["emit", "greeting.knv"]);
    let emit = emit.assert_exit_code(0);
    assert!(emit.stdout().contains("fn main() {"));
    assert_eq!(emit.stderr().matches("note:").count(), 1);
    assert!(emit.stderr().contains("rustfmt"));
    assert!(emit.stderr().contains("unformatted"));
}

#[test]
fn emit_with_a_failing_rustfmt_prints_one_note_and_writes_unformatted_rust() {
    let sandbox = Sandbox::new("emit_failing_rustfmt")
        .with_file("greeting.knv", GREETING)
        .with_executable("tools/rustfmt", FAILING_RUSTFMT);
    let emit = sandbox.compiler_with_path("tools", &["emit", "greeting.knv"]);
    let emit = emit.assert_exit_code(0);
    assert!(emit.stdout().contains("fn main() {"));
    assert_eq!(emit.stderr().matches("note:").count(), 1);
    assert!(
        emit.stderr().contains("rustfmt failed"),
        "stderr: {}",
        emit.stderr()
    );
}

struct NestedIfProgram;

impl NestedIfProgram {
    const DEPTH: usize = 25;

    fn source() -> String {
        let opening = "(if true ".repeat(Self::DEPTH);
        let closing = " 0)".repeat(Self::DEPTH);
        format!("(defn main ()\n  (println {opening}1{closing}))\n")
    }
}

#[test]
fn emit_stops_rustfmt_past_its_time_limit_and_writes_unformatted_rust() {
    let time_limit = Duration::from_millis(500);
    let sandbox =
        Sandbox::new("emit_rustfmt_time_limit").with_file("nested.knv", &NestedIfProgram::source());
    let started = Instant::now();
    let emit = sandbox.compiler_with_rustfmt_time_limit(time_limit, &["emit", "nested.knv"]);
    assert!(started.elapsed() < time_limit + Duration::from_secs(5));
    let emit = emit.assert_exit_code(0);
    assert_eq!(emit.stderr().matches("note:").count(), 1);
    assert!(
        emit.stderr()
            .contains("rustfmt did not finish within 500ms"),
        "stderr: {}",
        emit.stderr()
    );
    assert_eq!(
        emit.stdout().matches("if true").count(),
        NestedIfProgram::DEPTH
    );
}

#[test]
fn emit_rejects_a_malformed_rustfmt_time_limit_as_a_usage_error() {
    let sandbox =
        Sandbox::new("emit_malformed_rustfmt_time_limit").with_file("greeting.knv", GREETING);
    let emit = sandbox.compiler_with_rustfmt_time_limit_value("soon", &["emit", "greeting.knv"]);
    let emit = emit.assert_exit_code(2);
    assert!(
        emit.stderr()
            .contains(&EmittedFormatting::time_limit_variable()),
        "stderr: {}",
        emit.stderr()
    );
}

#[test]
fn build_reports_rustc_errors_as_program_errors() {
    let sandbox = Sandbox::new("build_rustc_error").with_file("sign.knv", RUSTC_SYNTAX_ERROR);
    let build = sandbox.compiler(&["build", "sign.knv"]);
    assert!(
        build
            .assert_exit_code(1)
            .stderr()
            .contains("note: reported by rustc")
    );
}

#[test]
fn run_prints_the_program_output() {
    let sandbox = Sandbox::new("run").with_file("greeting.knv", GREETING);
    let run = sandbox.compiler(&["run", "--release", "greeting.knv"]);
    assert_eq!(run.assert_exit_code(0).stdout(), "hello from the sandbox\n");
}

#[test]
fn run_passes_the_arguments_after_the_separator() {
    let sandbox = Sandbox::new("run_arguments").with_file("echo.knv", ECHO);
    let run = sandbox.compiler(&["run", "echo.knv", "--", "fake-argument"]);
    assert_eq!(run.assert_exit_code(0).stdout(), "fake-argument\n");
}

#[test]
fn run_exits_with_the_program_exit_code() {
    let sandbox = Sandbox::new("run_exit_code").with_file("exit.knv", EXIT_THREE);
    sandbox.compiler(&["run", "exit.knv"]).assert_exit_code(3);
}

#[test]
fn run_dies_by_sigpipe_when_its_reader_closes() {
    let sandbox = Sandbox::new("run_closed_pipe").with_file("lines.knv", THOUSAND_LINES);
    let run = sandbox.compiler_writing_to_a_closed_pipe(&["run", "lines.knv"]);
    assert_eq!(run.assert_killed_by(Outcome::SIGPIPE).stderr(), "");
}

#[test]
fn run_dies_by_the_signal_that_kills_the_program() {
    let sandbox = Sandbox::new("run_abort").with_file("abort.knv", ABORT);
    let run = sandbox.compiler(&["run", "abort.knv"]);
    assert_eq!(run.assert_killed_by(Outcome::SIGABRT).stderr(), "");
}

#[test]
fn run_dies_by_the_program_signal_that_it_inherits_blocked() {
    let sandbox = Sandbox::new("run_abort_blocked").with_file("abort.knv", ABORT);
    sandbox
        .compiler_blocking_sigabrt(&["run", "abort.knv"])
        .assert_killed_by(Outcome::SIGABRT);
}

#[test]
fn run_prints_a_main_error_with_display_and_exits_one() {
    let sandbox = Sandbox::new("run_main_error").with_file("main_error.knv", MAIN_ERROR);
    let run = sandbox.compiler(&["run", "main_error.knv"]);
    run.assert_exit_code(1);
    let stderr = run.stderr();
    assert!(
        stderr.contains("No such file or directory"),
        "stderr: {stderr}"
    );
    assert!(!stderr.contains("Os {"), "stderr: {stderr}");
}

#[test]
fn every_command_reports_program_errors() {
    let sandbox = Sandbox::new("program_errors").with_file("unbound.knv", UNBOUND_NAME);
    for command in ["expand", "emit", "build", "run"] {
        sandbox
            .compiler(&[command, "unbound.knv"])
            .assert_exit_code(1);
    }
}

#[test]
fn build_without_rustc_is_a_usage_error() {
    let sandbox = Sandbox::new("build_without_rustc")
        .with_file("greeting.knv", GREETING)
        .with_executable("tools/rustfmt", PASSTHROUGH_RUSTFMT);
    let build = sandbox.compiler_with_path("tools", &["build", "greeting.knv"]);
    assert!(
        build
            .assert_exit_code(2)
            .stderr()
            .starts_with("error: cannot run rustc")
    );
}

#[test]
fn build_cannot_write_under_a_file() {
    let sandbox = Sandbox::new("build_under_file")
        .with_file("greeting.knv", GREETING)
        .with_file("blocker", "");
    let build = sandbox.compiler(&["build", "greeting.knv", "-o", "blocker/program"]);
    assert!(
        build
            .assert_exit_code(2)
            .stderr()
            .starts_with("error: cannot create `blocker`")
    );
}

#[test]
fn build_reports_a_rustc_failure_without_diagnostics() {
    let sandbox = Sandbox::new("build_rustc_crash")
        .with_file("greeting.knv", GREETING)
        .with_executable("tools/rustc", CRASHING_RUSTC)
        .with_executable("tools/rustfmt", PASSTHROUGH_RUSTFMT);
    let build = sandbox.compiler_with_path("tools", &["build", "greeting.knv"]);
    insta::assert_snapshot!(build.assert_exit_code(1).stderr());
}

#[test]
fn run_reports_a_program_that_cannot_start() {
    let sandbox = Sandbox::new("run_missing_program")
        .with_file("greeting.knv", GREETING)
        .with_executable("tools/rustc", SILENT_RUSTC)
        .with_executable("tools/rustfmt", PASSTHROUGH_RUSTFMT);
    let run = sandbox.compiler_with_path("tools", &["run", "greeting.knv"]);
    assert!(run.assert_exit_code(2).stderr().starts_with(&format!(
        "error: cannot run `{}`",
        Sandbox::built_path("greeting/greeting").display()
    )));
}

#[test]
fn eval_prints_a_value_through_print_form() {
    let sandbox = Sandbox::new("eval_value").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&["eval", "session.knv", "(->point (square 2) 3)"]);
    assert_eq!(
        eval.assert_silent_success().stdout(),
        "(->point :x 4 :y 3)\n"
    );
}

#[test]
fn eval_prints_nothing_for_a_unit_result() {
    let sandbox = Sandbox::new("eval_unit").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&["eval", "session.knv", "(println \"side \" (square 3))"]);
    assert_eq!(eval.assert_silent_success().stdout(), "side 9\n");
}

#[test]
fn eval_runs_every_form_once_in_order_as_one_body() {
    let sandbox = Sandbox::new("eval_body").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&[
        "eval",
        "session.knv",
        "(local x 5) (println \"x\") (* x 2) (main)",
    ]);
    assert_eq!(
        eval.assert_silent_success().stdout(),
        "x\n10\nwritten main\n"
    );
}

#[test]
fn eval_notes_a_value_whose_type_does_not_implement_show() {
    let sandbox = Sandbox::new("eval_no_show").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&["eval", "session.knv", "(square 2) (fn ((x i64)) x)"]);
    insta::assert_snapshot!(eval.assert_exit_code(0).stderr());
    assert_eq!(eval.stdout(), "4\n");
}

#[test]
fn eval_reports_an_open_type_on_the_form() {
    let sandbox = Sandbox::new("eval_open_type").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&["eval", "session.knv", "(println \"never\") none"]);
    insta::assert_snapshot!(eval.assert_exit_code(1).stderr());
    assert_eq!(eval.stdout(), "");
}

#[test]
fn eval_wraps_a_lone_open_form() {
    let sandbox = Sandbox::new("eval_lone_open_form").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&["eval", "session.knv", "(vec)"]);
    insta::assert_snapshot!(eval.assert_exit_code(1).stderr());
    assert_eq!(eval.stdout(), "");
}

#[test]
fn eval_wraps_the_innermost_open_value_of_a_form() {
    let sandbox = Sandbox::new("eval_inner_open_type").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&["eval", "session.knv", "(some none)"]);
    insta::assert_snapshot!(eval.assert_exit_code(1).stderr());
    assert_eq!(eval.stdout(), "");
}

#[test]
fn eval_reports_a_diagnostic_of_the_module_and_runs_nothing() {
    let sandbox = Sandbox::new("eval_module_error").with_file("geometry.knv", AREA_OF_A_STRING);
    let eval = sandbox.compiler(&["eval", "geometry.knv", "(println \"never\")"]);
    insta::assert_snapshot!(eval.assert_exit_code(1).stderr());
    assert_eq!(eval.stdout(), "");
}

#[test]
fn eval_reports_a_diagnostic_of_a_form_at_its_own_line() {
    let sandbox =
        Sandbox::new("eval_form_error").with_file("geometry.knv", TOP_LEVEL_AREA_OF_A_STRING);
    let eval = sandbox.compiler(&[
        "eval",
        "--line",
        "2",
        "--column",
        "1",
        "geometry.knv",
        "(area 2 \"3\")",
    ]);
    insta::assert_snapshot!(eval.assert_exit_code(1).stderr());
}

#[test]
fn eval_reports_a_diagnostic_inside_a_definition_once() {
    let sandbox =
        Sandbox::new("eval_form_in_definition_error").with_file("geometry.knv", AREA_OF_A_STRING);
    let eval = sandbox.compiler(&[
        "eval",
        "--line",
        "2",
        "--column",
        "17",
        "geometry.knv",
        "(area 2 \"3\")",
    ]);
    insta::assert_snapshot!(eval.assert_exit_code(1).stderr());
}

#[test]
fn eval_runs_a_top_level_form_that_the_module_check_leaves_out() {
    let sandbox = Sandbox::new("eval_top_level").with_file("geometry.knv", EVALUATED_GEOMETRY);
    let eval = sandbox.compiler(&[
        "eval",
        "--line",
        "2",
        "--column",
        "1",
        "geometry.knv",
        "(area 2 3)",
    ]);
    assert_eq!(eval.assert_silent_success().stdout(), "6\n");
}

#[test]
fn eval_notes_a_definition_form_and_runs_nothing() {
    let sandbox = Sandbox::new("eval_definition").with_file("geometry.knv", GEOMETRY_DEFINITIONS);
    let eval = sandbox.compiler(&[
        "eval",
        "--line",
        "1",
        "--column",
        "1",
        "geometry.knv",
        "(defn area (w h) (* w h))",
    ]);
    insta::assert_snapshot!(eval.assert_exit_code(0).stderr());
    assert_eq!(eval.stdout(), "");
}

#[test]
fn eval_notes_a_definition_inside_a_top_level_do() {
    let sandbox =
        Sandbox::new("eval_definition_in_do").with_file("geometry.knv", GEOMETRY_DEFINITIONS);
    let eval = sandbox.compiler(&[
        "eval",
        "--line",
        "4",
        "--column",
        "5",
        "geometry.knv",
        "(defn twice (n) (* 2 n))",
    ]);
    insta::assert_snapshot!(eval.assert_exit_code(0).stderr());
}

#[test]
fn eval_runs_a_form_inside_a_definition() {
    let sandbox = Sandbox::new("eval_inside").with_file("geometry.knv", GEOMETRY_DEFINITIONS);
    let eval = sandbox.compiler(&[
        "eval",
        "--line",
        "3",
        "--column",
        "12",
        "geometry.knv",
        "(area 4 5)",
    ]);
    assert_eq!(eval.assert_silent_success().stdout(), "20\n");
}

#[test]
fn eval_rejects_forms_that_are_not_the_text_at_their_position() {
    let sandbox = Sandbox::new("eval_misplaced").with_file("geometry.knv", EVALUATED_GEOMETRY);
    let eval = sandbox.compiler(&[
        "eval",
        "--line",
        "2",
        "--column",
        "2",
        "geometry.knv",
        "(area 2 3)",
    ]);
    assert_eq!(
        eval.assert_exit_code(2).stderr(),
        "error: the forms are not the text of `geometry.knv` at 2:2\n"
    );
}

#[test]
fn eval_rejects_a_line_without_a_column() {
    let sandbox = Sandbox::new("eval_line_only").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&["eval", "--line", "2", "session.knv", "1"]);
    assert!(
        eval.assert_exit_code(2)
            .stderr()
            .contains("--column <COLUMN>")
    );
}

#[test]
fn eval_reads_the_module_from_standard_input() {
    let sandbox = Sandbox::new("eval_stdin");
    let eval = sandbox.compiler_reading(SESSION, &["eval", "--stdin", "unsaved.knv", "(square 7)"]);
    assert_eq!(eval.assert_silent_success().stdout(), "49\n");
}

#[test]
fn eval_resolves_use_from_the_root() {
    let sandbox = Sandbox::new("eval_root")
        .with_file("shapes.knv", SHAPES)
        .with_file("src/user.knv", SHAPES_USER);
    let eval = sandbox.compiler(&["eval", "--root", ".", "src/user.knv", "(circle 2)"]);
    assert_eq!(eval.assert_silent_success().stdout(), "12\n");
}

#[test]
fn eval_exits_with_the_program_exit_code() {
    let sandbox = Sandbox::new("eval_exit").with_file("exits.knv", EXITS);
    sandbox
        .compiler(&["eval", "exits.knv", "(exit 3)"])
        .assert_exit_code(3);
}

#[test]
fn eval_exits_with_the_code_of_a_panic() {
    let sandbox = Sandbox::new("eval_panic").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&["eval", "session.knv", "(at (vec 1) 5)"]);
    assert!(
        eval.assert_exit_code(101)
            .stderr()
            .contains("index out of bounds")
    );
}

#[test]
fn eval_dies_by_the_signal_that_kills_the_program() {
    let sandbox = Sandbox::new("eval_signal").with_file("exits.knv", EXITS);
    sandbox
        .compiler(&["eval", "exits.knv", "(abort)"])
        .assert_killed_by(Outcome::SIGABRT);
}

#[test]
fn eval_dies_by_sigpipe_when_its_output_closes() {
    let sandbox = Sandbox::new("eval_closed_output").with_file("session.knv", SESSION);
    let forms = "(for-each (range 0 1000000) (fn (i) (println i)))";
    sandbox
        .compiler_writing_to_a_closed_pipe(&["eval", "session.knv", forms])
        .assert_killed_by(Outcome::SIGPIPE);
}

#[test]
fn eval_leaves_no_build_directory_beside_the_module() {
    let sandbox = Sandbox::new("eval_no_target").with_file("session.knv", SESSION);
    sandbox
        .compiler(&["eval", "session.knv", "(square 2)"])
        .assert_exit_code(0);
    assert!(!sandbox.directory.join("target").exists());
}

#[test]
fn eval_prints_a_warning_of_the_module_once_beside_a_warning_of_a_form() {
    let sandbox = Sandbox::new("eval_warnings").with_file("pick.knv", UNREACHABLE_ARM);
    let eval = sandbox.compiler(&["eval", "pick.knv", "(match 3 (_ 1) (_ 2))"]);
    let stderr = eval.assert_exit_code(0).stderr();
    assert_eq!(stderr.matches("--> pick.knv").count(), 1, "{stderr}");
    assert_eq!(stderr.matches("--> <eval>").count(), 1, "{stderr}");
    assert_eq!(eval.stdout(), "1\n");
}

#[test]
fn eval_prints_a_borrowed_value() {
    let sandbox = Sandbox::new("eval_borrowed").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&["eval", "session.knv", "(as-str (str \"ab\"))"]);
    assert_eq!(eval.assert_silent_success().stdout(), "\"ab\"\n");
}

#[test]
fn eval_notes_a_top_level_do_of_definitions() {
    let sandbox =
        Sandbox::new("eval_do_of_definitions").with_file("geometry.knv", GEOMETRY_DEFINITIONS);
    let eval = sandbox.compiler(&[
        "eval",
        "--line",
        "4",
        "--column",
        "1",
        "geometry.knv",
        "(do (defn twice (n) (* 2 n)))",
    ]);
    insta::assert_snapshot!(eval.assert_exit_code(0).stderr());
}

#[test]
fn eval_runs_a_top_level_do_of_expressions() {
    let sandbox =
        Sandbox::new("eval_do_of_expressions").with_file("square.knv", TOP_LEVEL_DO_OF_EXPRESSIONS);
    let eval = sandbox.compiler(&[
        "eval",
        "--line",
        "2",
        "--column",
        "1",
        "square.knv",
        "(do (prn (square 3)))",
    ]);
    assert_eq!(eval.assert_silent_success().stdout(), "9\n");
}

#[test]
fn eval_prints_a_warning_of_the_module_once_beside_a_failing_form() {
    let sandbox =
        Sandbox::new("eval_warning_beside_error").with_file("pick.knv", UNREACHABLE_ARM_IN_A_BODY);
    let eval = sandbox.compiler(&["eval", "pick.knv", "(square \"a\")"]);
    let stderr = eval.assert_exit_code(1).stderr();
    assert_eq!(
        stderr.matches("unreachable `match` arm").count(),
        1,
        "{stderr}"
    );
}

#[test]
fn eval_prints_a_warning_of_a_form_inside_a_definition_once() {
    let sandbox =
        Sandbox::new("eval_warning_inside").with_file("pick.knv", UNREACHABLE_ARM_IN_A_BODY);
    let eval = sandbox.compiler(&[
        "eval",
        "--line",
        "3",
        "--column",
        "3",
        "pick.knv",
        "(match 3 (_ 1) (_ 2))",
    ]);
    let stderr = eval.assert_exit_code(0).stderr();
    assert_eq!(
        stderr.matches("unreachable `match` arm").count(),
        1,
        "{stderr}"
    );
    assert_eq!(eval.stdout(), "1\n");
}

#[test]
fn eval_prints_a_value_behind_a_mutable_borrow() {
    let sandbox = Sandbox::new("eval_mutable").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&["eval", "session.knv", "(local v (vec 1)) (mut v)"]);
    assert_eq!(eval.assert_silent_success().stdout(), "(vec 1)\n");
}

#[test]
fn eval_reports_an_expression_beside_a_definition_in_a_top_level_do() {
    let sandbox = Sandbox::new("eval_mixed_do").with_file("mixed.knv", MIXED_TOP_LEVEL_DO);
    let eval = sandbox.compiler(&[
        "eval",
        "--line",
        "2",
        "--column",
        "1",
        "mixed.knv",
        "(do (defn cube (n) (* n (square n))) (prn 7))",
    ]);
    insta::assert_snapshot!(eval.assert_exit_code(1).stderr());
    assert_eq!(eval.stdout(), "");
}

#[test]
fn eval_prints_a_warning_of_a_form_at_the_range_of_a_module_warning() {
    let sandbox = Sandbox::new("eval_warning_same_range").with_file("pick.knv", UNREACHABLE_ARM);
    let indented = format!("{}(match 3 (_ 1) (_ 2))", " ".repeat(15));
    let eval = sandbox.compiler(&["eval", "pick.knv", &indented]);
    let stderr = eval.assert_exit_code(0).stderr();
    assert_eq!(stderr.matches("--> pick.knv").count(), 1, "{stderr}");
    assert_eq!(stderr.matches("--> <eval>").count(), 1, "{stderr}");
}

#[test]
fn eval_json_streams_output_and_values_in_order() {
    let sandbox = Sandbox::new("eval_json_streams").with_file("session.knv", SESSION);
    let forms = "(do (print \"x\") 6) (println \"hi\") (square 3)";
    let eval = sandbox.compiler(&["eval", "--json", "session.knv", forms]);
    assert_eq!(
        eval.assert_silent_success().json_replies(),
        [
            json!({"out": "x"}),
            json!({"value": "6", "form": 1}),
            json!({"out": "hi\n"}),
            json!({"value": "9", "form": 3}),
            json!({"status": "done", "exit": 0}),
        ]
    );
}

#[test]
fn eval_json_echoes_the_id_on_every_object() {
    let sandbox = Sandbox::new("eval_json_id").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&[
        "eval",
        "--json",
        "--id",
        "7",
        "session.knv",
        "(println \"a\") (square 2)",
    ]);
    let replies = eval.assert_silent_success().json_replies();
    assert_eq!(replies.len(), 3);
    assert!(
        replies.iter().all(|reply| reply["id"] == "7"),
        "{replies:?}"
    );
}

#[test]
fn eval_json_numbers_a_value_by_its_form_among_definitions() {
    let sandbox = Sandbox::new("eval_json_numbers").with_file("geometry.knv", EVALUATED_GEOMETRY);
    let eval = sandbox.compiler(&[
        "eval",
        "--json",
        "--line",
        "1",
        "--column",
        "1",
        "geometry.knv",
        "(defn area (w h) (* w h))\n(area 2 3)",
    ]);
    let replies = eval.assert_exit_code(0).json_replies();
    assert_eq!(replies[1], json!({"value": "6", "form": 2}));
}

#[test]
fn eval_json_prints_nothing_for_a_comment_form() {
    let sandbox = Sandbox::new("eval_json_comment").with_file("scratch.knv", SCRATCH);
    let eval = sandbox.compiler(&[
        "eval",
        "--json",
        "--line",
        "3",
        "--column",
        "1",
        "scratch.knv",
        "(comment (f 1) (exit 3))",
    ]);
    assert_eq!(
        eval.assert_silent_success().json_replies(),
        [json!({"status": "done", "exit": 0})]
    );
}

#[test]
fn eval_json_runs_a_form_sent_from_inside_a_comment() {
    let sandbox = Sandbox::new("eval_json_inside_comment").with_file("scratch.knv", SCRATCH);
    let eval = sandbox.compiler(&[
        "eval",
        "--json",
        "--line",
        "3",
        "--column",
        "10",
        "scratch.knv",
        "(f 1)",
    ]);
    assert_eq!(
        eval.assert_silent_success().json_replies(),
        [
            json!({"value": "2", "form": 1}),
            json!({"status": "done", "exit": 0})
        ]
    );
}

#[test]
fn eval_json_prints_a_value_that_borrows_a_temporary() {
    let sandbox = Sandbox::new("eval_json_temporary").with_file("session.knv", SESSION);
    let forms = "(trim (as-str (str \" a\" \"b \")))";
    let eval = sandbox.compiler(&["eval", "--json", "session.knv", forms]);
    assert_eq!(
        eval.assert_silent_success().json_replies()[0],
        json!({"value": "\"ab\"", "form": 1})
    );
}

#[test]
fn eval_json_carries_an_inferred_type_as_a_string_value() {
    let sandbox = Sandbox::new("eval_json_infer").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&["eval", "--json", "session.knv", "(infer (vec 1 2))"]);
    assert_eq!(
        eval.assert_silent_success().json_replies()[0],
        json!({"value": "\"(vec i64)\"", "form": 1})
    );
}

#[test]
fn eval_json_streams_a_doc_page_as_out() {
    let sandbox = Sandbox::new("eval_json_doc").with_file("page.knv", PAGE_PROGRAM);
    let eval = sandbox.compiler(&["eval", "--json", "page.knv", "(doc twice)"]);
    assert_eq!(
        eval.assert_silent_success().json_replies(),
        [
            json!({"out": TWICE_PAGE}),
            json!({"status": "done", "exit": 0})
        ]
    );
}

#[test]
fn eval_json_streams_standard_error_as_err() {
    let sandbox = Sandbox::new("eval_json_err").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&["eval", "--json", "session.knv", "(eprintln \"oops\")"]);
    assert_eq!(
        eval.assert_silent_success().json_replies(),
        [
            json!({"err": "oops\n"}),
            json!({"status": "done", "exit": 0})
        ]
    );
}

#[test]
fn eval_json_reports_a_note_as_a_diagnostic_without_a_code() {
    let sandbox = Sandbox::new("eval_json_note").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&["eval", "--json", "session.knv", "(fn ((x i64)) x)"]);
    let replies = eval.assert_silent_success().json_replies();
    let note = &replies[0]["diagnostics"][0];
    assert_eq!(note["severity"], "note");
    assert!(note.get("code").is_none(), "{note}");
    assert_eq!(
        (
            &note["file"],
            &note["line"],
            &note["column"],
            &note["end_column"]
        ),
        (&json!("<eval>"), &json!(1), &json!(1), &json!(17))
    );
    assert!(
        note["rendered"]
            .as_str()
            .is_some_and(|rendered| rendered.starts_with("note: the value has type"))
    );
    assert_eq!(replies[1], json!({"status": "done", "exit": 0}));
}

#[test]
fn eval_json_places_an_error_in_the_module_text_in_bytes() {
    let sandbox = Sandbox::new("eval_json_error").with_file(
        "geometry.knv",
        "(defn area (w h) (* w h))\n(do \"☃\" (area 2 \"3\"))\n",
    );
    let eval = sandbox.compiler(&[
        "eval",
        "--json",
        "--line",
        "2",
        "--column",
        "1",
        "geometry.knv",
        "(do \"☃\" (area 2 \"3\"))",
    ]);
    let replies = eval.assert_exit_code(1).json_replies();
    let error = &replies[0]["diagnostics"][0];
    assert_eq!(
        (&error["severity"], &error["code"], &error["file"]),
        (&json!("error"), &json!("L0401"), &json!("geometry.knv"))
    );
    assert_eq!(
        (
            &error["line"],
            &error["column"],
            &error["end_line"],
            &error["end_column"]
        ),
        (&json!(2), &json!(19), &json!(2), &json!(22))
    );
    assert!(eval.stdout().starts_with(
        r#"{"diagnostics":[{"severity":"error","code":"L0401","message":"mismatched types","file":"geometry.knv","line":2,"column":19,"end_line":2,"end_column":22,"rendered":"#
    ));
    assert_eq!(replies[1], json!({"status": "done", "exit": 1}));
}

#[test]
fn eval_json_reports_a_diagnostic_of_the_module_and_runs_nothing() {
    let sandbox =
        Sandbox::new("eval_json_module_error").with_file("geometry.knv", AREA_OF_A_STRING);
    let eval = sandbox.compiler(&["eval", "--json", "geometry.knv", "(println \"never\")"]);
    let replies = eval.assert_exit_code(1).json_replies();
    assert_eq!(replies.len(), 2, "{replies:?}");
    assert_eq!(replies[0]["diagnostics"][0]["code"], "L0401");
    assert_eq!(replies[1], json!({"status": "done", "exit": 1}));
    assert_eq!(eval.stderr(), "");
}

#[test]
fn eval_json_ends_with_the_exit_code_of_the_program() {
    let sandbox = Sandbox::new("eval_json_exit").with_file("exits.knv", EXITS);
    let eval = sandbox.compiler(&["eval", "--json", "exits.knv", "(exit 3)"]);
    assert_eq!(
        eval.assert_exit_code(3).json_replies(),
        [json!({"status": "done", "exit": 3})]
    );
}

#[test]
fn eval_json_ends_with_128_plus_the_signal_then_dies_by_it() {
    let sandbox = Sandbox::new("eval_json_signal").with_file("exits.knv", EXITS);
    let eval = sandbox.compiler(&["eval", "--json", "exits.knv", "(abort)"]);
    assert_eq!(
        eval.assert_killed_by(Outcome::SIGABRT).json_replies(),
        [json!({"status": "done", "exit": 134})]
    );
}

#[test]
fn eval_json_dies_by_sigpipe_when_its_output_closes() {
    let sandbox = Sandbox::new("eval_json_closed_output").with_file("session.knv", SESSION);
    let forms = "(for-each (range 0 1000000) (fn (i) (println i)))";
    sandbox
        .compiler_writing_to_a_closed_pipe(&["eval", "--json", "session.knv", forms])
        .assert_killed_by(Outcome::SIGPIPE);
}

#[test]
fn eval_json_writes_no_json_for_misplaced_forms() {
    let sandbox = Sandbox::new("eval_json_misplaced").with_file("geometry.knv", EVALUATED_GEOMETRY);
    let eval = sandbox.compiler(&[
        "eval",
        "--json",
        "--line",
        "9",
        "--column",
        "1",
        "geometry.knv",
        "(area 2 3)",
    ]);
    assert_eq!(eval.assert_exit_code(2).stdout(), "");
    assert!(
        eval.stderr()
            .starts_with("error: the forms are not the text")
    );
}

#[test]
fn eval_json_writes_no_json_for_a_missing_module() {
    let sandbox = Sandbox::new("eval_json_missing");
    let eval = sandbox.compiler(&["eval", "--json", "missing.knv", "1"]);
    assert_eq!(eval.assert_exit_code(2).stdout(), "");
    assert!(
        eval.stderr()
            .starts_with("error: cannot read `missing.knv`")
    );
}

#[test]
fn eval_rejects_an_id_without_json() {
    let sandbox = Sandbox::new("eval_id_without_json").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&["eval", "--id", "7", "session.knv", "1"]);
    assert_eq!(eval.assert_exit_code(2).stdout(), "");
}

#[test]
fn eval_json_prints_a_value_beside_a_module_println() {
    let sandbox = Sandbox::new("eval_json_shadowed").with_file("square.knv", SHADOWED_PRINTLN);
    let eval = sandbox.compiler(&["eval", "--json", "square.knv", "(square 2)"]);
    assert_eq!(
        eval.assert_silent_success().json_replies()[0],
        json!({"value": "4", "form": 1})
    );
}

#[test]
fn eval_json_reports_a_warning_with_its_severity_and_code() {
    let sandbox = Sandbox::new("eval_json_warning").with_file("pick.knv", UNREACHABLE_ARM);
    let eval = sandbox.compiler(&["eval", "--json", "pick.knv", "(do)"]);
    let replies = eval.assert_silent_success().json_replies();
    let warning = &replies[0]["diagnostics"][0];
    assert_eq!(warning["severity"], "warning");
    assert!(warning["code"].is_string(), "{warning}");
}

#[test]
fn eval_json_stops_a_program_whose_output_closes() {
    let sandbox = Sandbox::new("eval_json_closed_count").with_file("count.knv", COUNT_THEN_FAIL);
    sandbox
        .compiler_writing_to_a_closed_pipe(&["eval", "--json", "count.knv", "(count-up 0)"])
        .assert_killed_by(Outcome::SIGPIPE);
}

#[test]
fn eval_json_ends_with_status_2_for_a_program_that_cannot_start() {
    let sandbox = Sandbox::new("eval_json_missing_program")
        .with_file("session.knv", SESSION)
        .with_executable("tools/rustc", SILENT_RUSTC)
        .with_executable("tools/rustfmt", PASSTHROUGH_RUSTFMT);
    let eval = sandbox.compiler_with_path("tools", &["eval", "--json", "session.knv", "1"]);
    assert_eq!(
        eval.assert_exit_code(2).json_replies(),
        [json!({"status": "done", "exit": 2})]
    );
    assert!(eval.stderr().starts_with("error: cannot run"));
}

#[test]
fn eval_json_ends_with_status_2_without_rustc() {
    let sandbox = Sandbox::new("eval_json_missing_rustc")
        .with_file("pick.knv", UNREACHABLE_ARM)
        .with_executable("tools/rustfmt", PASSTHROUGH_RUSTFMT);
    let eval = sandbox.compiler_with_path("tools", &["eval", "--json", "pick.knv", "(pick 1)"]);
    let replies = eval.assert_exit_code(2).json_replies();
    assert_eq!(replies[0]["diagnostics"][0]["severity"], "warning");
    assert_eq!(replies[1], json!({"status": "done", "exit": 2}));
}

#[test]
fn eval_json_ends_with_status_2_without_a_build_directory() {
    let sandbox = Sandbox::new("eval_json_no_temporary").with_file("session.knv", SESSION);
    let eval = sandbox
        .compiler_with_temporary_directory("missing", &["eval", "--json", "session.knv", "1"]);
    assert_eq!(
        eval.assert_exit_code(2).json_replies(),
        [json!({"status": "done", "exit": 2})]
    );
    assert!(
        eval.stderr()
            .starts_with("error: cannot create a build directory")
    );
}

#[test]
fn eval_json_prints_values_of_two_types_in_one_call() {
    let sandbox = Sandbox::new("eval_json_two_types").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&[
        "eval",
        "--json",
        "session.knv",
        "(square 3) (->point :x 4 :y 3)",
    ]);
    assert_eq!(
        eval.assert_silent_success().json_replies(),
        [
            json!({"value": "9", "form": 1}),
            json!({"value": "(->point :x 4 :y 3)", "form": 2}),
            json!({"status": "done", "exit": 0}),
        ]
    );
}

#[test]
fn eval_json_reports_a_reader_error_in_the_forms() {
    let sandbox = Sandbox::new("eval_json_reader_error").with_file("session.knv", SESSION);
    let eval = sandbox.compiler(&["eval", "--json", "session.knv", "(square 2"]);
    let replies = eval.assert_exit_code(1).json_replies();
    assert_eq!(replies[0]["diagnostics"][0]["code"], "L0101");
    assert_eq!(replies[1], json!({"status": "done", "exit": 1}));
    assert_eq!(eval.stderr(), "");
}

#[test]
fn color_diagnostics_on_a_terminal() {
    let sandbox = Sandbox::new("color_terminal").with_file("unbound.knv", UNBOUND_NAME);
    let check = sandbox.compiler_on_a_terminal(&[("TERM", "xterm")], &["check", "unbound.knv"]);
    assert!(check.stdout_is_colored(), "{}", check.stdout());
}

#[test]
fn color_stays_off_diagnostics_on_a_dumb_terminal() {
    let sandbox = Sandbox::new("color_dumb").with_file("unbound.knv", UNBOUND_NAME);
    let check = sandbox.compiler_on_a_terminal(&[("TERM", "dumb")], &["check", "unbound.knv"]);
    assert!(
        check.stdout().contains("error[L0201]"),
        "{}",
        check.stdout()
    );
    assert!(!check.stdout_is_colored(), "{}", check.stdout());
}

#[test]
fn color_stays_off_diagnostics_on_a_terminal_when_clicolor_is_zero() {
    let sandbox = Sandbox::new("color_clicolor").with_file("unbound.knv", UNBOUND_NAME);
    let environment = [("TERM", "xterm"), ("CLICOLOR", "0")];
    let check = sandbox.compiler_on_a_terminal(&environment, &["check", "unbound.knv"]);
    assert!(
        check.stdout().contains("error[L0201]"),
        "{}",
        check.stdout()
    );
    assert!(!check.stdout_is_colored(), "{}", check.stdout());
}

#[test]
fn color_stays_off_diagnostics_on_a_terminal_when_no_color_is_set() {
    let sandbox = Sandbox::new("color_no_color").with_file("unbound.knv", UNBOUND_NAME);
    let environment = [("TERM", "xterm"), ("NO_COLOR", "1")];
    let check = sandbox.compiler_on_a_terminal(&environment, &["check", "unbound.knv"]);
    assert!(
        check.stdout().contains("error[L0201]"),
        "{}",
        check.stdout()
    );
    assert!(!check.stdout_is_colored(), "{}", check.stdout());
}

#[test]
fn color_stays_off_diagnostics_written_to_a_file_from_a_terminal() {
    let sandbox = Sandbox::new("color_file").with_file("unbound.knv", UNBOUND_NAME);
    let arguments = ["check", "unbound.knv", "2>", "errors.log"];
    sandbox.compiler_on_a_terminal(&[("TERM", "xterm")], &arguments);
    let errors = sandbox.written_file("errors.log");
    assert!(errors.contains("error[L0201]"), "{errors}");
    assert!(!errors.contains(Outcome::ESCAPE), "{errors}");
}

#[test]
fn color_stays_off_diagnostics_in_a_pipe() {
    let sandbox = Sandbox::new("color_pipe").with_file("unbound.knv", UNBOUND_NAME);
    let check = sandbox.compiler_with_colors(&[("TERM", "xterm")], &["check", "unbound.knv"]);
    assert!(
        !check.assert_exit_code(1).stderr_is_colored(),
        "{}",
        check.stderr()
    );
}

#[test]
fn color_diagnostics_in_a_pipe_when_clicolor_force_is_set() {
    let sandbox = Sandbox::new("color_force").with_file("unbound.knv", UNBOUND_NAME);
    let check = sandbox.compiler_with_colors(&[("CLICOLOR_FORCE", "1")], &["check", "unbound.knv"]);
    assert!(
        check.assert_exit_code(1).stderr_is_colored(),
        "{}",
        check.stderr()
    );
}

#[test]
fn color_no_color_wins_over_clicolor_force() {
    let sandbox = Sandbox::new("color_no_color_force").with_file("unbound.knv", UNBOUND_NAME);
    let environment = [("CLICOLOR_FORCE", "1"), ("NO_COLOR", "1")];
    let check = sandbox.compiler_with_colors(&environment, &["check", "unbound.knv"]);
    assert!(
        !check.assert_exit_code(1).stderr_is_colored(),
        "{}",
        check.stderr()
    );
}

#[test]
fn color_follows_the_decision_in_a_doc_term_that_does_not_read() {
    let sandbox = Sandbox::new("color_doc_term");
    let doc = sandbox.compiler_with_colors(&[("CLICOLOR_FORCE", "1")], &["doc", "("]);
    assert!(
        doc.assert_exit_code(2).stderr_is_colored(),
        "{}",
        doc.stderr()
    );
}

#[test]
fn color_follows_the_decision_in_eval_forms_that_do_not_read() {
    let sandbox = Sandbox::new("color_eval_forms").with_file("session.knv", SESSION);
    let environment = [("CLICOLOR_FORCE", "1")];
    let eval = sandbox.compiler_with_colors(&environment, &["eval", "session.knv", "("]);
    assert!(eval.stderr().contains("L0101"), "{}", eval.stderr());
    assert!(eval.stderr_is_colored(), "{}", eval.stderr());
}

#[test]
fn color_always_colors_diagnostics_in_a_pipe() {
    let sandbox = Sandbox::new("color_always").with_file("unbound.knv", UNBOUND_NAME);
    let check = sandbox.compiler_with_colors(&[], &["--color", "always", "check", "unbound.knv"]);
    assert!(
        check.assert_exit_code(1).stderr_is_colored(),
        "{}",
        check.stderr()
    );
}

#[test]
fn color_never_wins_over_clicolor_force_after_the_subcommand() {
    let sandbox = Sandbox::new("color_never").with_file("unbound.knv", UNBOUND_NAME);
    let environment = [("CLICOLOR_FORCE", "1")];
    let check =
        sandbox.compiler_with_colors(&environment, &["check", "unbound.knv", "--color=never"]);
    assert!(
        !check.assert_exit_code(1).stderr_is_colored(),
        "{}",
        check.stderr()
    );
}

#[test]
fn color_never_keeps_the_help_plain_before_the_subcommand() {
    let sandbox = Sandbox::new("color_never_help");
    let environment = [("CLICOLOR_FORCE", "1")];
    let help = sandbox.compiler_with_colors(&environment, &["--color", "never", "--help"]);
    assert!(
        !help.assert_exit_code(0).stdout_is_colored(),
        "{}",
        help.stdout()
    );
}

#[test]
fn color_never_keeps_the_help_plain_after_the_subcommand() {
    let sandbox = Sandbox::new("color_never_command_help");
    let environment = [("CLICOLOR_FORCE", "1")];
    let help = sandbox.compiler_with_colors(&environment, &["check", "--help", "--color=never"]);
    assert!(
        !help.assert_exit_code(0).stdout_is_colored(),
        "{}",
        help.stdout()
    );
}

#[test]
fn color_always_colors_the_help_after_the_subcommand() {
    let sandbox = Sandbox::new("color_always_command_help");
    let help = sandbox.compiler_with_colors(&[], &["check", "--help", "--color", "always"]);
    assert!(
        help.assert_exit_code(0).stdout_is_colored(),
        "{}",
        help.stdout()
    );
}

#[test]
fn color_never_keeps_a_usage_error_plain() {
    let sandbox = Sandbox::new("color_never_usage");
    let environment = [("CLICOLOR_FORCE", "1")];
    let usage = sandbox.compiler_with_colors(&environment, &["--color", "never", "nonsense"]);
    assert!(
        !usage.assert_exit_code(2).stderr_is_colored(),
        "{}",
        usage.stderr()
    );
}

#[test]
fn color_always_colors_a_usage_error() {
    let sandbox = Sandbox::new("color_always_usage");
    let usage = sandbox.compiler_with_colors(&[], &["--color", "always", "nonsense"]);
    assert!(
        usage.assert_exit_code(2).stderr_is_colored(),
        "{}",
        usage.stderr()
    );
}

#[test]
fn color_after_the_end_of_options_is_not_read() {
    let sandbox = Sandbox::new("color_after_end");
    let usage = sandbox.compiler_with_colors(&[], &["nonsense", "--", "--color", "always"]);
    assert!(
        !usage.assert_exit_code(2).stderr_is_colored(),
        "{}",
        usage.stderr()
    );
}

#[test]
fn color_with_an_unknown_value_is_a_usage_error_under_auto() {
    let sandbox = Sandbox::new("color_unknown").with_file("unbound.knv", UNBOUND_NAME);
    let environment = [("CLICOLOR_FORCE", "1")];
    let arguments = ["--color", "sometimes", "check", "unbound.knv"];
    let forced = sandbox.compiler_with_colors(&environment, &arguments);
    let piped = sandbox.compiler_with_colors(&[], &arguments);
    assert!(
        forced.assert_exit_code(2).stderr().contains("sometimes"),
        "{}",
        forced.stderr()
    );
    assert!(forced.stderr_is_colored(), "{}", forced.stderr());
    assert!(
        !piped.assert_exit_code(2).stderr_is_colored(),
        "{}",
        piped.stderr()
    );
}

#[test]
fn color_follows_the_last_color_argument() {
    let sandbox = Sandbox::new("color_last");
    let arguments = ["--color", "always", "check", "--help", "--color", "never"];
    let help = sandbox.compiler_with_colors(&[], &arguments);
    assert!(
        !help.assert_exit_code(0).stdout_is_colored(),
        "{}",
        help.stdout()
    );
}

#[test]
fn color_always_keeps_eval_json_plain() {
    let sandbox = Sandbox::new("color_json").with_file("session.knv", SESSION);
    let arguments = [
        "--color",
        "always",
        "eval",
        "--json",
        "session.knv",
        "(+ 1 \"b\")",
    ];
    let eval = sandbox.compiler_with_colors(&[], &arguments);
    let replies = eval.assert_exit_code(1).json_replies();
    let rendered = replies[0]["diagnostics"][0]["rendered"].as_str();
    assert!(
        rendered.is_some_and(|text| text.starts_with("error[L0401]")),
        "{rendered:?}"
    );
}

#[test]
fn eval_notes_a_struct_that_the_module_defines() {
    let sandbox = Sandbox::new("eval_struct_definition").with_file("points.knv", POINTS);
    let eval = sandbox.compiler(&[
        "eval",
        "--line",
        "1",
        "--column",
        "1",
        "points.knv",
        "(defstruct point :x i64 :y i64)",
    ]);
    assert_eq!(eval.assert_exit_code(0).stdout(), "");
}

#[test]
fn repl_keeps_definitions_and_prints_expressions() {
    let sandbox = Sandbox::new("repl_definitions");
    let repl = sandbox.compiler_reading(PROMPT_DEFINITIONS, &["repl"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn repl_prints_a_doc_page_as_text() {
    let sandbox = Sandbox::new("repl_doc");
    let repl = sandbox.compiler_reading(PROMPT_DOC, &["repl"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn repl_keeps_its_session_after_a_macro_passes_the_step_limit() {
    let sandbox = Sandbox::new("repl_step_limit");
    let repl = sandbox.compiler_reading(PROMPT_RUNAWAY_MACRO, &["repl"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn eval_reports_a_macro_that_passes_the_step_limit() {
    let sandbox = Sandbox::new("eval_step_limit")
        .with_file("session.knv", "(defmacro spin () ((fn () (recur))))\n");
    let eval = sandbox.compiler(&["eval", "session.knv", "(with-macro-steps 1000 (spin))"]);
    insta::assert_snapshot!(eval.transcript());
}

#[test]
fn repl_prints_an_inferred_type_as_a_string() {
    let sandbox = Sandbox::new("repl_infer");
    let repl = sandbox.compiler_reading(PROMPT_INFER, &["repl"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn repl_replaces_an_earlier_definition_of_the_same_name() {
    let sandbox = Sandbox::new("repl_redefinition");
    let repl = sandbox.compiler_reading(PROMPT_REDEFINITION, &["repl"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn repl_sets_aside_a_name_that_a_replacement_drops() {
    let sandbox = Sandbox::new("repl_dropping_replacement");
    let repl = sandbox.compiler_reading(PROMPT_DROPPING_REPLACEMENT, &["repl"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn repl_replaces_every_earlier_input_that_shares_a_name() {
    let sandbox = Sandbox::new("repl_replacement_of_two");
    let repl = sandbox.compiler_reading(PROMPT_REPLACEMENT_OF_TWO, &["repl"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn repl_rejects_a_failing_definition_and_keeps_the_buffer() {
    let sandbox = Sandbox::new("repl_failing_definition");
    let repl = sandbox.compiler_reading(PROMPT_FAILING_DEFINITION, &["repl"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn repl_enters_each_form_of_a_line_as_its_own_input() {
    let sandbox = Sandbox::new("repl_forms_on_one_line");
    let repl = sandbox.compiler_reading(PROMPT_FORMS_ON_ONE_LINE, &["repl"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn repl_reads_an_unclosed_list_or_string_across_lines() {
    let sandbox = Sandbox::new("repl_multi_line");
    let repl = sandbox.compiler_reading(PROMPT_MULTI_LINE, &["repl"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn repl_ends_an_input_at_an_extra_close() {
    let sandbox = Sandbox::new("repl_extra_close");
    let repl = sandbox.compiler_reading(PROMPT_EXTRA_CLOSE, &["repl"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn repl_resolves_use_from_the_root_and_sets_aside_an_import_it_drops() {
    let sandbox = Sandbox::new("repl_uses").with_file("lib/shapes.knv", SHAPES_OF_TWO);
    let repl = sandbox.compiler_reading(PROMPT_USES, &["repl", "--root", "lib"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn repl_shows_a_panic_and_keeps_its_definitions() {
    let sandbox = Sandbox::new("repl_panic");
    let repl = sandbox.compiler_reading(PROMPT_PANIC, &["repl"]);
    let stderr = repl.assert_exit_code(0).stderr();
    assert!(stderr.contains("index out of bounds"), "{stderr}");
    assert!(
        stderr.ends_with("note: the program exited with code 101\n"),
        "{stderr}"
    );
    assert_eq!(repl.stdout(), "8\n");
}

#[test]
fn repl_notes_128_plus_the_signal_that_kills_a_program_and_keeps_going() {
    let sandbox = Sandbox::new("repl_abort");
    let repl = sandbox.compiler_reading(PROMPT_ABORT, &["repl"]);
    let stderr = repl.assert_exit_code(0).stderr();
    assert!(
        stderr.contains("note: the program exited with code 134\n"),
        "{stderr}"
    );
    assert_eq!(repl.stdout(), "3\n");
}

#[test]
fn repl_notes_a_nonzero_exit_of_a_program() {
    let sandbox = Sandbox::new("repl_exit");
    let repl = sandbox.compiler_reading(PROMPT_EXIT, &["repl"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn repl_reports_the_end_of_input_inside_a_list_and_quits() {
    let sandbox = Sandbox::new("repl_unclosed_end");
    let repl = sandbox.compiler_reading(PROMPT_UNCLOSED_END, &["repl"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn repl_replaces_an_enum_by_every_name_it_defines() {
    let sandbox = Sandbox::new("repl_enum_redefinition");
    let repl = sandbox.compiler_reading(PROMPT_ENUM_REDEFINITION, &["repl"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn repl_prints_a_warning_of_a_definition_once() {
    let sandbox = Sandbox::new("repl_warning");
    let repl = sandbox.compiler_reading(PROMPT_WARNING, &["repl"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn repl_reports_unreadable_input() {
    let sandbox = Sandbox::new("repl_unreadable_input");
    let repl = sandbox.compiler_reading(PROMPT_NOT_UTF8, &["repl"]);
    assert_eq!(repl.assert_exit_code(2).stdout(), "2\n");
    assert_eq!(
        repl.stderr(),
        "error: cannot read standard input: stream did not contain valid UTF-8\n"
    );
}

#[test]
fn eval_reports_unreadable_input() {
    let sandbox = Sandbox::new("eval_unreadable_input");
    let eval = sandbox.compiler_reading(b"\xff", &["eval", "--stdin", "buffer.knv", "1"]);
    assert_eq!(eval.assert_exit_code(2).stdout(), "");
    assert_eq!(
        eval.stderr(),
        "error: cannot read standard input: stream did not contain valid UTF-8\n"
    );
}

struct PromptColors;

impl PromptColors {
    const EXIT_NOTE: &str = "the program exited with code 3";
    const CONTROL_SEQUENCE: &str = "\u{1b}[";

    fn prompt() -> String {
        let style = DiagnosticRenderer::GUTTER_STYLE;
        format!("{style}> {style:#}")
    }

    fn exit_note() -> String {
        DiagnosticRenderer::styled(&SourceMap::default()).render_note(Self::EXIT_NOTE)
    }

    fn plain_exit_note() -> String {
        format!("note: {}", Self::EXIT_NOTE)
    }

    fn is_styled(session: &str) -> bool {
        session
            .split(Self::CONTROL_SEQUENCE)
            .skip(1)
            .any(|sequence| {
                sequence
                    .trim_start_matches(|character: char| {
                        character.is_ascii_digit() || character == ';'
                    })
                    .starts_with('m')
            })
    }

    fn session(name: &str, environment: &[(&str, &str)], arguments: &[&str]) -> String {
        let sandbox = Sandbox::new(name);
        sandbox
            .compiler_on_a_terminal_reading(PROMPT_EXIT_ON_A_TERMINAL, environment, arguments)
            .assert_exit_code(0)
            .stdout()
    }
}

#[test]
fn color_repl_prompt_and_exit_note_on_a_terminal() {
    let session = PromptColors::session("color_repl", &[("TERM", "xterm")], &["repl"]);
    assert!(session.contains(&PromptColors::prompt()), "{session:?}");
    assert!(session.contains(&PromptColors::exit_note()), "{session:?}");
}

#[test]
fn color_stays_off_the_repl_prompt_and_exit_note_when_no_color_is_set() {
    let session = PromptColors::session(
        "color_repl_no_color",
        &[("TERM", "xterm"), ("NO_COLOR", "1")],
        &["repl"],
    );
    assert!(!PromptColors::is_styled(&session), "{session:?}");
    assert!(session.contains("> "), "{session:?}");
    assert!(
        session.contains(&PromptColors::plain_exit_note()),
        "{session:?}"
    );
}

#[test]
fn color_never_keeps_the_repl_prompt_and_exit_note_plain() {
    let session = PromptColors::session(
        "color_repl_never",
        &[("TERM", "xterm")],
        &["--color", "never", "repl"],
    );
    assert!(!PromptColors::is_styled(&session), "{session:?}");
    assert!(session.contains("> "), "{session:?}");
    assert!(
        session.contains(&PromptColors::plain_exit_note()),
        "{session:?}"
    );
}

struct ValueColors;

impl ValueColors {
    const TAG: &str = "(defstruct tag :name string :size i64)\n";
    const TAG_FORM: &str = "(->tag :name (str \"an\" \"a\") :size (+ 1 1))";
    const TAG_VALUE: &str = "(->tag :name \"ana\" :size 2)";
    const PROGRAM_COLORS: &str = "\u{1b}[35mviolet\u{1b}[0m";

    fn painted_value() -> String {
        let keyword = anstyle::AnsiColor::Cyan.on_default();
        format!(
            "(->tag {} {} {} {})",
            Self::styled(keyword, ":name"),
            Self::styled(anstyle::AnsiColor::Green.on_default(), "\"ana\""),
            Self::styled(keyword, ":size"),
            Self::styled(anstyle::AnsiColor::Yellow.on_default(), "2"),
        )
    }

    fn styled(style: anstyle::Style, text: &str) -> String {
        format!("{style}{text}{style:#}")
    }

    fn assert_painted(output: &str) {
        let painted = Self::painted_value();
        assert!(output.contains(&painted), "{painted:?} in {output:?}");
    }

    fn assert_plain(output: &str) {
        assert!(output.contains(Self::TAG_VALUE), "{output:?}");
    }

    fn typed_value() -> String {
        format!("{}\r", Self::TAG_FORM)
    }

    fn session(name: &str, environment: &[(&str, &str)], arguments: &[&str]) -> String {
        let environment: Vec<(&str, &str)> = [("TERM", "xterm")]
            .into_iter()
            .chain(environment.iter().copied())
            .collect();
        let typed_tag = Self::TAG.replace('\n', "\r");
        let typed_value = Self::typed_value();
        Sandbox::new(name)
            .compiler_on_a_terminal_reading(
                &[
                    Keys::AtPrompt(&typed_tag),
                    Keys::AtPrompt(&typed_value),
                    Keys::AtPrompt(Keys::END_OF_INPUT),
                ],
                &environment,
                arguments,
            )
            .assert_exit_code(0)
            .stdout()
    }

    fn eval_on_a_terminal(name: &str, environment: &[(&str, &str)], forms: &str) -> Outcome {
        Self::eval_on_a_terminal_then(name, environment, forms, &[])
    }

    fn eval_on_a_terminal_then(
        name: &str,
        environment: &[(&str, &str)],
        forms: &str,
        shell_words: &[&str],
    ) -> Outcome {
        let environment: Vec<(&str, &str)> = [("TERM", "xterm")]
            .into_iter()
            .chain(environment.iter().copied())
            .collect();
        let quoted = format!("'{forms}'");
        let arguments: Vec<&str> = ["eval", "tags.knv", &quoted]
            .into_iter()
            .chain(shell_words.iter().copied())
            .collect();
        Sandbox::new(name)
            .with_file("tags.knv", Self::TAG)
            .compiler_on_a_terminal(&environment, &arguments)
    }
}

#[test]
fn color_repl_values_by_token_kind() {
    let session = ValueColors::session("color_repl_values", &[], &["repl"]);
    ValueColors::assert_painted(&session);
}

#[test]
fn color_repl_passes_the_program_escape_codes_unchanged() {
    let sandbox = Sandbox::new("color_repl_program_codes");
    let session = sandbox
        .compiler_on_a_terminal_reading(
            &[
                Keys::AtPrompt("(println \"\\u{1b}[35mviolet\\u{1b}[0m\")\r"),
                Keys::AtPrompt(Keys::END_OF_INPUT),
            ],
            &[("TERM", "xterm")],
            &["repl"],
        )
        .assert_exit_code(0)
        .stdout();
    assert!(session.contains(ValueColors::PROGRAM_COLORS), "{session:?}");
}

#[test]
fn color_repl_shows_the_program_prompt_before_it_reads() {
    let sandbox = Sandbox::new("color_repl_program_prompt");
    let session = sandbox
        .compiler_on_a_terminal_reading(
            &[
                Keys::AtPrompt("(do (println (str \"na\" \"me?\")) (read-line))\r"),
                Keys::AfterOutput("name?", "ana\r"),
                Keys::AtPrompt(Keys::END_OF_INPUT),
            ],
            &[("TERM", "xterm")],
            &["repl"],
        )
        .assert_exit_code(0)
        .stdout();
    let painted = ValueColors::styled(anstyle::AnsiColor::Green.on_default(), "\"ana\"");
    assert!(session.contains(&painted), "{session:?}");
}

#[test]
fn color_stays_off_repl_values_when_no_color_is_set() {
    let session = ValueColors::session(
        "color_repl_values_no_color",
        &[("NO_COLOR", "1")],
        &["repl"],
    );
    ValueColors::assert_plain(&session);
}

#[test]
fn color_never_keeps_repl_values_plain() {
    let session = ValueColors::session(
        "color_repl_values_never",
        &[],
        &["--color", "never", "repl"],
    );
    ValueColors::assert_plain(&session);
}

#[test]
fn color_stays_off_repl_values_in_a_pipe() {
    let input = format!("{}{}\n", ValueColors::TAG, ValueColors::TAG_FORM);
    let repl = Sandbox::new("color_repl_values_pipe").compiler_reading(input, &["repl"]);
    let output = repl.assert_exit_code(0).stdout();
    assert!(!output.contains(Outcome::ESCAPE), "{output:?}");
    ValueColors::assert_plain(&output);
}

#[test]
fn color_eval_values_on_a_terminal() {
    let eval = ValueColors::eval_on_a_terminal("color_eval_values", &[], ValueColors::TAG_FORM);
    ValueColors::assert_painted(&eval.assert_exit_code(0).stdout());
}

#[test]
fn color_stays_off_eval_values_when_no_color_is_set() {
    let eval = ValueColors::eval_on_a_terminal(
        "color_eval_values_no_color",
        &[("NO_COLOR", "1")],
        ValueColors::TAG_FORM,
    );
    ValueColors::assert_plain(&eval.assert_exit_code(0).stdout());
}

#[test]
fn color_eval_keeps_a_piped_value_plain_and_colors_the_diagnostics() {
    let forms = format!("(iter (vec 1 2)) {}", ValueColors::TAG_FORM);
    let eval =
        ValueColors::eval_on_a_terminal_then("color_eval_piped_value", &[], &forms, &["|", "cat"]);
    let output = eval.assert_exit_code(0).stdout();
    ValueColors::assert_plain(&output);
    assert!(output.contains("note"), "{output:?}");
    assert!(eval.stdout_is_colored(), "{output:?}");
}

struct DocColors;

impl DocColors {
    const SCALE_PROGRAM: &str = "(defn scale (:size i64 :factor i64 (defaults :factor 2))\n  \"Scales :size by \\\"factor\\\" 2.\"\n  (* size factor))\n";
    const SCALE_BODY: &str = "\n\nScales :size by \"factor\" 2.\n";
    const SCALE_HEADER: &str = "(defn scale (:size i64 :factor i64 (defaults :factor 2)))";

    fn painted_header() -> String {
        let keyword = |text| ValueColors::styled(anstyle::AnsiColor::Cyan.on_default(), text);
        let number = ValueColors::styled(anstyle::AnsiColor::Yellow.on_default(), "2");
        format!(
            "(defn scale ({} i64 {} i64 (defaults {} {number})))",
            keyword(":size"),
            keyword(":factor"),
            keyword(":factor"),
        )
    }

    fn page_on_a_terminal(name: &str, environment: &[(&str, &str)], flags: &[&str]) -> String {
        let environment: Vec<(&str, &str)> = [("TERM", "xterm")]
            .into_iter()
            .chain(environment.iter().copied())
            .collect();
        let arguments: Vec<&str> = ["doc"]
            .into_iter()
            .chain(flags.iter().copied())
            .chain(["--file", "scale.knv", "scale"])
            .collect();
        Sandbox::new(name)
            .with_file("scale.knv", Self::SCALE_PROGRAM)
            .compiler_on_a_terminal(&environment, &arguments)
            .assert_exit_code(0)
            .stdout()
            .replace("\r\n", "\n")
    }
}

#[test]
fn color_doc_header_on_a_terminal_and_leave_the_docstring_plain() {
    let page = DocColors::page_on_a_terminal("color_doc_header", &[], &[]);
    let expected = format!("{}{}", DocColors::painted_header(), DocColors::SCALE_BODY);
    assert!(page.ends_with(&expected), "{page:?}");
}

#[test]
fn color_doc_header_below_the_type_line() {
    let page = DocColors::page_on_a_terminal("color_doc_typed_header", &[], &["--types"]);
    let expected = format!("{}{}", DocColors::painted_header(), DocColors::SCALE_BODY);
    assert!(page.ends_with(&expected), "{page:?}");
}

#[test]
fn color_stays_off_the_doc_header_when_no_color_is_set() {
    let page = DocColors::page_on_a_terminal("color_doc_no_color", &[("NO_COLOR", "1")], &[]);
    let expected = format!("{}{}", DocColors::SCALE_HEADER, DocColors::SCALE_BODY);
    assert!(page.ends_with(&expected), "{page:?}");
}

struct RedefinitionSession;

impl RedefinitionSession {
    fn transcript(name: &str, input: &str) -> String {
        let sandbox = Sandbox::new(name).with_file("shapes.knv", SHAPES_OF_TWO);
        sandbox.compiler_reading(input, &["repl"]).transcript()
    }

    fn after_a_module_change(rest: &str) -> String {
        format!(
            "{WRITE_FILE}(use shapes circle square)\n(defn area () (square 2))\n{SHAPES_WITHOUT_SQUARE}{rest}"
        )
    }
}

#[test]
fn repl_sets_aside_an_input_that_a_redefinition_breaks() {
    insta::assert_snapshot!(RedefinitionSession::transcript(
        "repl_redefined_struct",
        PROMPT_REDEFINED_STRUCT
    ));
}

#[test]
fn repl_rejects_a_failed_redefinition_and_keeps_the_old_definition() {
    insta::assert_snapshot!(RedefinitionSession::transcript(
        "repl_failed_redefined_struct",
        PROMPT_FAILED_REDEFINED_STRUCT
    ));
}

#[test]
fn repl_sets_aside_the_inputs_that_a_set_aside_input_breaks() {
    insta::assert_snapshot!(RedefinitionSession::transcript(
        "repl_cascade",
        PROMPT_CASCADE
    ));
}

#[test]
fn repl_rejects_a_call_to_a_broken_macro() {
    insta::assert_snapshot!(RedefinitionSession::transcript(
        "repl_broken_macro_called",
        PROMPT_BROKEN_MACRO_CALLED
    ));
}

#[test]
fn repl_rejects_a_broken_macro_that_an_earlier_input_calls() {
    insta::assert_snapshot!(RedefinitionSession::transcript(
        "repl_broken_macro_redefined",
        PROMPT_BROKEN_MACRO_REDEFINED
    ));
}

#[test]
fn repl_notes_a_name_that_an_input_stops_binding() {
    insta::assert_snapshot!(RedefinitionSession::transcript(
        "repl_stops_binding",
        PROMPT_STOPS_BINDING
    ));
}

#[test]
fn repl_sets_aside_the_later_of_two_inputs_that_bind_one_name() {
    insta::assert_snapshot!(RedefinitionSession::transcript(
        "repl_two_binders",
        PROMPT_TWO_BINDERS
    ));
}

#[test]
fn repl_notes_what_a_rejected_input_would_set_aside() {
    insta::assert_snapshot!(RedefinitionSession::transcript(
        "repl_would_set_aside",
        PROMPT_WOULD_SET_ASIDE
    ));
}

#[test]
fn repl_sets_aside_the_inputs_of_a_module_changed_on_disk() {
    let input = RedefinitionSession::after_a_module_change("(defn z () 1)\n(area)\n");
    insta::assert_snapshot!(RedefinitionSession::transcript(
        "repl_module_changed",
        &input
    ));
}

#[test]
fn repl_helps_an_expression_after_a_module_change() {
    let input = RedefinitionSession::after_a_module_change("(circle 2)\n");
    insta::assert_snapshot!(RedefinitionSession::transcript(
        "repl_expression_after_module_change",
        &input
    ));
}

#[test]
fn repl_forgets_the_record_of_a_name_entered_again() {
    let sandbox = Sandbox::new("repl_re_entered_type").with_file("geo.knv", GEO_WITHOUT_POINT);
    let repl = sandbox.compiler_reading(PROMPT_RE_ENTERED_TYPE, &["repl"]);
    insta::assert_snapshot!(repl.transcript());
}

#[test]
fn repl_gives_an_expression_calling_a_broken_macro_no_module_help() {
    insta::assert_snapshot!(RedefinitionSession::transcript(
        "repl_expression_broken_macro",
        PROMPT_EXPRESSION_CALLING_A_BROKEN_MACRO
    ));
}

#[test]
fn repl_warns_once_for_an_input_with_two_errors() {
    insta::assert_snapshot!(RedefinitionSession::transcript(
        "repl_two_errors_in_one_input",
        PROMPT_TWO_ERRORS_IN_ONE_INPUT
    ));
}

#[test]
fn repl_keeps_the_suggestion_of_a_module_typo_before_the_help_to_enter_again() {
    let input = format!("{WRITE_FILE}(use shapes circle)\n{SHAPES_WITH_A_TYPO}(defn z () 1)\n");
    insta::assert_snapshot!(RedefinitionSession::transcript("repl_module_typo", &input));
}

#[test]
fn repl_helps_an_expression_whose_error_lies_in_a_changed_module() {
    let input = format!("{WRITE_FILE}(use shapes circle)\n{SHAPES_WITH_A_TYPO}(circle 2)\n");
    insta::assert_snapshot!(RedefinitionSession::transcript(
        "repl_expression_module_typo",
        &input
    ));
}

struct EditedSession {
    sandbox: Sandbox,
}

impl EditedSession {
    const INPUT_LINE: &str = "(f :x 1 \"s\") ; c\r";

    fn new(name: &str) -> Self {
        Self {
            sandbox: Sandbox::new(name),
        }
    }

    fn typing(&self, typed: &[Keys], environment: &[(&str, &str)]) -> Outcome {
        let environment: Vec<(&str, &str)> = [("TERM", "xterm")]
            .into_iter()
            .chain(environment.iter().copied())
            .collect();
        self.sandbox
            .compiler_on_a_terminal_reading(typed, &environment, &["repl"])
    }

    fn session(&self, typed: &[Keys], environment: &[(&str, &str)]) -> String {
        self.typing(typed, environment).assert_exit_code(0).stdout()
    }

    fn with_tool(self, name: &str, script: &str) -> Self {
        Self {
            sandbox: self
                .sandbox
                .with_executable(&format!("tools/{name}"), script),
        }
    }

    fn path_with_tools(&self) -> String {
        format!(
            "{}:{}",
            self.sandbox.directory.join("tools").display(),
            std::env::var("PATH").expect("the tests run with a PATH")
        )
    }

    fn history(&self) -> String {
        self.sandbox
            .written_file(&Self::history_path(Sandbox::STATE_DIRECTORY))
    }

    fn history_path(state: &str) -> String {
        format!("{state}/{}/history", Language::NAME)
    }

    fn styled(style: anstyle::Style, text: &str) -> String {
        format!("{style}{text}{style:#}")
    }

    fn printed_number(number: &str) -> String {
        let painted = Self::styled(anstyle::AnsiColor::Yellow.on_default(), number);
        format!("\r\n{painted}\r\n")
    }

    fn history_notes(session: &str) -> usize {
        session.matches("history is off").count()
    }
}

#[test]
fn repl_edits_a_line_on_a_terminal() {
    let edited = EditedSession::new("repl_edited_line");
    let typed = format!("(+ 2){} 5\r", Keys::LEFT);
    let session = edited.session(
        &[Keys::AtPrompt(&typed), Keys::AtPrompt(Keys::END_OF_INPUT)],
        &[("NO_COLOR", "1")],
    );
    assert!(session.contains("(+ 2 5)"), "{session:?}");
    assert!(session.contains("\r\n7\r\n"), "{session:?}");
}

#[test]
fn repl_indents_a_typed_line_inside_an_unclosed_list() {
    let edited = EditedSession::new("repl_typed_continuation");
    let session = edited.session(
        &[
            Keys::AtPrompt("(defn twice (x)\r(* 2 x))\r"),
            Keys::AtPrompt("(twice 4)\r"),
            Keys::AtPromptAfter("\r\n8\r\n", Keys::END_OF_INPUT),
        ],
        &[("NO_COLOR", "1")],
    );
    let screen = RenderedScreen::of(&session);
    assert!(
        screen.contains("> (defn twice (x)\n  (* 2 x))\n"),
        "{screen}"
    );
}

#[test]
fn repl_adds_no_spaces_inside_an_unclosed_string() {
    let edited = EditedSession::new("repl_string_continuation");
    let session = edited.session(
        &[
            Keys::AtPrompt("(println \"a\rb\")\r"),
            Keys::AtPromptAfter("\r\na\r\nb\r\n", Keys::END_OF_INPUT),
        ],
        &[("NO_COLOR", "1")],
    );
    let screen = RenderedScreen::of(&session);
    assert!(screen.contains("> (println \"a\nb\")\na\nb\n"), "{screen}");
}

#[test]
fn repl_reads_plain_lines_on_an_unsupported_terminal() {
    let edited = EditedSession::new("repl_dumb_terminal");
    let session = edited.session(
        &[
            Keys::AfterOutput("> ", "(+ 1 2)\n"),
            Keys::AfterOutput("\r\n3\r\n> ", "(+ 1\n"),
            Keys::AfterOutput("  ", Keys::END_OF_INPUT),
        ],
        &[("TERM", "dumb")],
    );
    assert!(session.contains("error[L0101]"), "{session:?}");
    assert!(
        !session.contains(TerminalScreen::PROMPT_READY),
        "{session:?}"
    );
}

#[test]
fn repl_shows_a_multi_line_paste_once() {
    let edited = EditedSession::new("repl_paste");
    let paste = format!(
        "{}(defn twice (x)\n  (* 2 x))\n(twice 4)\n{}\r",
        Keys::PASTE_START,
        Keys::PASTE_END
    );
    let session = edited.session(
        &[Keys::AtPrompt(&paste), Keys::AtPrompt(Keys::END_OF_INPUT)],
        &[("NO_COLOR", "1")],
    );
    assert!(
        session.contains("> (defn twice (x)\r\n  (* 2 x))\r\n(twice 4)\r\n"),
        "{session:?}"
    );
    assert!(session.contains("\r\n8\r\n"), "{session:?}");
    assert!(!session.contains("  > "), "{session:?}");
}

#[test]
fn repl_keeps_lines_typed_ahead_of_the_prompt() {
    let edited = EditedSession::new("repl_typed_ahead");
    let session = edited.session(
        &[
            Keys::AtPrompt("(+ 1 2)\r(+ 3\r 4)\r"),
            Keys::AtPromptAfter("\r\n7\r\n", Keys::END_OF_INPUT),
        ],
        &[("NO_COLOR", "1")],
    );
    assert!(session.contains("\r\n3\r\n"), "{session:?}");
    assert!(session.contains(" 4)\u{1b}[?2004l\r\n7\r\n"), "{session:?}");
}

#[test]
fn repl_colors_the_input_as_it_is_typed() {
    let edited = EditedSession::new("repl_input_colors");
    let session = edited
        .typing(
            &[
                Keys::AtPrompt(EditedSession::INPUT_LINE),
                Keys::AtPrompt(Keys::END_OF_INPUT),
            ],
            &[],
        )
        .stdout();
    for (color, text) in [
        (anstyle::AnsiColor::Cyan.on_default(), ":x"),
        (anstyle::AnsiColor::Yellow.on_default(), "1"),
        (anstyle::AnsiColor::Green.on_default(), "\"s\""),
        (anstyle::Style::new().dimmed(), "; c"),
    ] {
        let styled = EditedSession::styled(color, text);
        assert!(session.contains(&styled), "{styled:?} in {session:?}");
    }
}

#[test]
fn repl_leaves_the_input_plain_when_color_is_off() {
    let edited = EditedSession::new("repl_input_plain");
    let session = edited
        .typing(
            &[
                Keys::AtPrompt(EditedSession::INPUT_LINE),
                Keys::AtPrompt(Keys::END_OF_INPUT),
            ],
            &[("NO_COLOR", "1")],
        )
        .stdout();
    assert!(session.contains("(f :x 1 \"s\") ; c"), "{session:?}");
    assert!(!PromptColors::is_styled(&session), "{session:?}");
}

#[test]
fn repl_quits_at_an_empty_prompt_on_end_of_input() {
    let edited = EditedSession::new("repl_end_of_input");
    let quit = edited.typing(&[Keys::AtPrompt(Keys::END_OF_INPUT)], &[]);
    assert_eq!(quit.assert_exit_code(0).stderr(), "");
}

#[test]
fn repl_drops_the_typed_input_on_interrupt() {
    let edited = EditedSession::new("repl_interrupt_typing");
    let interrupted = format!("(+ 1{}", Keys::INTERRUPT);
    let session = edited.session(
        &[
            Keys::AtPrompt(&interrupted),
            Keys::AtPrompt("(+ 2 3)\r"),
            Keys::AtPrompt(Keys::END_OF_INPUT),
        ],
        &[],
    );
    assert!(
        session.contains(&EditedSession::printed_number("5")),
        "{session:?}"
    );
    assert_eq!(edited.history(), "#V2\n(+ 2 3)\n");
}

#[test]
fn repl_reports_unreadable_input_on_a_terminal() {
    let edited = EditedSession::new("repl_unreadable_terminal");
    let session = edited
        .typing(&[Keys::BytesAtPrompt(b"\xff\r")], &[("NO_COLOR", "1")])
        .assert_exit_code(2)
        .stdout();
    assert!(
        session.contains("error: cannot read standard input: invalid data"),
        "{session:?}"
    );
}

#[test]
fn repl_keeps_history_under_the_state_home() {
    let edited = EditedSession::new("repl_history");
    edited.session(
        &[
            Keys::AtPrompt("(+ 1 2)\r"),
            Keys::AtPrompt(Keys::END_OF_INPUT),
        ],
        &[],
    );
    let recalled = edited.session(
        &[
            Keys::AtPrompt("\u{1b}[A\r"),
            Keys::AtPrompt(Keys::END_OF_INPUT),
        ],
        &[],
    );
    assert!(
        recalled.contains(&EditedSession::printed_number("3")),
        "{recalled:?}"
    );
    assert_eq!(edited.history(), "#V2\n(+ 1 2)\n");
}

#[test]
fn repl_keeps_history_under_the_home_directory() {
    let edited = EditedSession::new("repl_history_home");
    let home = edited.sandbox.directory.join("home");
    let home = home.to_str().expect("the sandbox path is UTF-8");
    edited.session(
        &[
            Keys::AtPrompt("(+ 1 2)\r"),
            Keys::AtPrompt(Keys::END_OF_INPUT),
        ],
        &[("XDG_STATE_HOME", ""), ("HOME", home)],
    );
    let path = EditedSession::history_path("home/.local/state");
    assert_eq!(edited.sandbox.written_file(&path), "#V2\n(+ 1 2)\n");
}

#[test]
fn repl_works_without_history_when_no_state_directory_is_known() {
    let edited = EditedSession::new("repl_history_unknown");
    let session = edited.session(
        &[
            Keys::AtPrompt("(+ 1 2)\r"),
            Keys::AtPrompt(Keys::END_OF_INPUT),
        ],
        &[("XDG_STATE_HOME", ""), ("HOME", "")],
    );
    assert!(
        session.contains("history is off: no state directory"),
        "{session:?}"
    );
    assert!(
        session.contains(&EditedSession::printed_number("3")),
        "{session:?}"
    );
}

#[test]
fn repl_works_without_history_when_the_state_directory_cannot_be_made() {
    let edited = EditedSession::new("repl_history_blocked");
    fs::write(edited.sandbox.directory.join("blocked"), "").expect("the sandbox accepts files");
    let blocked = edited.sandbox.directory.join("blocked");
    let blocked = blocked.to_str().expect("the sandbox path is UTF-8");
    let session = edited.session(
        &[
            Keys::AtPrompt("(+ 1 2)\r"),
            Keys::AtPrompt(Keys::END_OF_INPUT),
        ],
        &[("XDG_STATE_HOME", blocked)],
    );
    assert!(
        session.contains("history is off: cannot write"),
        "{session:?}"
    );
    assert_eq!(EditedSession::history_notes(&session), 1, "{session:?}");
}

#[test]
fn repl_works_without_history_when_the_history_cannot_be_read() {
    let edited = EditedSession::new("repl_history_unreadable");
    let path = EditedSession::history_path(Sandbox::STATE_DIRECTORY);
    fs::create_dir_all(edited.sandbox.directory.join(&path))
        .expect("the sandbox accepts directories");
    let session = edited.session(
        &[
            Keys::AtPrompt("(+ 1 2)\r"),
            Keys::AtPrompt(Keys::END_OF_INPUT),
        ],
        &[],
    );
    assert!(
        session.contains("history is off: cannot read"),
        "{session:?}"
    );
    assert!(
        session.contains(&EditedSession::printed_number("3")),
        "{session:?}"
    );
}

#[test]
fn repl_says_once_that_history_cannot_be_written() {
    let edited = EditedSession::new("repl_history_read_only");
    let path = EditedSession::history_path(Sandbox::STATE_DIRECTORY);
    let file = edited.sandbox.directory.join(&path);
    fs::create_dir_all(file.parent().expect("the history has a directory"))
        .expect("the sandbox accepts directories");
    fs::write(&file, "#V2\n").expect("the sandbox accepts files");
    fs::set_permissions(&file, fs::Permissions::from_mode(Sandbox::READ_ONLY))
        .expect("the sandbox sets permissions");
    let session = edited.session(
        &[
            Keys::AtPrompt("(+ 1 2)\r"),
            Keys::AtPrompt("(+ 3 4)\r"),
            Keys::AtPrompt(Keys::END_OF_INPUT),
        ],
        &[],
    );
    assert!(
        session.contains("history is off: cannot write"),
        "{session:?}"
    );
    assert_eq!(EditedSession::history_notes(&session), 1, "{session:?}");
    assert!(
        session.contains(&EditedSession::printed_number("7")),
        "{session:?}"
    );
}

#[test]
fn build_stops_quietly_when_rustc_is_interrupted() {
    let sandbox = Sandbox::new("build_rustc_interrupted")
        .with_file("greeting.knv", GREETING)
        .with_executable("tools/rustc", INTERRUPTED_RUSTC)
        .with_executable("tools/rustfmt", PASSTHROUGH_RUSTFMT);
    let build = sandbox.compiler_with_path("tools", &["build", "greeting.knv"]);
    assert_eq!(build.assert_exit_code(130).stderr(), "");
}

#[test]
fn repl_interrupts_a_running_program_and_keeps_the_definitions() {
    let edited = EditedSession::new("repl_interrupt_program");
    let session = edited.session(
        &[
            Keys::AtPrompt("(defn spin (i) (if (< i 0) i (recur (+ i 1))))\r"),
            Keys::AtPrompt("(do (println \"spinning\") (spin 0)) (+ 40 2)\r"),
            Keys::AfterOutput("spinning\r\n", Keys::INTERRUPT),
            Keys::AtPromptAfter("interrupted\r\n", "(spin -2)\r"),
            Keys::AtPromptAfter("\r\n-2\r\n", Keys::END_OF_INPUT),
        ],
        &[("NO_COLOR", "1")],
    );
    assert!(!session.contains("exited with code"), "{session:?}");
    assert!(!session.contains("\r\n42\r\n"), "{session:?}");
    assert_eq!(session.matches("interrupted").count(), 1, "{session:?}");
}

#[test]
fn repl_interrupts_rustc_without_an_error() {
    let edited = EditedSession::new("repl_interrupt_rustc").with_tool("rustc", WAITING_RUSTC);
    let session = edited.session(
        &[
            Keys::AtPrompt("(+ 1 2)\r"),
            Keys::AfterOutput("compiling\r\n", Keys::INTERRUPT),
            Keys::AtPromptAfter("interrupted\r\n", Keys::END_OF_INPUT),
        ],
        &[("NO_COLOR", "1"), ("PATH", &edited.path_with_tools())],
    );
    assert!(!session.contains("error"), "{session:?}");
}

#[test]
fn repl_interrupts_a_rustc_that_catches_the_signal() {
    let edited = EditedSession::new("repl_interrupt_catching_rustc")
        .with_tool("rustc", SIGNAL_CATCHING_RUSTC);
    let session = edited.session(
        &[
            Keys::AtPrompt("(+ 1 2)\r"),
            Keys::AfterOutput("compiling\r\n", Keys::INTERRUPT),
            Keys::AtPromptAfter("interrupted\r\n", Keys::END_OF_INPUT),
        ],
        &[("NO_COLOR", "1"), ("PATH", &edited.path_with_tools())],
    );
    assert!(!session.contains("error"), "{session:?}");
}

#[test]
fn repl_reports_a_rustc_failure_on_a_terminal() {
    let edited = EditedSession::new("repl_rustc_failure_terminal")
        .with_tool("rustc", CRASHING_RUSTC)
        .with_tool("rustfmt", FAILING_RUSTFMT);
    let session = edited.session(
        &[
            Keys::AtPrompt("(+ 1 2)\r"),
            Keys::AtPromptAfter("rustc crashed", Keys::END_OF_INPUT),
        ],
        &[("NO_COLOR", "1"), ("PATH", &edited.path_with_tools())],
    );
    assert!(session.contains("note: rustfmt failed"), "{session:?}");
    assert!(!session.contains("interrupted"), "{session:?}");
}

#[test]
fn repl_interrupts_a_rustfmt_that_catches_the_signal() {
    let edited = EditedSession::new("repl_interrupt_catching_rustfmt")
        .with_tool("rustfmt", SIGNAL_CATCHING_RUSTFMT);
    let session = edited.session(
        &[
            Keys::AtPrompt("(+ 40 2)\r"),
            Keys::AfterOutput("formatting\r\n", Keys::INTERRUPT),
            Keys::AtPromptAfter("interrupted\r\n", Keys::END_OF_INPUT),
        ],
        &[("NO_COLOR", "1"), ("PATH", &edited.path_with_tools())],
    );
    assert!(!session.contains("note"), "{session:?}");
    assert!(!session.contains("\r\n42\r\n"), "{session:?}");
}

#[test]
fn repl_interrupts_a_running_program_on_an_unsupported_terminal() {
    let edited = EditedSession::new("repl_interrupt_dumb_terminal");
    let session = edited.session(
        &[
            Keys::AfterOutput("> ", "(defn spin (i) (if (< i 0) i (recur (+ i 1))))\n"),
            Keys::AfterOutput("> ", "(do (println \"spinning\") (spin 0))\n"),
            Keys::AfterOutput("spinning\r\n", Keys::INTERRUPT),
            Keys::AfterOutput("interrupted\r\n> ", "(spin -2)\n"),
            Keys::AfterOutput("\r\n-2\r\n> ", Keys::END_OF_INPUT),
        ],
        &[("TERM", "dumb")],
    );
    assert!(!session.contains("exited with code"), "{session:?}");
}

#[test]
fn emit_stops_quietly_when_rustfmt_is_interrupted() {
    let sandbox = Sandbox::new("emit_rustfmt_interrupted")
        .with_file("greeting.knv", GREETING)
        .with_executable("tools/rustfmt", INTERRUPTED_RUSTFMT);
    let emit = sandbox.compiler_with_path("tools", &["emit", "greeting.knv"]);
    assert_eq!(emit.assert_exit_code(130).stderr(), "");
}

#[test]
fn emit_stops_quietly_when_rustfmt_is_interrupted_before_it_reads_its_input() {
    let sandbox = Sandbox::new("emit_rustfmt_interrupted_before_reading")
        .with_file("greeting.knv", &Sandbox::pipe_overflowing_greeting())
        .with_executable("tools/rustfmt", INTERRUPTED_RUSTFMT);
    let emit = sandbox.compiler_with_path("tools", &["emit", "greeting.knv"]);
    assert_eq!(emit.assert_exit_code(130).stderr(), "");
}

#[test]
fn emit_notes_a_rustfmt_that_fails_before_it_reads_its_input() {
    let sandbox = Sandbox::new("emit_rustfmt_failed_before_reading")
        .with_file("greeting.knv", &Sandbox::pipe_overflowing_greeting())
        .with_executable("tools/rustfmt", FAILING_RUSTFMT);
    let emit = sandbox.compiler_with_path("tools", &["emit", "greeting.knv"]);
    assert!(
        emit.assert_exit_code(0).stderr().contains("Broken pipe"),
        "{}",
        emit.stderr()
    );
}

#[test]
fn repl_interrupts_rustfmt_and_skips_the_input() {
    let edited = EditedSession::new("repl_interrupt_rustfmt").with_tool("rustfmt", WAITING_RUSTFMT);
    let session = edited.session(
        &[
            Keys::AtPrompt("(+ 40 2)\r"),
            Keys::AfterOutput("formatting\r\n", Keys::INTERRUPT),
            Keys::AtPromptAfter("interrupted\r\n", Keys::END_OF_INPUT),
        ],
        &[("NO_COLOR", "1"), ("PATH", &edited.path_with_tools())],
    );
    assert!(!session.contains("note"), "{session:?}");
    assert!(!session.contains("\r\n42\r\n"), "{session:?}");
}

#[test]
fn repl_skips_rustc_after_an_interrupt_in_the_front_end() {
    let edited =
        EditedSession::new("repl_interrupt_front_end").with_tool("rustfmt", UNINTERRUPTED_RUSTFMT);
    let session = edited.session(
        &[
            Keys::AtPrompt("(+ 40 2)\r"),
            Keys::AfterOutput("formatting\r\n", Keys::INTERRUPT),
            Keys::AtPromptAfter("interrupted\r\n", "(+ 1 2)\r"),
            Keys::AtPromptAfter("\r\n3\r\n", Keys::END_OF_INPUT),
        ],
        &[("NO_COLOR", "1"), ("PATH", &edited.path_with_tools())],
    );
    assert!(!session.contains("\r\n42\r\n"), "{session:?}");
}

#[test]
fn repl_on_piped_input_stops_at_an_interrupt() {
    let sandbox = Sandbox::new("repl_piped_interrupt");
    let mut child = sandbox
        .command(Path::new(Sandbox::COMPILER), &["repl"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("the compiler starts");
    let mut input = child.stdin.take().expect("stdin is piped");
    std::io::Write::write_all(&mut input, b"(+ 1 2)\n").expect("the compiler reads its input");
    let mut first_value = String::new();
    BufReader::new(child.stdout.take().expect("stdout is piped"))
        .read_line(&mut first_value)
        .expect("the compiler prints the value");
    assert_eq!(first_value, "3\n");
    let interrupt = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .expect("kill runs");
    assert!(interrupt.success());
    let deadline = Instant::now() + Duration::from_secs(30);
    while child
        .try_wait()
        .expect("the compiler is waitable")
        .is_none()
    {
        if Instant::now() > deadline {
            child.kill().ok();
            panic!("the piped prompt kept running after an interrupt");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    drop(input);
    let status = child.wait().expect("the compiler finishes");
    assert_eq!(status.signal(), Some(2));
}

const UNIMPORTED_TESTS: &str = r#"(defn pad (text) (str " " text))

(deftest pads-a-letter (assert (= (pad "a") " a")))

(deftest pads-nothing (assert (= (pad "") "") "the empty string grows"))
"#;
const PADDING_TESTS: &str = r#"(use prelude/test assert deftest)

(defn pad (text) (str " " text))

(deftest pads-a-letter (assert (= (pad "a") " a")))

(deftest pads-nothing
  "Pads the empty string to one space."
  (assert (= (pad "") "") "the empty string grows"))

(deftest unwraps-nothing (unwrap (the (option i64) none)))

(deftest counts-letters (assert (> (len (pad "")) 2)))
"#;

const CRASHING_TESTS: &str = r#"(use prelude/test assert deftest)

(def abort (rust "std::process::abort()" (fn () unit)))

(deftest runs-first (assert true))

(deftest crashes (exit 3))

(deftest exits-cleanly (exit 0))

(deftest aborts (abort))

(deftest runs-last (assert true))
"#;

const FAILING_DOC_EXAMPLE: &str = r#"(defn twice (x)
  "Doubles a number.

  (twice 3)
  => 7"
  (* 2 x))
"#;

#[test]
fn test_reports_a_failing_docstring_example_at_its_line() {
    let sandbox = Sandbox::new("test_doc_example").with_file("twice.knv", FAILING_DOC_EXAMPLE);
    let test = sandbox.compiler(&["test", "twice.knv"]);
    insta::assert_snapshot!(test.assert_exit_code(1).transcript());
}

#[test]
fn test_selects_a_docstring_example_by_its_full_name() {
    let sandbox = Sandbox::new("test_doc_filter").with_file("twice.knv", FAILING_DOC_EXAMPLE);
    let test = sandbox.compiler(&["test", "twice.knv", "--exact", "twice/doc:twice"]);
    insta::assert_snapshot!(test.assert_exit_code(1).transcript());
}

#[test]
fn test_filter_runs_only_docstring_examples() {
    let source = format!(
        "(use prelude/test assert deftest)\n\n(deftest plain (assert true))\n\n{FAILING_DOC_EXAMPLE}"
    );
    let sandbox = Sandbox::new("test_doc_only").with_file("twice.knv", &source);
    let test = sandbox.compiler(&["test", "twice.knv", "--filter", "doc:"]);
    insta::assert_snapshot!(test.assert_exit_code(1).transcript());
}

#[test]
fn test_runs_a_docstring_example_with_crlf_line_endings() {
    let sandbox = Sandbox::new("test_doc_crlf").with_file(
        "twice.knv",
        "(defn twice (x)\r\n  \"Doubles.\r\n\r\n  (twice 3)\r\n  => several forms ...\r\n  (twice 1)\r\n  => 2\"\r\n  (* 2 x))\r\n",
    );
    sandbox.compiler(&["test", "twice.knv"]).assert_exit_code(0);
}

#[test]
fn test_accepts_a_thousand_docstrings_without_a_docstring_example() {
    let source: String = (0..1100)
        .map(|number| format!("(defn term-{number} (x) \"Documented.\" x)\n"))
        .collect();
    let sandbox = Sandbox::new("test_doc_many").with_file("many.knv", &source);
    sandbox.compiler(&["test", "many.knv"]).assert_exit_code(0);
}

#[test]
fn test_runs_two_hundred_docstring_examples_in_one_paragraph() {
    let examples: String = (0..200).map(|_| "  (twice 1)\n  => 2\n").collect();
    let source = format!("(defn twice (x)\n  \"Doubles.\n\n{examples}\"\n  (* 2 x))\n");
    let sandbox = Sandbox::new("test_doc_paragraph").with_file("twice.knv", &source);
    sandbox.compiler(&["test", "twice.knv"]).assert_exit_code(0);
}

#[test]
fn check_leaves_a_docstring_example_unexpanded() {
    let sandbox = Sandbox::new("check_doc_example").with_file(
        "twice.knv",
        "(defn twice (x)\n  \"Doubles.\n\n  (twic 3)\n  => 6\"\n  (* 2 x))\n",
    );
    sandbox
        .compiler(&["check", "twice.knv"])
        .assert_exit_code(0);
}

#[test]
fn test_reports_passing_failing_and_panicking_tests() {
    let sandbox = Sandbox::new("test_report").with_file("padding.knv", PADDING_TESTS);
    let test = sandbox.compiler(&["test", "padding.knv"]);
    insta::assert_snapshot!(test.assert_exit_code(1).transcript());
}

#[test]
fn test_exits_zero_when_every_selected_test_passes() {
    let sandbox = Sandbox::new("test_pass").with_file("padding.knv", PADDING_TESTS);
    let test = sandbox.compiler(&["test", "padding.knv", "--filter", "pads-a"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "running 1 test\ntest padding/pads-a-letter ... ok\n1 passed, 0 failed\n"
    );
}

#[test]
fn test_runs_tests_that_use_no_module() {
    let sandbox = Sandbox::new("test_without_use").with_file("padding.knv", UNIMPORTED_TESTS);
    let test = sandbox.compiler(&["test", "padding.knv"]);
    insta::assert_snapshot!(test.assert_exit_code(1).stdout());
}

#[test]
fn test_runs_the_tests_whose_full_name_contains_the_filter() {
    let sandbox = Sandbox::new("test_filter").with_file("padding.knv", PADDING_TESTS);
    let test = sandbox.compiler(&["test", "padding.knv", "--filter", "padding/pads"]);
    insta::assert_snapshot!(test.assert_exit_code(1).stdout());
}

#[test]
fn test_with_exact_runs_only_the_test_of_that_full_name() {
    let sandbox = Sandbox::new("test_exact").with_file("padding.knv", PADDING_TESTS);
    let test = sandbox.compiler(&["test", "padding.knv", "--exact", "padding/pads-a-letter"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "running 1 test\ntest padding/pads-a-letter ... ok\n1 passed, 0 failed\n"
    );
}

#[test]
fn test_with_exact_runs_no_test_whose_name_only_contains_it() {
    let sandbox = Sandbox::new("test_exact_part").with_file("padding.knv", PADDING_TESTS);
    let test = sandbox.compiler(&["test", "padding.knv", "--exact", "padding/pads"]);
    test.assert_exit_code(0);
    assert_eq!(test.stdout(), "running 0 tests\n0 passed, 0 failed\n");
    assert_eq!(test.stderr(), "note: no test is named `padding/pads`\n");
}

#[test]
fn test_notes_a_filter_that_matches_no_test() {
    let sandbox = Sandbox::new("test_no_match").with_file("padding.knv", PADDING_TESTS);
    let test = sandbox.compiler(&["test", "padding.knv", "--filter", "trims"]);
    test.assert_exit_code(0);
    assert_eq!(test.stdout(), "running 0 tests\n0 passed, 0 failed\n");
    assert_eq!(test.stderr(), "note: no test name contains `trims`\n");
}

#[test]
fn test_of_a_program_without_tests_runs_none() {
    let sandbox = Sandbox::new("test_none").with_file("plain.knv", "(defn twice (x) (* 2 x))\n");
    let test = sandbox.compiler(&["test", "plain.knv"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "running 0 tests\n0 passed, 0 failed\n"
    );
}

#[test]
fn test_points_a_bare_filter_to_the_filter_flag() {
    let sandbox = Sandbox::new("test_bare_filter").with_file("padding.knv", PADDING_TESTS);
    let test = sandbox.compiler(&["test", "padding.knv", "pads"]);
    insta::assert_snapshot!(test.assert_exit_code(2).stderr());
}

#[test]
fn test_rejects_both_a_filter_and_exact() {
    let sandbox = Sandbox::new("test_filter_and_exact").with_file("padding.knv", PADDING_TESTS);
    sandbox
        .compiler(&[
            "test",
            "padding.knv",
            "--filter",
            "pads",
            "--exact",
            "padding/pads-nothing",
        ])
        .assert_exit_code(2);
}

#[test]
fn test_reports_a_crash_and_counts_the_tests_not_run() {
    let sandbox = Sandbox::new("test_crash").with_file("crash.knv", CRASHING_TESTS);
    let test = sandbox.compiler(&["test", "crash.knv"]);
    insta::assert_snapshot!(test.assert_exit_code(1).transcript());
}

#[test]
fn test_reports_a_test_that_exits_with_zero_as_a_crash() {
    let sandbox = Sandbox::new("test_exit_zero").with_file("crash.knv", CRASHING_TESTS);
    let test = sandbox.compiler(&["test", "crash.knv", "--filter", "exits"]);
    assert_eq!(
        test.assert_exit_code(1).stdout(),
        "running 1 test\n\
         test crash/exits-cleanly ... CRASH\n\
         CRASH crash/exits-cleanly: the program exited with code 0\n\
         0 passed, 0 failed, 1 crashed, 0 not run\n"
    );
}

#[test]
fn test_reports_an_abort_by_its_signal() {
    let sandbox = Sandbox::new("test_abort").with_file("crash.knv", CRASHING_TESTS);
    let test = sandbox.compiler(&["test", "crash.knv", "--filter", "aborts"]);
    assert_eq!(
        test.assert_exit_code(1).stdout(),
        "running 1 test\n\
         test crash/aborts ... CRASH\n\
         CRASH crash/aborts: the program stopped on signal 6 (SIGABRT)\n\
         0 passed, 0 failed, 1 crashed, 0 not run\n"
    );
}

#[test]
fn test_puts_the_result_after_the_output_of_the_test() {
    let sandbox = Sandbox::new("test_output").with_file(
        "talking.knv",
        "(use prelude/test deftest)\n\n(deftest prints-a-line (println \"hello\"))\n\n\
         (deftest prints-a-word (print \"hello\"))\n\n\
         (deftest reads (prn (read-line)))\n",
    );
    let test = sandbox.compiler_reading("line one\n", &["test", "talking.knv"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "running 3 tests\n\
         test talking/prints-a-line ... hello\nok\n\
         test talking/prints-a-word ... hello\nok\n\
         test talking/reads ... (ok (some \"line one\"))\nok\n\
         3 passed, 0 failed\n"
    );
}

#[test]
fn test_checks_the_entry_main_without_running_it() {
    let sandbox = Sandbox::new("test_main").with_file(
        "app.knv",
        "(use prelude/test deftest)\n\n(defn main () (println \"main ran\"))\n\n(deftest calls-main (main))\n",
    );
    let test = sandbox.compiler(&["test", "app.knv"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "running 1 test\ntest app/calls-main ... main ran\nok\n1 passed, 0 failed\n"
    );
}

#[test]
fn test_runs_the_tests_of_every_module_of_a_directory() {
    let sandbox = Sandbox::new("test_directory")
        .with_file("checks/text.knv", "(use prelude/test assert deftest)\n(use shelf/stock restock)\n\n(deftest restocks (assert (= (restock 1) 2)))\n")
        .with_file("checks/notes.txt", "(deftest ignored (assert false))\n")
        .with_file("checks/shelf/stock.knv", "(use prelude/test assert deftest)\n\n(export restock)\n\n(defn restock (count) (+ count 1))\n\n(deftest keeps-count (assert (= (restock 0) 1)))\n");
    let test = sandbox.compiler(&["test", "checks"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "running 2 tests\n\
         test shelf/stock/keeps-count ... ok\n\
         test text/restocks ... ok\n\
         2 passed, 0 failed\n"
    );
}

#[test]
fn test_skips_hidden_directories_and_the_target_directory_of_its_walk() {
    let sandbox = Sandbox::new("test_skipped_directories")
        .with_file("checks/stock.knv", PROJECT_TESTS)
        .with_file("checks/.direnv/stock.knv", UNCLOSED)
        .with_file("checks/target/stock.knv", UNCLOSED)
        .with_file("checks/shelf/target/stock.knv", PROJECT_TESTS);
    let test = sandbox.compiler(&["test", "checks"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "running 2 tests\n\
         test shelf/target/stock/adds ... ok\n\
         test stock/adds ... ok\n\
         2 passed, 0 failed\n"
    );
}

#[test]
fn test_in_a_project_walks_a_target_directory_of_its_test_directory() {
    let sandbox = Sandbox::new("project_test_inner_target")
        .with_file(&Language::project_file_name(), PROJECT)
        .with_file("tests/target/stock.knv", PROJECT_TESTS);
    let test = sandbox.compiler(&["test", "tests"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "running 1 test\ntest target/stock/adds ... ok\n1 passed, 0 failed\n"
    );
}

#[test]
fn test_in_a_project_skips_only_the_project_build_directory() {
    let sandbox = Sandbox::new("project_test_skipped_directories")
        .with_file(&Language::project_file_name(), PROJECT)
        .with_file("tests/stock.knv", PROJECT_TESTS)
        .with_file("tests/target/stock.knv", PROJECT_TESTS)
        .with_file("target/stock.knv", UNCLOSED)
        .with_file(".direnv/stock.knv", UNCLOSED);
    let test = sandbox.compiler(&["test", "."]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "running 2 tests\n\
         test tests/stock/adds ... ok\n\
         test tests/target/stock/adds ... ok\n\
         2 passed, 0 failed\n"
    );
}

#[test]
fn test_lists_the_exports_that_no_test_writes() {
    let sandbox = Sandbox::new("test_untested")
        .with_file("app.knv", "(use prelude/test assert deftest)\n(use shelf restock)\n\n(deftest restocks (assert (= (restock 1) 2)))\n")
        .with_file("shelf.knv", "(export restock shelve stack)\n\n(defn restock (count) (+ count 1))\n\n(defn shelve (count) (- count 1))\n\n(defn stack (count) (* count 2))\n");
    let test = sandbox.compiler(&["test", "app.knv", "--untested", "shelf"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "untested in shelf (2 of 3): shelve stack\n\
         running 1 test\n\
         test app/restocks ... ok\n\
         1 passed, 0 failed\n"
    );
}

#[test]
fn test_rejects_untested_for_a_module_the_program_does_not_load() {
    let sandbox = Sandbox::new("test_untested_unloaded").with_file("padding.knv", PADDING_TESTS);
    let test = sandbox.compiler(&["test", "padding.knv", "--untested", "shelf"]);
    test.assert_exit_code(2);
    assert_eq!(test.stdout(), "");
    assert_eq!(
        test.stderr(),
        "error: the program does not load module `shelf`\n"
    );
}

#[test]
fn test_of_a_program_with_errors_reports_them_and_runs_nothing() {
    let sandbox = Sandbox::new("test_errors").with_file(
        "broken.knv",
        "(use prelude/test assert deftest)\n\n(deftest pads (assert (= (missing 1) 2)))\n",
    );
    let test = sandbox.compiler(&["test", "broken.knv"]);
    test.assert_exit_code(1);
    assert_eq!(test.stdout(), "");
    assert!(test.stderr().contains("error[L0201]"), "{}", test.stderr());
}

#[test]
fn test_of_a_missing_path_is_a_usage_error() {
    let sandbox = Sandbox::new("test_missing_path");
    sandbox
        .compiler(&["test", "missing.knv"])
        .assert_exit_code(2);
}

#[test]
fn test_runs_the_prelude_tests_and_lists_the_untested_prelude_names() {
    let mut command = Command::new(Sandbox::COMPILER);
    command
        .args(["test", "prelude/tests", "--untested", "prelude"])
        .current_dir(env!("CARGO_MANIFEST_DIR"));
    Sandbox::isolated(&mut command, &Sandbox::unsandboxed_cache());
    let test = Outcome::of(&mut command);
    insta::assert_snapshot!(test.assert_silent_success().stdout());
}

#[test]
fn test_ends_quietly_when_its_reader_closes() {
    let sandbox = Sandbox::new("test_closed_pipe").with_file("padding.knv", PADDING_TESTS);
    sandbox
        .compiler_writing_to_a_closed_pipe(&["test", "padding.knv"])
        .assert_silent_success();
}

#[test]
#[cfg(target_os = "linux")]
fn test_reports_a_standard_output_it_cannot_write() {
    let sandbox = Sandbox::new("test_unwritable").with_file("padding.knv", PADDING_TESTS);
    let test = sandbox.compiler_writing_to_a_full_device(&["test", "padding.knv"]);
    assert!(
        test.assert_exit_code(2)
            .stderr()
            .starts_with("error: cannot write standard output: "),
        "{}",
        test.stderr()
    );
}

const ASSERTING_PROGRAM: &str = "(use prelude/test assert deftest)\n\n\
(defn positive (number)\n  (assert (> number 0) \"needs a positive number\")\n  number)\n\n\
(deftest keeps-a-positive-number (assert (= (positive 2) 2)))\n\n\
(defn main ()\n  (assert (= (positive 2) 2))\n  (println \"passed\"))\n";

const FAILING_ASSERT: &str = "(use prelude/test assert)\n\n(defn main ()\n  (assert (= (str \"a\") \"b\") \"they differ\"))\n";

const FAILED_ASSERT_TEXT: &str = "assertion failed at failing.knv:4:3: \
(assert (= (str \"a\") \"b\") \"they differ\")\nleft:  \"a\"\nright: \"b\"\nthey differ\n";

#[test]
fn assert_in_a_helper_shared_with_a_test_builds_and_runs() {
    let sandbox = Sandbox::new("assert_shared").with_file("shared.knv", ASSERTING_PROGRAM);
    sandbox
        .compiler(&["build", "shared.knv"])
        .assert_silent_success();
    let run = sandbox.compiler(&["run", "shared.knv"]);
    assert_eq!(run.assert_silent_success().stdout(), "passed\n");
    let test = sandbox.compiler(&["test", "shared.knv"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "running 1 test\ntest shared/keeps-a-positive-number ... ok\n1 passed, 0 failed\n"
    );
}

#[test]
fn a_failed_assert_in_main_prints_its_location_and_both_sides() {
    let sandbox = Sandbox::new("assert_failed_run").with_file("failing.knv", FAILING_ASSERT);
    let run = sandbox.compiler(&["run", "failing.knv"]);
    assert!(
        run.assert_exit_code(101)
            .stderr()
            .contains(FAILED_ASSERT_TEXT),
        "{}",
        run.stderr()
    );
}

#[test]
fn assert_runs_under_eval() {
    let sandbox = Sandbox::new("assert_eval").with_file("shared.knv", ASSERTING_PROGRAM);
    let eval = sandbox.compiler(&["eval", "shared.knv", "(assert (= (positive 2) 2))"]);
    eval.assert_silent_success();
}

#[test]
fn a_failed_assert_under_eval_prints_both_sides() {
    let sandbox = Sandbox::new("assert_eval_failed").with_file("shared.knv", ASSERTING_PROGRAM);
    let eval = sandbox.compiler(&["eval", "shared.knv", "(assert (= (positive 2) 3))"]);
    assert!(
        eval.assert_exit_code(101).stderr().contains(
            "assertion failed at <eval>:1:1: (assert (= (positive 2) 3))\nleft:  2\nright: 3\n"
        ),
        "{}",
        eval.stderr()
    );
}

#[test]
fn assert_runs_at_the_prompt() {
    let sandbox = Sandbox::new("assert_prompt");
    let prompt = sandbox.compiler_reading(
        "(use prelude/test assert)\n(assert (= 1 1))\n(println \"passed\")\n(assert (= 1 2))\n",
        &["repl"],
    );
    assert_eq!(prompt.assert_exit_code(0).stdout(), "passed\n");
    assert!(
        prompt
            .stderr()
            .contains("assertion failed at <input 4>:1:1: (assert (= 1 2))\nleft:  1\nright: 2\n"),
        "{}",
        prompt.stderr()
    );
}

#[test]
fn test_prints_no_names_when_a_test_writes_every_export() {
    let sandbox = Sandbox::new("test_all_written")
        .with_file(
            "app.knv",
            "(use prelude/test assert deftest)\n(use shelf restock)\n\n(deftest restocks (assert (= (restock 1) 2)))\n",
        )
        .with_file(
            "shelf.knv",
            "(export restock)\n\n(defn restock (count) (+ count 1))\n",
        );
    let test = sandbox.compiler(&["test", "app.knv", "--untested", "shelf"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "untested in shelf (0 of 1)\nrunning 1 test\ntest app/restocks ... ok\n1 passed, 0 failed\n"
    );
}

#[test]
fn test_does_not_count_a_binding_of_the_same_name_as_an_export() {
    let sandbox = Sandbox::new("test_same_name")
        .with_file(
            "app.knv",
            "(use prelude/test assert deftest)\n(use shelf shelve)\n\n(defn restock () (shelve 2))\n\n(deftest restocks (assert (= (restock) 1)))\n",
        )
        .with_file(
            "shelf.knv",
            "(export restock shelve)\n\n(defn restock (count) (+ count 1))\n\n(defn shelve (count) (- count 1))\n",
        );
    let test = sandbox.compiler(&["test", "app.knv", "--untested", "shelf"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "untested in shelf (2 of 2): restock shelve\nrunning 1 test\ntest app/restocks ... ok\n1 passed, 0 failed\n"
    );
}

#[test]
fn test_reports_a_missing_build_directory() {
    let sandbox = Sandbox::new("test_no_build_directory").with_file("padding.knv", PADDING_TESTS);
    let test = sandbox.compiler_with_temporary_directory("missing", &["test", "padding.knv"]);
    assert!(
        test.assert_exit_code(2)
            .stderr()
            .starts_with("error: cannot create a build directory"),
        "{}",
        test.stderr()
    );
}

#[test]
fn test_reports_a_program_that_cannot_start() {
    let sandbox = Sandbox::new("test_missing_program")
        .with_file("padding.knv", PADDING_TESTS)
        .with_executable("tools/rustc", SILENT_RUSTC)
        .with_executable("tools/rustfmt", PASSTHROUGH_RUSTFMT);
    let test = sandbox.compiler_with_path("tools", &["test", "padding.knv"]);
    assert!(
        test.assert_exit_code(2)
            .stderr()
            .starts_with("error: cannot run"),
        "{}",
        test.stderr()
    );
}

#[test]
fn test_reports_a_directory_it_cannot_read() {
    let sandbox = Sandbox::new("test_unreadable_directory")
        .with_file("checks/hidden/stock.knv", "(defn restock () 1)\n");
    let hidden = sandbox.directory.join("checks/hidden");
    fs::set_permissions(&hidden, fs::Permissions::from_mode(Sandbox::NO_ACCESS))
        .expect("the sandbox changes permissions");
    let test = sandbox.compiler(&["test", "checks"]);
    fs::set_permissions(&hidden, fs::Permissions::from_mode(Sandbox::EXECUTABLE))
        .expect("the sandbox changes permissions");
    assert!(
        test.assert_exit_code(2)
            .stderr()
            .starts_with("error: cannot read `checks`"),
        "{}",
        test.stderr()
    );
}

#[test]
fn form_location_keeps_the_name_of_a_source_outside_the_root() {
    let sandbox = Sandbox::new("form_location_outside_root").with_file(
        "library/located.knv",
        "(use code form-location)\n\n(def here (macro (form) (form-location form)))\n",
    );
    let eval = sandbox.compiler(&["eval", "library/located.knv", "(println (here))"]);
    assert_eq!(eval.assert_silent_success().stdout(), "<eval>:1:10\n");
}

#[test]
fn test_lists_the_untested_names_of_a_program_with_no_tests() {
    let sandbox = Sandbox::new("test_untested_no_tests")
        .with_file(
            "app.knv",
            "(use shelf restock)\n\n(defn main () (println (restock 1)))\n",
        )
        .with_file(
            "shelf.knv",
            "(export restock)\n\n(defn restock (count) (+ count 1))\n",
        );
    let test = sandbox.compiler(&["test", "app.knv", "--untested", "shelf"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "untested in shelf (1 of 1): restock\nrunning 0 tests\n0 passed, 0 failed\n"
    );
}

const PROJECT: &str = "(project greeter)\n";
const PROJECT_WITH_UNKNOWN_KEY: &str = "(project greeter :entyr \"src/main.knv\")\n";
const UNLAID_PROJECT: &str = "(project browser :version \"0.1.0\" :entry \"src/main.knv\"\n  (libraries (hints :path \"../hints\"))\n\n  (dependencies (cef \"144.0\") (serde \"1.0\" :features (\"derive\"))))\n";
const BARE_NAMES_PROJECT: &str = "(project demo alpha beta gamma delta epsilon zeta eta theta iota kappa lambda)\n\n(libraries alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu)\n\n(dependencies alpha beta gamma delta epsilon zeta eta theta iota kappa lambda)\n\n(dev-dependencies alpha beta gamma delta epsilon zeta eta theta iota kappa lambda)\n";
const MISSING_COMMIT: &str = "0000000000000000000000000000000000000000";
const SELF_IMPORT: &str = "(use sub/c)\n\n(defn f () 1)\n";
const PROJECT_ENTRY: &str =
    "(use text/shout shout)\n\n(defn main ()\n  (println (shout \"hi\")))\n";
const PROJECT_SHOUT: &str =
    "(use text/marks bang)\n\n(export shout)\n\n(defn shout (text) (str text bang))\n";
const PROJECT_MARKS: &str = "(export bang)\n\n(def bang \"!\")\n";
const PROJECT_SCRIPT: &str = "(use helper twice)\n\n(defn main ()\n  (println (twice 2)))\n";
const PROJECT_HELPER: &str = "(export twice)\n\n(defn twice (n) (* 2 n))\n";
const PROJECT_TESTS: &str =
    "(use prelude/test assert deftest)\n\n(deftest adds (assert (= (+ 1 1) 2)))\n";

impl Sandbox {
    fn project_program(name: &str) -> PathBuf {
        Path::new("target")
            .join(Language::NAME)
            .join(name)
            .join(name)
    }

    fn with_project(self, project: &str) -> Self {
        self.with_file(&Language::project_file_name(), project)
            .with_file("src/main.knv", PROJECT_ENTRY)
            .with_file("src/text/shout.knv", PROJECT_SHOUT)
            .with_file("src/text/marks.knv", PROJECT_MARKS)
    }
}

#[test]
fn check_defaults_to_the_project_entry() {
    let sandbox = Sandbox::new("project_check").with_project(PROJECT);
    sandbox.compiler(&["check"]).assert_silent_success();
}

#[test]
fn run_finds_the_project_from_a_subdirectory() {
    let sandbox = Sandbox::new("project_run").with_project(PROJECT);
    let run = sandbox.compiler_in("src/text", &["run"]);
    assert_eq!(run.assert_exit_code(0).stdout(), "hi!\n");
}

#[test]
fn build_writes_a_project_under_its_root_with_the_project_name() {
    let sandbox = Sandbox::new("project_build").with_project(PROJECT);
    sandbox.compiler_in("src", &["build"]).assert_exit_code(0);
    let greeter = sandbox.built_program(Sandbox::project_program("greeter"));
    assert_eq!(greeter.assert_exit_code(0).stdout(), "hi!\n");
}

#[test]
fn building_a_file_beside_the_entry_leaves_the_project_program_alone() {
    let sandbox = Sandbox::new("project_build_script")
        .with_project(PROJECT)
        .with_file("tools/script.knv", PROJECT_SCRIPT)
        .with_file("tools/helper.knv", PROJECT_HELPER);
    sandbox.compiler(&["build"]).assert_exit_code(0);
    sandbox
        .compiler(&["build", "tools/script.knv"])
        .assert_exit_code(0);
    let greeter = sandbox.built_program(Sandbox::project_program("greeter"));
    assert_eq!(greeter.assert_exit_code(0).stdout(), "hi!\n");
}

#[test]
fn run_in_a_project_with_only_dev_crates_builds_without_cargo() {
    let sandbox = Sandbox::new("crate_dev_only_run").with_fixture_tree("crates");
    let run = sandbox.compiler_in("dev-only-app", &["run"]);
    assert_eq!(run.assert_exit_code(0).stdout(), "hi\n");
    assert!(
        !sandbox
            .directory
            .join("dev-only-app/target/knave/dev-only-app/Cargo.toml")
            .exists()
    );
}

#[test]
fn a_second_test_run_through_cargo_rewrites_and_relinks_nothing() {
    let sandbox = Sandbox::new("crate_test_unchanged").with_fixture_tree("crates");
    let built = || {
        sandbox
            .compiler_in("dev-only-app", &["test"])
            .assert_exit_code(0);
        let deps = sandbox
            .directory
            .join(Sandbox::CACHE_DIRECTORY)
            .join(Language::NAME)
            .join("target/debug/deps");
        let binary = fs::read_dir(&deps)
            .expect("the shared target holds the built crates")
            .map(|entry| entry.expect("the target lists its files").path())
            .find(|path| {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                name.starts_with("dev_only_app_tests_") && path.extension().is_none()
            })
            .expect("the test target is built");
        let source = sandbox
            .directory
            .join("dev-only-app/target/knave/dev-only-app/test_runner/src/main.rs");
        [binary, source].map(|path| {
            fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .expect("the built files have a time")
        })
    };
    let first = built();
    assert_eq!(built(), first);
}

#[test]
fn two_test_runs_of_one_project_at_once_both_pass() {
    let sandbox = Sandbox::new("crate_test_together").with_fixture_tree("crates");
    let runs: Vec<Child> = (0..2)
        .map(|_| {
            sandbox
                .command(Path::new(Sandbox::COMPILER), &["test"])
                .current_dir(sandbox.directory.join("dev-only-app"))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("the compiler starts")
        })
        .collect();
    for run in runs {
        let output = run.wait_with_output().expect("the run ends");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "{stdout}");
        assert!(
            stdout.contains("test checks/reads-the-dev-crate ... ok"),
            "{stdout}"
        );
    }
}

#[test]
fn a_test_run_waits_while_another_run_builds_the_same_crate() {
    let sandbox = Sandbox::new("crate_test_waiting").with_fixture_tree("crates");
    let crate_directory = sandbox.directory.join(format!(
        "dev-only-app/target/{}/dev-only-app/test_runner",
        Language::NAME
    ));
    fs::create_dir_all(&crate_directory).expect("the sandbox accepts directories");
    let held = fs::File::create(crate_directory.join("build.lock")).expect("the lock file opens");
    let locked = unsafe { libc::flock(std::os::fd::AsRawFd::as_raw_fd(&held), libc::LOCK_EX) };
    assert_eq!(locked, 0);
    let mut run = sandbox
        .command(Path::new(Sandbox::COMPILER), &["test"])
        .current_dir(sandbox.directory.join("dev-only-app"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the compiler starts");
    let mut first_line = String::new();
    BufReader::new(run.stderr.take().expect("standard error is piped"))
        .read_line(&mut first_line)
        .expect("standard error is readable");
    drop(held);
    let output = run.wait_with_output().expect("the run ends");
    assert!(
        first_line.starts_with("note: waiting for another build in `"),
        "{first_line}"
    );
    assert!(output.status.success());
}

#[test]
fn a_test_run_holds_its_crate_lock_while_cargo_builds() {
    let sandbox = Sandbox::new("crate_test_lock_held").with_fixture_tree("crates");
    let release = sandbox.directory.join("release-cargo");
    let fifo = std::ffi::CString::new(release.as_os_str().as_encoded_bytes())
        .expect("the sandbox path has no zero byte");
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let blocked_cargo = format!("#!/bin/sh\n: < '{}'\nexit 101\n", release.display());
    let fake_cargo = sandbox.directory.join("blocked-bin/cargo");
    fs::create_dir_all(fake_cargo.parent().expect("the fake cargo has a directory"))
        .expect("the sandbox accepts directories");
    fs::write(&fake_cargo, blocked_cargo).expect("the sandbox accepts files");
    fs::set_permissions(&fake_cargo, fs::Permissions::from_mode(Sandbox::EXECUTABLE))
        .expect("the sandbox sets permissions");
    let mut run = sandbox
        .command(Path::new(Sandbox::COMPILER), &["test"])
        .current_dir(sandbox.directory.join("dev-only-app"))
        .env("PATH", sandbox.directory.join("blocked-bin"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the compiler starts");
    let crate_directory = sandbox.directory.join(format!(
        "dev-only-app/target/{}/dev-only-app/test_runner",
        Language::NAME
    ));
    let started = Instant::now();
    while !crate_directory.join("src/main.rs").is_file() {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "the crate is written"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let lock = fs::File::open(crate_directory.join("build.lock")).expect("the lock file exists");
    let taken = unsafe {
        libc::flock(
            std::os::fd::AsRawFd::as_raw_fd(&lock),
            libc::LOCK_EX | libc::LOCK_NB,
        )
    };
    fs::write(&release, "").expect("the fake cargo is released");
    run.wait().expect("the run ends");
    assert_eq!(taken, -1, "the run holds its lock while cargo builds");
}

#[test]
fn a_test_program_does_not_pass_its_marker_to_child_processes() {
    let sandbox = Sandbox::new("test_marker_hidden").with_file(
        "hidden.knv",
        "(use prelude/test assert deftest)\n\n(def marker-hidden\n  (rust \"std::env::var(\\\"KNAVE_TEST_MARKER\\\").is_err()\" (fn () bool)))\n\n(deftest hides-the-marker\n  (assert (marker-hidden)))\n",
    );
    let stdout = sandbox
        .compiler(&["test", "hidden.knv"])
        .assert_exit_code(0)
        .stdout();
    assert!(
        stdout.contains("test hidden/hides-the-marker ... ok"),
        "{stdout}"
    );
}

#[test]
fn test_builds_through_cargo_with_only_dev_crates() {
    let sandbox = Sandbox::new("crate_dev_only_test").with_fixture_tree("crates");
    let stdout = sandbox
        .compiler_in("dev-only-app", &["test"])
        .assert_exit_code(0)
        .stdout();
    assert!(
        stdout.contains("test checks/reads-the-dev-crate ... ok"),
        "{stdout}"
    );
}

#[test]
fn eval_names_a_module_in_a_directory_that_does_not_exist() {
    let sandbox = Sandbox::new("project_absent_directory");
    let eval = sandbox.compiler_reading(
        "(defn f () 1)\n",
        &["eval", "--stdin", "notes/draft.knv", "(f)"],
    );
    assert_eq!(eval.assert_silent_success().stdout(), "1\n");
}

#[test]
fn eval_with_a_root_names_a_module_by_its_path_from_the_root() {
    let sandbox = Sandbox::new("root_module_name").with_file("sub/c.knv", SELF_IMPORT);
    let eval = sandbox.compiler(&["eval", "--root", ".", "sub/c.knv", "(f)"]);
    insta::assert_snapshot!(eval.assert_exit_code(1).stderr());
}

#[test]
fn check_resolves_use_from_the_source_root_for_a_module_in_a_subdirectory() {
    let sandbox = Sandbox::new("project_module").with_project(PROJECT);
    sandbox
        .compiler(&["check", "src/text/shout.knv"])
        .assert_silent_success();
}

#[test]
fn a_file_outside_the_source_root_resolves_use_from_its_directory() {
    let sandbox = Sandbox::new("project_script")
        .with_project(PROJECT)
        .with_file("tools/script.knv", PROJECT_SCRIPT)
        .with_file("tools/helper.knv", PROJECT_HELPER);
    let run = sandbox.compiler(&["run", "tools/script.knv"]);
    assert_eq!(run.assert_exit_code(0).stdout(), "4\n");
}

#[test]
fn a_command_without_a_file_outside_a_project_is_a_usage_error() {
    let sandbox = Sandbox::new("project_missing");
    let check = sandbox.compiler(&["check"]);
    insta::assert_snapshot!(check.assert_exit_code(2).stderr());
}

#[test]
fn a_project_file_error_stops_the_command() {
    let sandbox = Sandbox::new("project_error").with_project(PROJECT_WITH_UNKNOWN_KEY);
    let check = sandbox.compiler(&["check", "src/main.knv"]);
    insta::assert_snapshot!(check.assert_exit_code(1).stderr());
}

#[test]
fn eval_resolves_use_from_the_project_source_root() {
    let sandbox = Sandbox::new("project_eval").with_project(PROJECT);
    let eval = sandbox.compiler_in("src/text", &["eval", "shout.knv", "(shout \"a\")"]);
    assert_eq!(eval.assert_silent_success().stdout(), "\"a!\"\n");
}

#[test]
fn repl_resolves_use_from_the_project_source_root() {
    let sandbox = Sandbox::new("project_repl").with_project(PROJECT);
    let repl = sandbox.compiler_reading("(use text/shout shout)\n(shout \"b\")\n", &["repl"]);
    assert_eq!(repl.assert_exit_code(0).stdout(), "\"b!\"\n");
}

#[test]
fn test_skips_the_project_file_in_a_directory() {
    let sandbox = Sandbox::new("project_test")
        .with_file(&Language::project_file_name(), PROJECT)
        .with_file("src/checks.knv", PROJECT_TESTS);
    let test = sandbox.compiler(&["test", "."]);
    assert!(test.assert_exit_code(0).stdout().contains("1 passed"));
}

#[test]
fn test_in_a_project_reaches_a_source_module() {
    let sandbox = Sandbox::new("project_test_source").with_fixture_tree("libraries");
    let test = sandbox.compiler_in("tested-app", &["test", "tests", "--exact", "parse/parses"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "running 1 test\ntest parse/parses ... ok\n1 passed, 0 failed\n"
    );
}

#[test]
fn test_of_a_file_in_a_project_reaches_a_source_module() {
    let sandbox = Sandbox::new("project_test_file").with_fixture_tree("libraries");
    let test = sandbox.compiler_in("tested-app", &["test", "tests/parse.knv"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "running 2 tests\n\
         test parse/parses ... ok\n\
         test parser/parses-itself ... ok\n\
         2 passed, 0 failed\n"
    );
}

#[test]
fn test_of_the_project_root_loads_each_source_module_once() {
    let sandbox = Sandbox::new("project_test_root").with_fixture_tree("libraries");
    let test = sandbox.compiler_in("tested-app", &["test", ".", "--filter", "pars"]);
    insta::assert_snapshot!(test.assert_silent_success().stdout());
}

#[test]
fn test_in_a_project_reaches_a_library() {
    let sandbox = Sandbox::new("project_test_library").with_fixture_tree("libraries");
    let test = sandbox.compiler_in("tested-app", &["test", "tests", "--exact", "shout/shouts"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "running 1 test\ntest shout/shouts ... ok\n1 passed, 0 failed\n"
    );
}

#[test]
fn test_directory_module_shadows_a_source_module() {
    let sandbox = Sandbox::new("project_test_shadowing").with_fixture_tree("libraries");
    let test = sandbox.compiler_in(
        "tested-app",
        &[
            "test",
            "shadowing-tests",
            "--exact",
            "greets/reads-the-test-greeting",
        ],
    );
    assert_eq!(
        test.assert_exit_code(0).stdout(),
        "running 1 test\ntest greets/reads-the-test-greeting ... ok\n1 passed, 0 failed\n"
    );
    insta::assert_snapshot!(test.stderr());
}

#[test]
fn test_in_a_project_warns_of_a_source_module_hidden_by_a_library() {
    let sandbox = Sandbox::new("project_test_hidden_source")
        .with_fixture_tree("libraries")
        .with_file("tested-app/src/hints.knv", "(def unused 1)\n");
    let test = sandbox.compiler_in("tested-app", &["test", "tests", "--exact", "shout/shouts"]);
    insta::assert_snapshot!(test.assert_exit_code(0).stderr());
}

#[test]
fn test_refuses_a_test_file_that_takes_a_compiler_module_path() {
    let failing_test = "(use prelude/test deftest assert)\n(deftest fails (assert false))\n";
    let sandbox = Sandbox::new("test_compiler_module_paths")
        .with_file("tests/prelude.knv", failing_test)
        .with_file("tests/kernel.knv", failing_test)
        .with_file("tests/code.knv", "this does not ) read (\n")
        .with_file("tests/prelude/vec.knv", failing_test)
        .with_file("tests/prelude/extra.knv", failing_test)
        .with_file("tests/other.knv", failing_test);
    let test = sandbox.compiler(&["test", "tests"]);
    insta::assert_snapshot!(test.assert_exit_code(1).stderr());
}

#[test]
fn test_outside_a_project_resolves_use_from_the_directory_alone() {
    let sandbox = Sandbox::new("loose_test").with_fixture_tree("loose-tests");
    let test = sandbox.compiler(&["test", "tests"]);
    insta::assert_snapshot!(test.assert_exit_code(1).stderr());
}

#[test]
fn test_without_a_path_runs_the_project_tests_directory_from_a_subdirectory() {
    let sandbox = Sandbox::new("project_test_default").with_fixture_tree("libraries");
    let test = sandbox.compiler_in("tested-app/src", &["test"]);
    insta::assert_snapshot!(test.assert_silent_success().stdout());
}

#[test]
fn test_without_a_path_in_a_project_without_tests_directory_is_a_usage_error() {
    let sandbox = Sandbox::new("project_test_no_tests").with_fixture_tree("libraries");
    let test = sandbox.compiler_in("app", &["test"]);
    insta::assert_snapshot!(test.assert_exit_code(2).stderr());
}

#[test]
fn test_without_a_path_outside_a_project_is_a_usage_error() {
    let test = Sandbox::new("test_without_path").compiler(&["test"]);
    insta::assert_snapshot!(test.assert_exit_code(2).stderr());
}

#[test]
fn fmt_check_without_a_path_prints_paths_relative_to_the_working_directory() {
    let sandbox = Sandbox::new("fmt_project_check_paths")
        .with_file(&Language::project_file_name(), PROJECT)
        .with_file("src/twice.knv", UNFORMATTED);
    let from_root = sandbox.compiler(&["fmt", "--check"]);
    let from_source = sandbox.compiler_in("src", &["fmt", "--check"]);
    assert_eq!(
        [
            from_root.assert_exit_code(1).stdout(),
            from_source.assert_exit_code(1).stdout()
        ],
        ["src/twice.knv\n", "../src/twice.knv\n"]
    );
}

#[test]
fn fmt_without_a_path_lays_out_the_whole_project_from_a_subdirectory() {
    let sandbox = Sandbox::new("fmt_project_default")
        .with_file(&Language::project_file_name(), UNLAID_PROJECT)
        .with_file("src/twice.knv", UNFORMATTED)
        .with_file("src/.direnv/limit.knv", UNFORMATTED);
    sandbox.compiler_in("src", &["fmt"]).assert_silent_success();
    assert_eq!(
        [
            sandbox.contents("src/twice.knv"),
            sandbox.contents("src/.direnv/limit.knv"),
        ],
        [FORMATTED, UNFORMATTED].map(String::from)
    );
    insta::assert_snapshot!(sandbox.contents(&Language::project_file_name()));
}

#[test]
fn new_creates_a_project_that_runs() {
    let sandbox = Sandbox::new("new_project");
    sandbox
        .compiler(&["new", "demo-app"])
        .assert_silent_success();
    let run = sandbox.compiler_in("demo-app", &["run"]);
    assert_eq!(run.assert_exit_code(0).stdout(), "hello\n");
}

#[test]
fn new_writes_a_gitignore_that_lists_the_doc_folder() {
    let sandbox = Sandbox::new("new_gitignore");
    sandbox
        .compiler(&["new", "demo-app"])
        .assert_silent_success();
    assert_eq!(sandbox.contents("demo-app/.gitignore"), "/doc/\n");
}

#[test]
fn new_refuses_a_directory_that_exists() {
    let sandbox = Sandbox::new("new_existing").with_file("demo-app/notes.txt", "kept\n");
    let new = sandbox.compiler(&["new", "demo-app"]);
    insta::assert_snapshot!(new.assert_exit_code(2).stderr());
}

#[test]
fn new_rejects_a_name_that_is_not_kebab_case() {
    let sandbox = Sandbox::new("new_bad_name");
    let new = sandbox.compiler(&["new", "DemoApp"]);
    insta::assert_snapshot!(new.assert_exit_code(1).stderr());
}

#[test]
fn fmt_lays_out_the_project_file_by_its_shapes() {
    let sandbox =
        Sandbox::new("fmt_project").with_file(&Language::project_file_name(), UNLAID_PROJECT);
    let fmt = sandbox.compiler_reading(
        UNLAID_PROJECT,
        &["fmt", "-", "--stdin-path", &Language::project_file_name()],
    );
    insta::assert_snapshot!(fmt.assert_silent_success().stdout());
}

#[test]
fn fmt_puts_each_bare_name_in_the_project_file_on_its_own_line() {
    let sandbox = Sandbox::new("fmt_project_names");
    let fmt = sandbox.compiler_reading(
        BARE_NAMES_PROJECT,
        &["fmt", "-", "--stdin-path", &Language::project_file_name()],
    );
    insta::assert_snapshot!(fmt.assert_silent_success().stdout());
}

#[test]
fn new_labels_each_rejected_name_on_the_argument() {
    let sandbox = Sandbox::new("new_rejected_names");
    let rejected = [
        ("digit_first", "1abc"),
        ("spaced", "a b"),
        ("entry_injection", "x :entry \"../outside/escaped\""),
    ];
    for (label, name) in rejected {
        let new = sandbox.compiler(&["new", name]);
        insta::assert_snapshot!(
            format!("new_rejects_{label}"),
            new.assert_exit_code(1).stderr()
        );
    }
    assert_eq!(
        fs::read_dir(&sandbox.directory)
            .expect("the sandbox is readable")
            .count(),
        0
    );
}

#[test]
fn new_reports_a_directory_it_cannot_write() {
    let sandbox = Sandbox::new("new_unwritable");
    fs::set_permissions(
        &sandbox.directory,
        fs::Permissions::from_mode(Sandbox::UNWRITABLE_DIRECTORY),
    )
    .expect("the sandbox sets permissions");
    let new = sandbox.compiler(&["new", "demo"]);
    let stderr = new.assert_exit_code(2).stderr();
    assert!(
        stderr.starts_with("error: cannot write `demo/project.knv`: "),
        "{stderr}"
    );
}

#[test]
fn new_rejects_an_empty_name_as_a_usage_error() {
    let sandbox = Sandbox::new("new_empty_name");
    let new = sandbox.compiler(&["new", ""]);
    insta::assert_snapshot!(new.assert_exit_code(2).stderr());
}

#[cfg(target_os = "linux")]
#[test]
fn new_removes_its_directory_after_a_failed_write() {
    let sandbox = Sandbox::new("new_failed_write");
    let new = Outcome::of(&mut sandbox.command(
        Path::new("sh"),
        &[
            "-c",
            "trap '' XFSZ; ulimit -f 0; exec \"$0\" new demo",
            Sandbox::COMPILER,
        ],
    ));
    let stderr = new.assert_exit_code(2).stderr();
    assert!(
        stderr.starts_with("error: cannot write `demo/project.knv`: "),
        "{stderr}"
    );
    assert!(!sandbox.directory.join("demo").exists());
}

#[test]
fn run_uses_a_path_library_and_the_library_it_lists() {
    let sandbox = Sandbox::new("library_run").with_fixture_tree("libraries");
    let run = sandbox.compiler_in("app", &["run"]);
    assert_eq!(run.assert_exit_code(0).stdout(), "hi!k\n!\n");
}

#[test]
fn a_library_listed_twice_at_one_source_loads_once() {
    let sandbox = Sandbox::new("library_listed_twice").with_fixture_tree("libraries");
    let run = sandbox.compiler_in("listed-twice", &["run"]);
    assert_eq!(run.assert_exit_code(0).stdout(), "hi!k\n");
}

#[test]
fn eval_with_a_root_reaches_the_project_libraries() {
    let sandbox = Sandbox::new("library_eval_root").with_fixture_tree("libraries");
    let eval = sandbox.compiler_in(
        "app",
        &["eval", "--root", "src", "src/main.knv", "(shout \"e\")"],
    );
    assert_eq!(eval.assert_silent_success().stdout(), "\"e!k\"\n");
}

#[test]
fn repl_with_a_root_reaches_the_project_libraries() {
    let sandbox = Sandbox::new("library_repl_root").with_fixture_tree("libraries");
    let repl = sandbox.compiler_reading_in(
        "app",
        "(use hints shout)\n(shout \"r\")\n",
        &["repl", "--root", "src"],
    );
    assert_eq!(repl.assert_exit_code(0).stdout(), "\"r!k\"\n");
}

#[test]
fn a_program_cannot_use_a_library_its_project_does_not_list() {
    let check = Sandbox::check_fixture("libraries/unlisted");
    insta::assert_snapshot!(check.assert_exit_code(1).stderr());
}

#[test]
fn a_library_named_as_the_entry_module_leaves_the_entry_in_the_program() {
    Sandbox::check_fixture("libraries/main-library").assert_silent_success();
}

#[test]
fn a_library_at_two_sources_stops_the_command() {
    let check = Sandbox::check_fixture("libraries/two-sources");
    insta::assert_snapshot!(check.assert_exit_code(1).stderr());
}

#[test]
fn a_library_that_hides_a_local_module_warns_once() {
    let check = Sandbox::check_fixture("libraries/hidden-file");
    insta::assert_snapshot!(check.assert_exit_code(0).stderr());
}

#[test]
fn a_library_macro_cannot_import_the_prelude_only_require_module() {
    let check = Sandbox::check_fixture("libraries/loader-app");
    insta::assert_snapshot!(check.assert_exit_code(1).stderr());
}

#[test]
fn a_library_that_hides_a_local_directory_warns() {
    let check = Sandbox::check_fixture("libraries/hidden-directory");
    insta::assert_snapshot!(check.assert_exit_code(0).stderr());
}

#[test]
fn a_library_without_a_project_file_names_the_entry() {
    let check = Sandbox::check_fixture("libraries/missing-project");
    insta::assert_snapshot!(check.assert_exit_code(1).stderr());
}

#[test]
fn a_library_without_its_root_module_names_the_file() {
    let check = Sandbox::check_fixture("libraries/missing-lib");
    insta::assert_snapshot!(check.assert_exit_code(1).stderr());
}

#[test]
fn a_library_module_never_falls_back_to_a_local_file() {
    let check = Sandbox::check_fixture("libraries/hidden-directory-module");
    insta::assert_snapshot!(check.assert_exit_code(1).stderr());
}

#[test]
fn a_local_module_that_shares_a_library_path_is_an_error() {
    let check = Sandbox::check_fixture("libraries/local-keymap");
    insta::assert_snapshot!(check.assert_exit_code(1).stderr());
}

#[cfg(target_os = "linux")]
struct DeepProgram;

#[cfg(target_os = "linux")]
impl DeepProgram {
    const FILE: &str = "deep.knv";

    fn nested(prefix: &str, open: &str, innermost: &str, close: &str, suffix: &str) -> String {
        format!(
            "{prefix}{}{innermost}{}{suffix}",
            open.repeat(DEEP_NESTING),
            close.repeat(DEEP_NESTING)
        )
    }

    fn blocks() -> String {
        Self::nested("(defn main () (println ", "(do ", "1", ")", "))\n")
    }

    fn calls() -> String {
        Self::nested(
            "(defn add (a b) (+ a b))\n(defn main () (println ",
            "(add 1 ",
            "1",
            ")",
            "))\n",
        )
    }

    fn matches() -> String {
        let depth = DEEP_NESTING / 2;
        format!(
            "(defn step (n) (if (> n 1000) none (some (+ n 1))))\n(defn main () (println {}n{}))\n",
            "(match (step 0) ((some n) ".repeat(depth),
            ") (none 0))".repeat(depth)
        )
    }

    fn recur_body() -> String {
        Self::nested(
            "(defn count-down (n) (if (> n 0) ",
            "(do ",
            "(recur (- n 1))",
            ")",
            " n))\n(defn main () (println (count-down 3)))\n",
        )
    }

    fn passes_on_a_short_stack(test_name: &str, source: &str, command: &str) {
        let sandbox = Sandbox::new(test_name).with_file(Self::FILE, source);
        sandbox
            .compiler_on_a_short_stack(&[command, Self::FILE])
            .assert_exit_code(0);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn deep_blocks_check_on_a_short_stack() {
    DeepProgram::passes_on_a_short_stack("deep_blocks_check", &DeepProgram::blocks(), "check");
}

#[cfg(target_os = "linux")]
#[test]
fn deep_calls_check_on_a_short_stack() {
    DeepProgram::passes_on_a_short_stack("deep_calls_check", &DeepProgram::calls(), "check");
}

#[cfg(target_os = "linux")]
#[test]
fn deep_matches_check_on_a_short_stack() {
    DeepProgram::passes_on_a_short_stack("deep_matches_check", &DeepProgram::matches(), "check");
}

#[cfg(target_os = "linux")]
#[test]
fn a_deep_recur_body_checks_on_a_short_stack() {
    DeepProgram::passes_on_a_short_stack("deep_recur_check", &DeepProgram::recur_body(), "check");
}

#[cfg(target_os = "linux")]
#[test]
fn deep_blocks_emit_on_a_short_stack() {
    DeepProgram::passes_on_a_short_stack("deep_blocks_emit", &DeepProgram::blocks(), "emit");
}

#[cfg(target_os = "linux")]
#[test]
fn deep_calls_emit_on_a_short_stack() {
    DeepProgram::passes_on_a_short_stack("deep_calls_emit", &DeepProgram::calls(), "emit");
}

#[cfg(target_os = "linux")]
#[test]
fn deep_matches_emit_on_a_short_stack() {
    DeepProgram::passes_on_a_short_stack("deep_matches_emit", &DeepProgram::matches(), "emit");
}

#[cfg(target_os = "linux")]
#[test]
fn a_deep_recur_body_emits_on_a_short_stack() {
    DeepProgram::passes_on_a_short_stack("deep_recur_emit", &DeepProgram::recur_body(), "emit");
}

#[test]
fn run_fetches_a_git_library_into_the_cache_by_commit() {
    let (sandbox, commit) = Sandbox::new("git_library_run").with_git_library(None);
    let run = sandbox.compiler_in("git-app/src", &["run"]);
    assert_eq!(run.assert_exit_code(0).stdout(), "k\n");
    let checkout = sandbox
        .directory
        .join(Sandbox::CACHE_DIRECTORY)
        .join(Language::NAME)
        .join("git")
        .join(&commit);
    assert!(checkout.join("src/lib.knv").is_file());
}

#[test]
fn a_fetched_git_library_needs_no_repository_again() {
    let (sandbox, _) = Sandbox::new("git_library_cached").with_git_library(None);
    sandbox
        .compiler_in("git-app", &["check"])
        .assert_silent_success();
    fs::remove_dir_all(sandbox.directory.join("keymap.git"))
        .expect("the sandbox removes directories");
    let run = sandbox.compiler_in("git-app", &["run"]);
    assert_eq!(run.assert_exit_code(0).stdout(), "k\n");
}

#[test]
fn a_git_library_at_a_missing_commit_names_the_library() {
    let (sandbox, _) =
        Sandbox::new("git_library_missing_commit").with_git_library(Some(MISSING_COMMIT));
    let check = sandbox.compiler_in("git-app", &["check"]);
    let stderr = check.assert_exit_code(1).stderr();
    let shown: Vec<&str> = stderr
        .lines()
        .map(|line| {
            if line.contains(Sandbox::GIT_MESSAGE) {
                Sandbox::MASKED_GIT_MESSAGE
            } else {
                line
            }
        })
        .collect();
    insta::assert_snapshot!(shown.join("\n"));
    let store = sandbox
        .directory
        .join(Sandbox::CACHE_DIRECTORY)
        .join(Language::NAME)
        .join("git");
    assert_eq!(fs::read_dir(store).expect("the store exists").count(), 0);
}

#[test]
fn a_git_library_without_git_is_an_environment_error() {
    let (sandbox, _) = Sandbox::new("git_library_no_git").with_git_library(None);
    let mut command = sandbox.command(Path::new(Sandbox::COMPILER), &["check"]);
    command
        .current_dir(sandbox.directory.join("git-app"))
        .env("PATH", sandbox.directory.join("no-tools"));
    let check = Outcome::of(&mut command);
    let stderr = check.assert_exit_code(2).stderr();
    assert!(
        stderr.starts_with("error: cannot run `git` to fetch git libraries: "),
        "{stderr}"
    );
}

#[test]
fn a_git_library_without_a_cache_directory_is_an_environment_error() {
    let (sandbox, _) = Sandbox::new("git_library_no_cache").with_git_library(None);
    let mut command = sandbox.command(Path::new(Sandbox::COMPILER), &["check"]);
    command
        .current_dir(sandbox.directory.join("git-app"))
        .env_remove(Sandbox::CACHE_HOME_VARIABLE)
        .env_remove("HOME");
    let check = Outcome::of(&mut command);
    insta::assert_snapshot!(check.assert_exit_code(2).stderr());
}

#[test]
fn a_git_library_falls_back_to_the_cache_under_home() {
    let (sandbox, commit) = Sandbox::new("git_library_home_cache").with_git_library(None);
    let mut command = sandbox.command(Path::new(Sandbox::COMPILER), &["check"]);
    command
        .current_dir(sandbox.directory.join("git-app"))
        .env_remove(Sandbox::CACHE_HOME_VARIABLE)
        .env("HOME", sandbox.directory.join("home"));
    Outcome::of(&mut command).assert_silent_success();
    let checkout = sandbox
        .directory
        .join("home/.cache")
        .join(Language::NAME)
        .join("git")
        .join(&commit);
    assert!(checkout.is_dir());
}

#[test]
fn a_git_library_with_an_unwritable_cache_is_an_environment_error() {
    let (sandbox, _) = Sandbox::new("git_library_unwritable_cache").with_git_library(None);
    let cache = sandbox.directory.join(Sandbox::CACHE_DIRECTORY);
    fs::create_dir_all(&cache).expect("the sandbox accepts directories");
    fs::set_permissions(
        &cache,
        fs::Permissions::from_mode(Sandbox::UNWRITABLE_DIRECTORY),
    )
    .expect("the sandbox sets permissions");
    let check = sandbox.compiler_in("git-app", &["check"]);
    let stderr = check.assert_exit_code(2).stderr();
    fs::set_permissions(&cache, fs::Permissions::from_mode(Sandbox::EXECUTABLE))
        .expect("the sandbox sets permissions");
    assert!(stderr.starts_with("error: cannot write `"), "{stderr}");
}

#[test]
fn a_git_scheme_outside_the_allowed_list_is_refused() {
    let (sandbox, commit) = Sandbox::new("git_library_scheme").with_git_library(None);
    let project =
        format!("(project app (libraries (keymap :git \"ext::true\" :rev \"{commit}\")))\n");
    let sandbox = sandbox.with_file(
        &format!("git-app/{}", Language::project_file_name()),
        &project,
    );
    let check = sandbox.compiler_in("git-app", &["check"]);
    let stderr = check.assert_exit_code(1).stderr();
    assert!(
        stderr.starts_with("error[L0725]: cannot fetch git library `keymap`"),
        "{stderr}"
    );
    assert!(stderr.contains("not allowed"), "{stderr}");
}

#[test]
fn git_runs_with_no_prompt_the_allowed_schemes_and_batch_ssh() {
    let (sandbox, _) = Sandbox::new("git_library_environment").with_git_library(None);
    let stderr = sandbox
        .compiler_with_fake_git(None)
        .assert_exit_code(1)
        .stderr();
    assert!(
        stderr.contains(
            "git says: fatal: prompt=0 schemes=file:git:http:https:ssh ssh=ssh -o BatchMode=yes"
        ),
        "{stderr}"
    );
}

#[test]
fn git_keeps_the_ssh_command_the_user_set() {
    let (sandbox, _) = Sandbox::new("git_library_user_ssh").with_git_library(None);
    let stderr = sandbox
        .compiler_with_fake_git(Some("ssh -p 2222"))
        .assert_exit_code(1)
        .stderr();
    assert!(stderr.contains(" ssh=ssh -p 2222"), "{stderr}");
}

#[test]
fn two_git_libraries_share_one_store() {
    let sandbox = Sandbox::new("git_library_two").with_fixture_tree("libraries");
    let keymap = sandbox.git_repository("keymap");
    let marks = sandbox.git_repository("marks");
    let project = format!(
        "(project app (libraries (keymap :git \"../keymap.git\" :rev \"{keymap}\") (marks :git \"../marks.git\" :rev \"{marks}\")))\n"
    );
    let sandbox = sandbox.with_file(
        &format!("two-git-app/{}", Language::project_file_name()),
        &project,
    );
    let run = sandbox.compiler_in("two-git-app", &["run"]);
    assert_eq!(run.assert_exit_code(0).stdout(), "k?\n");
}

#[test]
fn an_empty_cache_variable_falls_back_to_home() {
    let (sandbox, commit) = Sandbox::new("git_library_empty_cache").with_git_library(None);
    let mut command = sandbox.command(Path::new(Sandbox::COMPILER), &["check"]);
    command
        .current_dir(sandbox.directory.join("git-app"))
        .env(Sandbox::CACHE_HOME_VARIABLE, "")
        .env("HOME", sandbox.directory.join("home"));
    Outcome::of(&mut command).assert_silent_success();
    let checkout = sandbox
        .directory
        .join("home/.cache")
        .join(Language::NAME)
        .join("git")
        .join(commit);
    assert!(checkout.is_dir());
}

#[test]
fn a_git_library_lib_outside_its_checkout_is_an_error() {
    let sandbox = Sandbox::new("git_library_lib_outside")
        .with_one_git_library("lib-outside", "../lib-outside.git");
    let check = sandbox.compiler_in("git-app", &["check"]);
    insta::assert_snapshot!(check.assert_exit_code(1).stderr());
}

#[test]
fn a_git_library_path_outside_its_checkout_is_an_error() {
    let sandbox = Sandbox::new("git_library_path_outside")
        .with_one_git_library("path-outside", "../path-outside.git");
    let stderr = sandbox
        .compiler_in("git-app", &["check"])
        .assert_exit_code(1)
        .stderr();
    assert!(
        stderr.starts_with("error[L0728]: library `other` names a `:path` outside its checkout"),
        "{stderr}"
    );
}

#[test]
fn a_git_library_relative_git_outside_its_checkout_is_an_error() {
    let sandbox = Sandbox::new("git_library_git_outside")
        .with_one_git_library("git-outside", "../git-outside.git");
    let stderr = sandbox
        .compiler_in("git-app", &["check"])
        .assert_exit_code(1)
        .stderr();
    assert!(
        stderr.starts_with("error[L0728]: library `other` names a `:git` outside its checkout"),
        "{stderr}"
    );
}

#[test]
fn a_library_inside_a_git_checkout_stays_inside_it_too() {
    let sandbox = Sandbox::new("git_library_chain").with_one_git_library("chain", "../chain.git");
    let stderr = sandbox
        .compiler_in("git-app", &["check"])
        .assert_exit_code(1)
        .stderr();
    assert!(
        stderr.starts_with("error[L0728]: library `far` names a `:path` outside its checkout"),
        "{stderr}"
    );
}

#[test]
fn a_git_library_file_url_outside_its_checkout_is_an_error() {
    let sandbox = Sandbox::new("git_library_file_url")
        .with_one_git_library("file-url-outside", "../file-url-outside.git");
    let stderr = sandbox
        .compiler_in("git-app", &["check"])
        .assert_exit_code(1)
        .stderr();
    assert!(
        stderr.starts_with("error[L0728]: library `other` names a `:git` outside its checkout"),
        "{stderr}"
    );
}

#[test]
fn a_git_library_absolute_path_with_a_colon_is_local() {
    let sandbox = Sandbox::new("git_library_colon_path")
        .with_one_git_library("colon-path-outside", "../colon-path-outside.git");
    let stderr = sandbox
        .compiler_in("git-app", &["check"])
        .assert_exit_code(1)
        .stderr();
    assert!(
        stderr.starts_with("error[L0728]: library `other` names a `:git` outside its checkout"),
        "{stderr}"
    );
}

#[test]
fn a_git_library_lib_through_a_symbolic_link_outside_is_an_error() {
    let sandbox = Sandbox::new("git_library_linked_lib")
        .with_one_git_library("linked-lib", "../linked-lib.git");
    let stderr = sandbox
        .compiler_in("git-app", &["check"])
        .assert_exit_code(1)
        .stderr();
    assert!(
        stderr
            .starts_with("error[L0728]: library `linked-lib` names a `:lib` outside its checkout"),
        "{stderr}"
    );
}

#[test]
fn libraries_in_one_git_checkout_may_name_each_other() {
    let sandbox = Sandbox::new("git_library_monorepo")
        .with_one_git_library("monorepo", "../monorepo.git")
        .with_file(
            "git-app/src/main.knv",
            "(defn main ()\n  (println \"hi\"))\n",
        );
    sandbox
        .compiler_in("git-app", &["check"])
        .assert_silent_success();
}

#[test]
fn a_git_library_lib_that_climbs_past_a_missing_directory_is_an_error() {
    let sandbox =
        Sandbox::new("git_library_lib_climb").with_one_git_library("lib-climb", "../lib-climb.git");
    let stderr = sandbox
        .compiler_in("git-app", &["check"])
        .assert_exit_code(1)
        .stderr();
    assert!(
        stderr.starts_with("error[L0728]: library `lib-climb` names a `:lib` outside its checkout"),
        "{stderr}"
    );
}

#[test]
fn a_git_library_may_list_another_git_library() {
    let sandbox = Sandbox::new("git_library_nested_git").with_fixture_tree("libraries");
    let inner = sandbox.git_repository("inner-git");
    let bundle = sandbox.directory.join("outer-git/inner.bundle");
    sandbox.git(
        "inner-git-source",
        &[
            "bundle",
            "create",
            "--quiet",
            &bundle.display().to_string(),
            "--all",
        ],
    );
    let outer_project = format!(
        "(project outer-git (libraries (inner-git :git \"inner.bundle\" :rev \"{inner}\")))\n"
    );
    let sandbox = sandbox.with_file(
        &format!("outer-git/{}", Language::project_file_name()),
        &outer_project,
    );
    let outer = sandbox.git_repository("outer-git");
    let project = format!(
        "(project app (libraries (outer-git :git \"../outer-git.git\" :rev \"{outer}\")))\n"
    );
    let sandbox = sandbox
        .with_file(
            &format!("git-app/{}", Language::project_file_name()),
            &project,
        )
        .with_file(
            "git-app/src/main.knv",
            "(use outer-git)\n\n(defn main ()\n  (println \"hi\"))\n",
        );
    let run = sandbox.compiler_in("git-app", &["run"]);
    assert_eq!(run.assert_exit_code(0).stdout(), "hi\n");
}

#[test]
fn a_module_file_linked_outside_a_git_checkout_is_an_error() {
    let sandbox = Sandbox::new("git_library_linked_module").with_fixture_tree("libraries");
    let sandbox = sandbox.with_one_git_library_from_tree("linked-module", "../linked-module.git");
    let sandbox = sandbox.with_file(
        "git-app/src/main.knv",
        "(use linked-module key)\n\n(defn main ()\n  (println key))\n",
    );
    let check = sandbox.compiler_in("git-app", &["check"]);
    let stderr = check.assert_exit_code(1).stderr();
    insta::assert_snapshot!(sandbox.masked(&stderr));
}

#[test]
fn run_builds_a_program_with_a_path_crate_through_cargo() {
    let sandbox = Sandbox::new("crate_run").with_fixture_tree("crates");
    let run = sandbox.compiler_in("app", &["run"]);
    assert_eq!(run.assert_exit_code(0).stdout(), "42\n");
    assert!(sandbox.directory.join("app/Cargo.lock").is_file());
}

#[test]
fn a_rustc_error_in_a_crate_build_maps_to_the_source() {
    let sandbox = Sandbox::new("crate_wrong_call").with_fixture_tree("crates");
    let build = sandbox.compiler_in("wrong-call-app", &["build"]);
    insta::assert_snapshot!(build.assert_exit_code(1).stderr());
}

#[test]
fn a_crate_that_fails_to_build_is_named() {
    let sandbox = Sandbox::new("crate_broken").with_fixture_tree("crates");
    let build = sandbox.compiler_in("broken-app", &["build"]);
    let stderr = build.assert_exit_code(2).stderr();
    assert!(
        stderr.starts_with("error: crate `broken` failed to build:"),
        "{stderr}"
    );
    assert!(stderr.contains("mismatched types"), "{stderr}");
}

#[test]
fn a_crate_cargo_cannot_resolve_prints_cargo_text() {
    let sandbox = Sandbox::new("crate_unresolved").with_fixture_tree("crates");
    let build = sandbox.compiler_in("unresolved-app", &["build"]);
    let stderr = build.assert_exit_code(2).stderr();
    assert!(stderr.starts_with("error: "), "{stderr}");
    assert!(stderr.contains("Cargo.toml"), "{stderr}");
}

#[test]
fn a_crate_build_without_cargo_is_an_environment_error() {
    let sandbox = Sandbox::new("crate_no_cargo").with_fixture_tree("crates");
    let mut command = sandbox.command(Path::new(Sandbox::COMPILER), &["build"]);
    command
        .current_dir(sandbox.directory.join("app"))
        .env("PATH", sandbox.directory.join("no-tools"));
    let stderr = Outcome::of(&mut command).assert_exit_code(2).stderr();
    assert!(
        stderr
            .contains("error: cannot run cargo, which builds a project with crate dependencies: "),
        "{stderr}"
    );
}

#[test]
fn eval_in_a_project_with_crates_says_it_cannot_use_them_yet() {
    let eval = Sandbox::compiler_in_fixture("crates/app", &["eval", "src/main.knv", "1"]);
    insta::assert_snapshot!(eval.assert_exit_code(1).stderr());
}

#[test]
fn repl_in_a_project_with_crates_says_it_cannot_use_them_yet() {
    let repl = Sandbox::compiler_in_fixture("crates/app", &["repl"]);
    insta::assert_snapshot!(repl.assert_exit_code(1).stderr());
}

#[test]
fn test_builds_through_cargo_with_only_the_project_crates() {
    let sandbox = Sandbox::new("crate_test_without_dev").with_fixture_tree("crates");
    let stdout = sandbox
        .compiler_in("app", &["test"])
        .assert_exit_code(0)
        .stdout();
    assert!(
        stdout.contains("test answers/reads-the-crate ... ok"),
        "{stdout}"
    );
}

#[test]
fn test_builds_through_cargo_with_the_crates_and_dev_crates() {
    let sandbox = Sandbox::new("crate_test").with_fixture_tree("crates");
    let stdout = sandbox
        .compiler_in("tested-app", &["test"])
        .assert_exit_code(0)
        .stdout();
    assert!(
        stdout.contains("test checks/matches-the-expected-value ... ok"),
        "{stdout}"
    );
    assert!(
        sandbox
            .directory
            .join("tested-app/target/knave/tested-app/test_runner/Cargo.toml")
            .is_file()
    );
}

#[test]
fn crate_requests_that_cannot_merge_are_an_error() {
    let check = Sandbox::check_fixture("crates/conflict-app");
    insta::assert_snapshot!(check.assert_exit_code(1).stderr());
}

#[test]
fn a_program_behaves_the_same_built_by_rustc_and_by_cargo() {
    let sandbox = Sandbox::new("crate_compare").with_fixture_tree("crates");
    for scenario in ["overflow", "zero", "leave"] {
        for profile in [None, Some("--release")] {
            let direct = sandbox.compared_run(scenario, profile, false);
            let through_cargo = sandbox.compared_run(scenario, profile, true);
            assert_eq!(direct, through_cargo, "{scenario} {profile:?}");
        }
    }
}

#[test]
fn a_git_library_crate_path_outside_its_checkout_is_an_error() {
    let sandbox = Sandbox::new("git_library_crate_outside")
        .with_one_git_library("crate-outside", "../crate-outside.git");
    let stderr = sandbox
        .compiler_in("git-app", &["check"])
        .assert_exit_code(1)
        .stderr();
    assert!(
        stderr.starts_with(
            "error[L0728]: crate `answer` names a `:path` outside its library's checkout"
        ),
        "{stderr}"
    );
}

#[test]
fn a_library_crate_joins_the_program_manifest() {
    let sandbox = Sandbox::new("crate_from_library").with_fixture_tree("crates");
    let run = sandbox.compiler_in("library-app", &["run"]);
    assert_eq!(run.assert_exit_code(0).stdout(), "42\n");
    let manifest = fs::read_to_string(
        sandbox
            .directory
            .join("library-app/target/knave/app/Cargo.toml"),
    )
    .expect("the build writes a manifest");
    let shown: Vec<String> = sandbox
        .masked(&manifest)
        .lines()
        .map(|line| match line.strip_prefix(Sandbox::BIN_NAME_PREFIX) {
            Some(_) => format!("{}<hash>\"", Sandbox::BIN_NAME_PREFIX),
            None => line.to_string(),
        })
        .collect();
    insta::assert_snapshot!(shown.join("\n"));
}

#[test]
fn an_unchanged_program_keeps_its_generated_source_file() {
    let sandbox = Sandbox::new("crate_unchanged_source").with_fixture_tree("crates");
    sandbox.compiler_in("app", &["build"]).assert_exit_code(0);
    let source = sandbox.directory.join("app/target/knave/app/src/main.rs");
    let first = fs::metadata(&source)
        .and_then(|metadata| metadata.modified())
        .expect("the source exists");
    sandbox.compiler_in("app", &["build"]).assert_exit_code(0);
    let second = fs::metadata(&source)
        .and_then(|metadata| metadata.modified())
        .expect("the source exists");
    assert_eq!(first, second);
}

#[test]
fn run_of_another_file_in_a_project_with_crates_builds_through_cargo() {
    let sandbox = Sandbox::new("crate_other_file").with_fixture_tree("crates");
    let run = sandbox.compiler_in("app", &["run", "src/tools/next.knv"]);
    assert_eq!(run.assert_exit_code(0).stdout(), "43\n");
}

#[test]
fn a_missing_crate_path_resolves_from_the_project_root() {
    let sandbox = Sandbox::new("crate_missing_path")
        .with_fixture_tree("crates")
        .with_file(
            &format!("app/{}", Language::project_file_name()),
            "(project app (dependencies (answer :path \"../nowhere\")))\n",
        );
    let stderr = sandbox
        .compiler_in("app", &["build"])
        .assert_exit_code(2)
        .stderr();
    assert!(
        sandbox
            .masked(&stderr)
            .contains("<sandbox>/nowhere/Cargo.toml"),
        "{stderr}"
    );
}

#[test]
fn a_dependency_named_like_the_program_package_is_still_a_dependency() {
    let sandbox = Sandbox::new("crate_program_like_name").with_fixture_tree("crates");
    Sandbox::copy_tree(
        &sandbox.directory.join("broken"),
        &sandbox.directory.join("knave-program-broken"),
    );
    let sandbox = sandbox.with_file(
        &format!("broken-app/{}", Language::project_file_name()),
        "(project app (dependencies (broken :path \"../knave-program-broken\")))\n",
    );
    let stderr = sandbox
        .compiler_in("broken-app", &["build"])
        .assert_exit_code(2)
        .stderr();
    assert!(
        stderr.starts_with("error: crate `broken` failed to build:"),
        "{stderr}"
    );
}

#[test]
fn a_build_uses_the_project_cargo_lock() {
    let sandbox = Sandbox::new("crate_lock_in").with_fixture_tree("crates");
    sandbox.compiler_in("app", &["build"]).assert_exit_code(0);
    let lock = sandbox.directory.join("app/Cargo.lock");
    let written = fs::read_to_string(&lock).expect("the build writes a lock file");
    let older = written.replacen("version = 4", "version = 3", 1);
    fs::write(&lock, &older).expect("the sandbox accepts files");
    let entry = sandbox.directory.join("app/src/main.knv");
    let source = fs::read_to_string(&entry).expect("the entry is readable");
    fs::write(&entry, format!("{source}\n")).expect("the sandbox accepts files");
    sandbox.compiler_in("app", &["build"]).assert_exit_code(0);
    let kept = fs::read_to_string(&lock).expect("the lock file stays");
    assert!(kept.contains("version = 3"), "{kept}");
}

#[test]
fn a_dev_crate_that_cannot_merge_with_a_dependency_is_an_error() {
    let check = Sandbox::check_fixture("crates/dev-conflict-app");
    insta::assert_snapshot!(check.assert_exit_code(1).stderr());
}

#[test]
fn a_dev_crate_that_cannot_merge_with_a_library_crate_is_an_error() {
    let check = Sandbox::check_fixture("crates/dev-library-conflict-app");
    insta::assert_snapshot!(check.assert_exit_code(1).stderr());
}

#[test]
fn eval_names_the_library_that_adds_crates() {
    let eval =
        Sandbox::compiler_in_fixture("crates/library-eval-app", &["eval", "src/main.knv", "1"]);
    insta::assert_snapshot!(eval.assert_exit_code(1).stderr());
}

#[test]
fn eval_with_a_root_in_a_project_with_crates_says_it_cannot_use_them_yet() {
    let eval = Sandbox::compiler_in_fixture(
        "crates/app",
        &["eval", "--root", "src", "src/main.knv", "1"],
    );
    assert!(
        eval.assert_exit_code(1)
            .stderr()
            .starts_with("error[L0719]: `eval` cannot use crate dependencies yet")
    );
}

#[test]
fn add_writes_a_crate_with_the_version_cargo_picks_and_its_features() {
    let sandbox = Sandbox::new("add_crate").with_fake_registry();
    sandbox
        .compiler_with_registry(&["add", "fake-json", "--features", "derive,std"])
        .assert_silent_success();
    insta::assert_snapshot!(sandbox.project_file_text());
    sandbox.compiler(&["check"]).assert_silent_success();
}

#[test]
fn add_keeps_the_features_a_listed_crate_has() {
    let sandbox = Sandbox::new("add_listed_crate").with_fake_registry();
    sandbox
        .compiler_with_registry(&["add", "fake-json", "--features", "derive"])
        .assert_silent_success();
    sandbox
        .compiler_with_registry(&["add", "fake-json", "--features", "std"])
        .assert_silent_success();
    assert!(
        sandbox
            .project_file_text()
            .contains(":features (\"derive\" \"std\")")
    );
}

#[test]
fn add_of_a_crate_cargo_cannot_find_prints_cargo_text() {
    let sandbox = Sandbox::new("add_unknown_crate").with_fake_registry();
    let stderr = sandbox
        .compiler_with_registry(&["add", "no-such-crate"])
        .assert_exit_code(2)
        .stderr();
    assert!(
        stderr.starts_with("error: the crate `no-such-crate` could not be found"),
        "{stderr}"
    );
    assert_eq!(sandbox.project_file_text(), "(project demo)\n");
}

#[test]
fn add_outside_a_project_is_a_usage_error() {
    let sandbox = Sandbox::new("add_outside_project");
    insta::assert_snapshot!(
        sandbox
            .compiler(&["add", "fake-json"])
            .assert_exit_code(2)
            .stderr()
    );
}

#[test]
fn add_without_cargo_is_an_environment_error() {
    let sandbox = Sandbox::new("add_no_cargo").with_fake_registry();
    let mut command = sandbox.command(Path::new(Sandbox::COMPILER), &["add", "fake-json"]);
    command.env("PATH", sandbox.directory.join("no-tools"));
    let stderr = Outcome::of(&mut command).assert_exit_code(2).stderr();
    assert!(
        stderr.starts_with("error: cannot run cargo, which finds the version of a crate to add: "),
        "{stderr}"
    );
}

#[test]
fn remove_deletes_the_entry_and_the_emptied_section() {
    let sandbox = Sandbox::new("remove_crate").with_fake_registry();
    sandbox
        .compiler_with_registry(&["add", "fake-json"])
        .assert_silent_success();
    sandbox
        .compiler(&["remove", "fake-json"])
        .assert_silent_success();
    assert_eq!(sandbox.project_file_text(), "(project demo)\n");
}

#[test]
fn add_with_dev_writes_the_crate_into_dev_dependencies() {
    let sandbox = Sandbox::new("add_dev_crate").with_fake_registry();
    sandbox
        .compiler_with_registry(&["add", "--dev", "fake-json", "--features", "std"])
        .assert_silent_success();
    insta::assert_snapshot!(sandbox.project_file_text());
}

#[test]
fn add_with_dev_refuses_a_path_crate_listed_in_dev_dependencies() {
    let sandbox = Sandbox::new("add_dev_path_crate")
        .with_fake_registry()
        .with_file(
            &Language::project_file_name(),
            "(project demo (dev-dependencies (fake-json :path \"../fake-json\")))\n",
        );
    let stderr = sandbox
        .compiler_with_registry(&["add", "--dev", "fake-json"])
        .assert_exit_code(2)
        .stderr();
    assert_eq!(
        stderr,
        "error: `fake-json` is listed with `:path`\nhelp: run `knave remove --dev fake-json` first to switch it to a version\n"
    );
}

#[test]
fn add_with_dev_refuses_a_path_crate_listed_in_dependencies() {
    let sandbox = Sandbox::new("add_dev_over_path_dependency")
        .with_fake_registry()
        .with_file(
            &Language::project_file_name(),
            "(project demo (dependencies (fake-json :path \"../fake-json\")))\n",
        );
    let stderr = sandbox
        .compiler_with_registry(&["add", "--dev", "fake-json"])
        .assert_exit_code(2)
        .stderr();
    assert_eq!(
        stderr,
        "error: `fake-json` is listed with `:path`\nhelp: run `knave remove fake-json` first to switch it to a version\n"
    );
}

#[test]
fn add_with_dev_keeps_the_features_of_the_dev_entry() {
    let sandbox = Sandbox::new("add_dev_beside_dependency")
        .with_fake_registry()
        .with_file(
            &Language::project_file_name(),
            "(project demo\n  (dependencies (fake-json \"1.0\"))\n  (dev-dependencies (fake-json \"1.0\" :features (\"std\"))))\n",
        );
    sandbox
        .compiler_with_registry(&["add", "--dev", "fake-json"])
        .assert_silent_success();
    insta::assert_snapshot!(sandbox.project_file_text());
}

#[test]
fn remove_of_a_crate_only_the_other_section_lists_names_that_section() {
    let sandbox = Sandbox::new("remove_from_other_section")
        .with_fake_registry()
        .with_file(
            &Language::project_file_name(),
            "(project demo (dev-dependencies (fake-json \"1.0\")))\n",
        );
    let stderr = sandbox
        .compiler(&["remove", "fake-json"])
        .assert_exit_code(2)
        .stderr();
    assert_eq!(
        stderr,
        "error: the project's `dependencies` do not list `fake-json`\nhelp: `dev-dependencies` lists it: run `knave remove --dev fake-json`\n"
    );
}

#[test]
fn remove_with_dev_of_a_crate_only_dependencies_list_names_them() {
    let sandbox = Sandbox::new("remove_dev_from_other_section")
        .with_fake_registry()
        .with_file(
            &Language::project_file_name(),
            "(project demo (dependencies (fake-json \"1.0\")))\n",
        );
    let stderr = sandbox
        .compiler(&["remove", "--dev", "fake-json"])
        .assert_exit_code(2)
        .stderr();
    assert_eq!(
        stderr,
        "error: the project's `dev-dependencies` do not list `fake-json`\nhelp: `dependencies` lists it: run `knave remove fake-json`\n"
    );
}

#[test]
fn remove_with_dev_leaves_the_dependencies_alone() {
    let sandbox = Sandbox::new("remove_dev_crate").with_fake_registry();
    for arguments in [
        ["add", "fake-json"].as_slice(),
        &["add", "--dev", "fake-json"],
    ] {
        sandbox
            .compiler_with_registry(arguments)
            .assert_silent_success();
    }
    sandbox
        .compiler(&["remove", "--dev", "fake-json"])
        .assert_silent_success();
    let text = sandbox.project_file_text();
    assert!(text.contains("(dependencies (fake-json"), "{text}");
    assert!(!text.contains("dev-dependencies"), "{text}");
}

#[test]
fn remove_with_dev_of_an_unlisted_crate_names_dev_dependencies() {
    let sandbox = Sandbox::new("remove_dev_unlisted").with_fake_registry();
    let stderr = sandbox
        .compiler(&["remove", "--dev", "fake-json"])
        .assert_exit_code(2)
        .stderr();
    assert_eq!(
        stderr,
        "error: the project's `dev-dependencies` do not list `fake-json`\n"
    );
}

#[test]
fn remove_of_a_crate_the_project_does_not_list_is_a_usage_error() {
    let sandbox = Sandbox::new("remove_unlisted").with_fake_registry();
    let remove = sandbox.compiler(&["remove", "fake-json"]);
    insta::assert_snapshot!(remove.assert_exit_code(2).stderr());
}

#[test]
fn add_of_a_feature_a_crate_already_has_lists_it_once() {
    let sandbox = Sandbox::new("add_same_feature").with_fake_registry();
    for _ in 0..2 {
        sandbox
            .compiler_with_registry(&["add", "fake-json", "--features", "derive"])
            .assert_silent_success();
    }
    assert!(
        sandbox
            .project_file_text()
            .contains(":features (\"derive\")")
    );
}

#[test]
fn add_of_a_feature_cargo_cannot_find_prints_cargo_text() {
    let sandbox = Sandbox::new("add_unknown_feature").with_fake_registry();
    let add = sandbox.compiler_with_registry(&["add", "fake-json", "--features", "nope"]);
    let stderr = add.assert_exit_code(2).stderr();
    assert!(stderr.contains("nope"), "{stderr}");
    assert_eq!(sandbox.project_file_text(), "(project demo)\n");
}

#[test]
fn remove_keeps_a_comment_before_the_entry() {
    let sandbox = Sandbox::new("remove_after_comment").with_fake_registry().with_file(
        &Language::project_file_name(),
        "(project demo\n  (dependencies ; first\n    (other \"1.0\")\n    ; json\n    (fake-json \"1.4.2\")))\n",
    );
    sandbox
        .compiler(&["remove", "fake-json"])
        .assert_silent_success();
    insta::assert_snapshot!(sandbox.project_file_text());
}

#[test]
fn add_of_a_name_that_is_not_a_crate_name_is_a_usage_error() {
    let sandbox = Sandbox::new("add_bad_name").with_fake_registry();
    for name in ["--path=x", "a b", "fake-json@1.4", "-quiet"] {
        let stderr = sandbox
            .compiler_with_registry(&["add", "--", name])
            .assert_exit_code(2)
            .stderr();
        assert_eq!(stderr, format!("error: `{name}` is not a crate name\n"));
    }
    assert_eq!(sandbox.project_file_text(), "(project demo)\n");
}

#[test]
fn add_of_a_value_that_is_not_a_feature_name_is_a_usage_error() {
    let sandbox = Sandbox::new("add_bad_feature").with_fake_registry();
    for feature in ["derive std", "a/b/c", "/derive", "-x", "--dry-run"] {
        let value = format!("--features={feature}");
        let add = sandbox.compiler_with_registry(&["add", "fake-json", &value]);
        assert_eq!(
            add.assert_exit_code(2).stderr(),
            format!("error: `{feature}` is not a feature name\n")
        );
    }
    sandbox
        .compiler_with_registry(&["add", "fake-json", "--features", "serde/std"])
        .assert_exit_code(2);
}

#[test]
fn add_of_a_crate_listed_with_a_path_is_a_usage_error() {
    let project = "(project demo (dependencies (fake-json :path \"../fake-json\")))\n";
    let sandbox = Sandbox::new("add_path_crate")
        .with_fake_registry()
        .with_file(&Language::project_file_name(), project);
    let add = sandbox.compiler_with_registry(&["add", "fake-json"]);
    insta::assert_snapshot!(add.assert_exit_code(2).stderr());
    assert_eq!(sandbox.project_file_text(), project);
}

#[test]
fn add_without_a_temporary_directory_is_an_environment_error() {
    let sandbox = Sandbox::new("add_no_temporary_directory").with_fake_registry();
    let mut command = sandbox.command(Path::new(Sandbox::COMPILER), &["add", "fake-json"]);
    command
        .env("CARGO_HOME", sandbox.directory.join("cargo-home"))
        .env(
            Sandbox::TEMPORARY_DIRECTORY_VARIABLE,
            sandbox.directory.join("missing"),
        );
    let stderr = Outcome::of(&mut command).assert_exit_code(2).stderr();
    assert!(
        stderr.starts_with("error: cannot write a scratch manifest for cargo: "),
        "{stderr}"
    );
}

#[test]
fn add_to_a_project_in_a_read_only_directory_reports_the_file() {
    let sandbox = Sandbox::new("add_read_only_project").with_fake_registry();
    fs::set_permissions(
        &sandbox.directory,
        fs::Permissions::from_mode(Sandbox::UNWRITABLE_DIRECTORY),
    )
    .expect("the sandbox sets permissions");
    let add = sandbox.compiler_with_registry(&["add", "fake-json"]);
    fs::set_permissions(
        &sandbox.directory,
        fs::Permissions::from_mode(Sandbox::EXECUTABLE),
    )
    .expect("the sandbox sets permissions");
    let stderr = add.assert_exit_code(2).stderr();
    assert!(
        stderr.starts_with(&format!(
            "error: cannot write `{}`: ",
            Language::project_file_name()
        )),
        "{stderr}"
    );
    assert_eq!(sandbox.project_file_text(), "(project demo)\n");
}

#[test]
fn add_writes_the_crate_name_cargo_wrote() {
    let sandbox = Sandbox::new("add_translated_name").with_fake_registry();
    sandbox
        .compiler_with_translating_cargo(&["add", "fake-json"])
        .assert_silent_success();
    assert_eq!(
        sandbox.project_file_text(),
        "(project demo (dependencies (fake_json \"1.0.0\")))\n"
    );
}

#[test]
fn add_refuses_a_path_crate_listed_under_the_name_cargo_wrote() {
    let project = "(project demo (dependencies (fake_json :path \"../fake-json\")))\n";
    let sandbox = Sandbox::new("add_translated_path_crate")
        .with_fake_registry()
        .with_file(&Language::project_file_name(), project);
    let stderr = sandbox
        .compiler_with_translating_cargo(&["add", "fake-json"])
        .assert_exit_code(2)
        .stderr();
    assert!(
        stderr.starts_with("error: `fake_json` is listed with `:path`"),
        "{stderr}"
    );
    assert_eq!(sandbox.project_file_text(), project);
}

#[test]
fn add_keeps_the_project_file_link_and_mode() {
    let sandbox = Sandbox::new("add_linked_project").with_fake_registry();
    let real = sandbox.directory.join("real-project.knv");
    fs::rename(sandbox.directory.join(Language::project_file_name()), &real)
        .expect("the sandbox renames files");
    std::os::unix::fs::symlink(
        "real-project.knv",
        sandbox.directory.join(Language::project_file_name()),
    )
    .expect("the sandbox accepts symbolic links");
    fs::set_permissions(&real, fs::Permissions::from_mode(Sandbox::PRIVATE_FILE))
        .expect("the sandbox sets permissions");
    sandbox
        .compiler_with_registry(&["add", "fake-json"])
        .assert_silent_success();
    let link = fs::symlink_metadata(sandbox.directory.join(Language::project_file_name()))
        .expect("the link stays");
    assert!(link.file_type().is_symlink());
    let mode = fs::metadata(&real)
        .expect("the file stays")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, Sandbox::PRIVATE_FILE);
    assert!(
        fs::read_to_string(&real)
            .expect("the file is readable")
            .contains("fake-json")
    );
}

#[test]
fn add_when_cargo_writes_no_version_is_an_environment_error() {
    let sandbox = Sandbox::new("add_versionless").with_fake_registry();
    let add = sandbox.compiler_with_fake_cargo(Sandbox::VERSIONLESS_CARGO, &["add", "fake-json"]);
    assert_eq!(
        add.assert_exit_code(2).stderr(),
        "error: cargo added `fake-json` without a version\n"
    );
    assert_eq!(sandbox.project_file_text(), "(project demo)\n");
}

#[test]
fn a_crate_build_puts_cargo_output_in_the_shared_target() {
    let sandbox = Sandbox::new("crate_shared_target").with_fixture_tree("crates");
    sandbox.compiler_in("app", &["build"]).assert_exit_code(0);
    let shared = sandbox
        .directory
        .join(Sandbox::CACHE_DIRECTORY)
        .join(Language::NAME)
        .join("target");
    assert!(shared.join("debug").is_dir());
    let build_directory = sandbox
        .directory
        .join("app/target")
        .join(Language::NAME)
        .join("app");
    assert!(build_directory.join("Cargo.toml").is_file());
    assert!(!build_directory.join("target").exists());
}

#[test]
fn a_user_cargo_target_dir_wins_over_the_shared_target() {
    let sandbox = Sandbox::new("crate_own_target").with_fixture_tree("crates");
    let mut command = sandbox.command(Path::new(Sandbox::COMPILER), &["build"]);
    command.current_dir(sandbox.directory.join("app")).env(
        Sandbox::CARGO_TARGET_VARIABLE,
        sandbox.directory.join("own-target"),
    );
    Outcome::of(&mut command).assert_exit_code(0);
    assert!(sandbox.directory.join("own-target/debug").is_dir());
    assert!(
        !sandbox
            .directory
            .join(Sandbox::CACHE_DIRECTORY)
            .join(Language::NAME)
            .join("target")
            .exists()
    );
}

#[test]
fn a_crate_build_waiting_on_another_build_says_so() {
    let sandbox = Sandbox::new("crate_waiting").with_fixture_tree("crates");
    let fake_cargo = sandbox.directory.join("waiting-bin/cargo");
    fs::create_dir_all(fake_cargo.parent().expect("the fake cargo has a directory"))
        .expect("the sandbox accepts directories");
    fs::write(&fake_cargo, Sandbox::WAITING_CARGO).expect("the sandbox accepts files");
    fs::set_permissions(&fake_cargo, fs::Permissions::from_mode(Sandbox::EXECUTABLE))
        .expect("the sandbox sets permissions");
    let mut command = sandbox.command(Path::new(Sandbox::COMPILER), &["build"]);
    command
        .current_dir(sandbox.directory.join("app"))
        .env("PATH", sandbox.directory.join("waiting-bin"));
    let stderr = Outcome::of(&mut command).stderr();
    let shown = sandbox.masked(&stderr);
    let expected = format!(
        "note: waiting for another build in `<sandbox>/cache/{}/target`",
        Language::NAME
    );
    assert!(shown.contains(&expected), "{shown}");
}

#[test]
fn a_user_cargo_build_target_dir_wins_over_the_shared_target() {
    let sandbox = Sandbox::new("crate_own_build_target").with_fixture_tree("crates");
    let mut command = sandbox.command(Path::new(Sandbox::COMPILER), &["build"]);
    command.current_dir(sandbox.directory.join("app")).env(
        Sandbox::CARGO_BUILD_TARGET_VARIABLE,
        sandbox.directory.join("own-target"),
    );
    Outcome::of(&mut command).assert_exit_code(0);
    assert!(sandbox.directory.join("own-target/debug").is_dir());
    assert!(
        !sandbox
            .directory
            .join(Sandbox::CACHE_DIRECTORY)
            .join(Language::NAME)
            .join("target")
            .exists()
    );
}

#[test]
fn a_relative_cache_variable_is_ignored() {
    let sandbox = Sandbox::new("crate_relative_cache").with_fixture_tree("crates");
    let mut command = sandbox.command(Path::new(Sandbox::COMPILER), &["build"]);
    command
        .current_dir(sandbox.directory.join("app"))
        .env(Sandbox::CACHE_HOME_VARIABLE, "relative-cache")
        .env("HOME", sandbox.directory.join("home"));
    Outcome::of(&mut command).assert_exit_code(0);
    let home_target = sandbox
        .directory
        .join("home/.cache")
        .join(Language::NAME)
        .join("target");
    assert!(home_target.join("debug").is_dir());
    let build_directory = sandbox
        .directory
        .join("app/target")
        .join(Language::NAME)
        .join("app");
    assert!(!build_directory.join("relative-cache").exists());
}

#[test]
fn test_calls_only_the_project_own_tests() {
    let sandbox = Sandbox::new("test_own_tests").with_fixture_tree("libraries");
    let test = sandbox.compiler_in("own-tests-app", &["test"]);
    assert_eq!(
        test.assert_exit_code(0).stdout(),
        "running 1 test\ntest app/uses-double ... ok\n1 passed, 0 failed\n"
    );
}

#[test]
fn test_of_a_module_that_asks_for_library_examples_runs_only_its_own_tests() {
    let sandbox = Sandbox::new("test_asks_library").with_fixture_tree("libraries");
    let test = sandbox.compiler_in("asks-library-app", &["test"]);
    assert_eq!(
        test.assert_exit_code(0).stdout(),
        "running 1 test\ntest app/runs ... ok\n1 passed, 0 failed\n"
    );
}

#[test]
fn test_checks_the_docstring_examples_of_a_library() {
    let sandbox = Sandbox::new("test_library_doc").with_fixture_tree("libraries");
    let test = sandbox.compiler_in("doc-checked-app", &["test"]);
    assert!(
        test.assert_exit_code(1)
            .stderr()
            .starts_with("error[L0401]: mismatched types\n --> ../doc-checked/src/lib.knv:6:10\n")
    );
}

#[test]
fn library_examples_check_without_emitting_or_checking_the_project() {
    let sandbox = Sandbox::new("library_examples_emit").with_fixture_tree("libraries");
    let emit = sandbox.compiler_in("library-examples-app", &["emit"]);
    let emitted = emit.assert_exit_code(0).stdout();
    assert!(!emitted.contains("check_equal"), "{emitted}");
    assert!(!emitted.contains("|| {"), "{emitted}");
}

#[test]
fn run_tests_from_a_program_main_checks_only_project_examples() {
    let sandbox = Sandbox::new("own_runner_check").with_fixture_tree("libraries");
    sandbox
        .compiler_in("own-runner-app", &["check"])
        .assert_exit_code(0);
}

#[test]
fn test_reports_library_and_project_example_errors_together() {
    let sandbox = Sandbox::new("test_example_errors").with_fixture_tree("libraries");
    let test = sandbox.compiler_in("unbound-example-app", &["test"]);
    let stderr = test.assert_exit_code(1).stderr();
    assert!(
        stderr.contains("cannot find `absent-in-project` in this scope\n --> tests/app.knv:7:4\n"),
        "{stderr}"
    );
    assert!(
        stderr.contains(
            "cannot find `absent-in-library` in this scope\n --> ../unbound-example/src/lib.knv:6:4\n"
        ),
        "{stderr}"
    );
}

#[test]
fn test_checks_the_tests_of_a_library() {
    let sandbox = Sandbox::new("test_library_test").with_fixture_tree("libraries");
    let test = sandbox.compiler_in("test-checked-app", &["test"]);
    assert!(
        test.assert_exit_code(1)
            .stderr()
            .starts_with("error[L0401]: mismatched types\n --> ../test-checked/src/lib.knv:8:22\n")
    );
}

const TASK_PROJECT: &str = "(project chores)\n";
const CYCLE_FIRST_TASK: &str = "(use second)\n\n(deftask first () (println 1))\n";
const CYCLE_SECOND_TASK: &str = "(use first)\n\n(deftask second () (println 2))\n";
const ROOT_ENTRY_PROJECT: &str = "(project chores :entry \"main.knv\")\n";
const ROOT_ENTRY: &str = "(defn main () (println \"m\"))\n";
const HELPER_TASK: &str = "(export loud)\n\n(defn loud (text) (str text \"!\"))\n\n(deftask shout (who) (println (loud who)))\n";
const HELPER_TASK_TEST: &str = "(use prelude/test assert deftest)\n(use tasks/build loud)\n\n(deftest louder (assert (= (loud \"a\") \"a!\")))\n";
const TASK_MODULE: &str = "(export greeting)\n\n(defn greeting (name) (str \"hello \" name))\n\n(defn main () (println (greeting \"main\")))\n";
const TASKS: &str = "(use main greeting)

(deftask greet ((who string) :times u16 1 :loud false)
  \"Greets someone a few times.\"
  (println (greeting who) \" \" times \" \" loud))

(deftask where ()
  \"Prints the working directory.\"
  (println (working-directory)))

(deftask leave ((code i32))
  (exit code))

(deftask count (:limit u16)
  (println limit))

(deftask paint (:color :coats 2)
  (println (as-str color) \" \" (+ coats 1)))

(deftask clean-up2 (:label \"none\" :shown true)
  (println label \" \" shown))

(def working-directory
  (rust \"std::env::current_dir().unwrap().display().to_string()\"
    (fn () string)))
";
const MISNAMED_TASK: &str = "(deftask Tidy_up () (println \"x\"))\n";
const LATE_MISNAMED_TASK: &str = "(deftask tidy_up () (println \"x\"))\n";
const DOUBLE_DASHED_TASK: &str = "(deftask tidy--up () (println \"x\"))\n";
const REPEATED_TASK: &str = "(deftask where () (println \"again\"))\n";
const MISTYPED_TASK: &str = "(deftask sum (:by u16) (println (+ by \"x\")))\n";
const FLAG_BEFORE_POSITIONAL_TASK: &str = "(deftask mix (:fast u16 who) (println who))\n";
const FAILING_TASK: &str = "(deftask refuse () (err (str \"refused\")))\n";

impl Sandbox {
    fn with_tasks(test_name: &str) -> Self {
        Self::new(test_name)
            .with_file(&Language::project_file_name(), TASK_PROJECT)
            .with_file("src/main.knv", TASK_MODULE)
            .with_file("tasks/chores.knv", TASKS)
    }
}

#[test]
fn task_without_a_name_lists_the_tasks_with_their_parameters_and_summaries() {
    let listing = Sandbox::with_tasks("task_listing").compiler_in("src", &["task"]);
    insta::assert_snapshot!(listing.assert_exit_code(0).stdout());
}

#[test]
fn task_binds_positional_arguments_flags_and_switches() {
    let sandbox = Sandbox::with_tasks("task_arguments");
    let given = sandbox.compiler(&["task", "greet", "ana", "--loud", "--times", "3"]);
    let defaulted = sandbox.compiler(&["task", "greet", "ana"]);
    assert_eq!(
        [
            given.assert_exit_code(0).stdout(),
            defaulted.assert_exit_code(0).stdout()
        ],
        ["hello ana 3 true\n", "hello ana 1 false\n"]
    );
}

#[test]
fn task_runs_at_the_project_root_from_a_subdirectory() {
    let sandbox = Sandbox::with_tasks("task_root");
    let root = fs::canonicalize(&sandbox.directory).expect("the sandbox has a canonical path");
    let task = sandbox.compiler_in("src", &["task", "where"]);
    assert_eq!(
        task.assert_exit_code(0).stdout(),
        format!("{}\n", root.display())
    );
}

#[test]
fn task_exits_with_the_tasks_exit_code() {
    let task = Sandbox::with_tasks("task_exit_code").compiler(&["task", "leave", "7"]);
    task.assert_exit_code(7);
}

#[test]
fn task_that_returns_an_err_fails_as_main_does() {
    let task = Sandbox::with_tasks("task_err")
        .with_file("tasks/failing.knv", FAILING_TASK)
        .compiler(&["task", "refuse"]);
    insta::assert_snapshot!(task.assert_exit_code(1).stderr());
}

#[test]
fn task_command_line_errors_name_the_parameter_and_exit_two() {
    let sandbox = Sandbox::with_tasks("task_command_line_errors");
    let errors: Vec<String> = [
        vec!["task", "greet"],
        vec!["task", "greet", "ana", "bo"],
        vec!["task", "greet", "ana", "--shout"],
        vec!["task", "greet", "ana", "--times"],
        vec!["task", "greet", "ana", "--times", "many"],
        vec!["task", "count"],
    ]
    .iter()
    .map(|arguments| sandbox.compiler(arguments).assert_exit_code(2).stderr())
    .collect();
    insta::assert_snapshot!(errors.join("---\n"));
}

#[test]
fn task_with_an_unknown_name_suggests_the_closest_task() {
    let task = Sandbox::with_tasks("task_unknown").compiler(&["task", "gret"]);
    insta::assert_snapshot!(task.assert_exit_code(2).stderr());
}

#[test]
fn task_with_an_unknown_name_far_from_every_task_says_how_to_list_them() {
    let task = Sandbox::with_tasks("task_unknown_far").compiler(&["task", "deploy-everything"]);
    insta::assert_snapshot!(task.assert_exit_code(2).stderr());
}

#[test]
fn task_in_a_project_without_tasks_directory_is_a_usage_error() {
    let task = Sandbox::new("task_no_directory")
        .with_file(&Language::project_file_name(), TASK_PROJECT)
        .with_file("src/main.knv", TASK_MODULE)
        .compiler(&["task"]);
    insta::assert_snapshot!(task.assert_exit_code(2).stderr());
}

#[test]
fn task_outside_a_project_is_a_usage_error() {
    let task = Sandbox::new("task_outside_project").compiler(&["task", "greet"]);
    insta::assert_snapshot!(task.assert_exit_code(2).stderr());
}

#[test]
fn task_with_a_name_that_is_not_kebab_case_is_an_error() {
    let task = Sandbox::with_tasks("task_misnamed")
        .with_file("tasks/misnamed.knv", MISNAMED_TASK)
        .compiler(&["task"]);
    insta::assert_snapshot!(task.assert_exit_code(1).stderr());
}

#[test]
fn task_defined_twice_is_an_error() {
    let task = Sandbox::with_tasks("task_repeated")
        .with_file("tasks/repeated.knv", REPEATED_TASK)
        .compiler(&["task"]);
    insta::assert_snapshot!(task.assert_exit_code(1).stderr());
}

#[test]
fn task_modules_that_import_each_other_name_the_tasks_entry_in_the_cycle() {
    let task = Sandbox::with_tasks("task_cycle")
        .with_file("tasks/first.knv", CYCLE_FIRST_TASK)
        .with_file("tasks/second.knv", CYCLE_SECOND_TASK)
        .compiler(&["task"]);
    insta::assert_snapshot!(task.assert_exit_code(1).stderr());
}

#[test]
fn test_can_use_a_task_module_beside_a_root_entry() {
    let sandbox = Sandbox::new("test_task_helper")
        .with_file(&Language::project_file_name(), ROOT_ENTRY_PROJECT)
        .with_file("main.knv", ROOT_ENTRY)
        .with_file("tasks/build.knv", HELPER_TASK)
        .with_file("tests/build_test.knv", HELPER_TASK_TEST);
    let test = sandbox.compiler(&["test"]);
    assert_eq!(
        test.assert_silent_success().stdout(),
        "running 1 test\ntest tests/build_test/louder ... ok\n1 passed, 0 failed\n"
    );
}

#[test]
fn check_without_a_file_checks_every_task() {
    let sandbox = Sandbox::with_tasks("check_tasks");
    let clean = sandbox.compiler(&["check"]);
    clean.assert_exit_code(0);
    let mistyped = sandbox
        .with_file("tasks/mistyped.knv", MISTYPED_TASK)
        .compiler(&["check"]);
    insta::assert_snapshot!(mistyped.assert_exit_code(1).stderr());
}

#[test]
fn check_of_a_task_file_resolves_use_from_the_source_root() {
    let check = Sandbox::with_tasks("check_task_file").compiler(&["check", "tasks/chores.knv"]);
    check.assert_exit_code(0);
}

const CRATE_TASK: &str = "(def answer
  (rust \"answer::answer()\" (fn () i64)))

(deftask show (:offset i64 2)
  (println (+ (answer) offset)))
";

#[test]
fn task_in_a_project_with_crates_builds_through_cargo_beside_the_program() {
    let sandbox = Sandbox::new("crate_task")
        .with_fixture_tree("crates")
        .with_file("app/tasks/show.knv", CRATE_TASK);
    let task = sandbox.compiler_in("app", &["task", "show"]);
    let run = sandbox.compiler_in("app", &["run"]);
    assert_eq!(
        [
            task.assert_exit_code(0).stdout(),
            run.assert_exit_code(0).stdout()
        ],
        ["44\n", "42\n"]
    );
}

#[test]
fn task_with_an_unknown_name_in_a_project_with_no_tasks_says_none_is_defined() {
    let task = Sandbox::new("task_none_defined")
        .with_file(&Language::project_file_name(), TASK_PROJECT)
        .with_file("src/main.knv", TASK_MODULE)
        .with_file("tasks/notes.knv", "(def note 1)\n")
        .compiler(&["task", "greet"]);
    insta::assert_snapshot!(task.assert_exit_code(2).stderr());
}

#[test]
fn check_without_a_file_in_a_project_without_tasks_checks_the_entry() {
    let check = Sandbox::new("check_no_tasks_directory")
        .with_file(&Language::project_file_name(), TASK_PROJECT)
        .with_file("src/main.knv", TASK_MODULE)
        .compiler(&["check"]);
    check.assert_silent_success();
}

#[test]
fn task_list_ends_each_line_and_prints_nothing_without_tasks() {
    let listing = Sandbox::with_tasks("task_listing_lines").compiler(&["task"]);
    let empty = Sandbox::new("task_listing_empty")
        .with_file(&Language::project_file_name(), TASK_PROJECT)
        .with_file("src/main.knv", TASK_MODULE)
        .with_file("tasks/notes.knv", "(def note 1)\n")
        .compiler(&["task"]);
    assert_eq!(
        [
            listing.assert_exit_code(0).stdout().lines().count(),
            empty.assert_exit_code(0).stdout().len()
        ],
        [6, 0]
    );
    assert!(
        listing
            .stdout()
            .ends_with("Prints the working directory.\n")
    );
}

#[test]
fn task_flag_takes_a_string_or_true_default() {
    let sandbox = Sandbox::with_tasks("task_true_default");
    let defaulted = sandbox.compiler(&["task", "clean-up2"]);
    let given = sandbox.compiler(&[
        "task",
        "clean-up2",
        "--label",
        "kitchen",
        "--shown",
        "false",
    ]);
    assert_eq!(
        [
            defaulted.assert_exit_code(0).stdout(),
            given.assert_exit_code(0).stdout()
        ],
        ["none true\n", "kitchen false\n"]
    );
}

#[test]
fn task_name_breaks_kebab_case_after_its_first_letter() {
    let sandbox = Sandbox::with_tasks("task_late_misnamed");
    let underscored = sandbox
        .with_file("tasks/misnamed.knv", LATE_MISNAMED_TASK)
        .compiler(&["task"]);
    let sandbox = Sandbox::with_tasks("task_double_dashed");
    let double_dashed = sandbox
        .with_file("tasks/misnamed.knv", DOUBLE_DASHED_TASK)
        .compiler(&["task"]);
    assert_eq!(
        [
            underscored.assert_exit_code(1).stderr().lines().next(),
            double_dashed.assert_exit_code(1).stderr().lines().next()
        ],
        [
            Some("error[L0357]: task name `tidy_up` is not kebab-case"),
            Some("error[L0357]: task name `tidy--up` is not kebab-case")
        ]
    );
}

#[test]
fn task_flag_without_type_or_default_is_required_beside_a_defaulted_flag() {
    let sandbox = Sandbox::with_tasks("task_flat_flags");
    let painted = sandbox.compiler(&["task", "paint", "--color", "teal"]);
    let missing = sandbox.compiler(&["task", "paint", "--coats", "3"]);
    assert_eq!(
        [
            painted.assert_exit_code(0).stdout(),
            missing.assert_exit_code(2).stderr()
        ],
        [
            "teal 3\n",
            "error: task `paint` needs `--color`\nusage: paint --color <color> [--coats 2]\n"
        ]
    );
}

#[test]
fn task_positional_parameter_after_a_flag_is_an_error() {
    let task = Sandbox::with_tasks("task_flag_before_positional")
        .with_file("tasks/mix.knv", FLAG_BEFORE_POSITIONAL_TASK)
        .compiler(&["task"]);
    insta::assert_snapshot!(task.assert_exit_code(1).stderr());
}

const INFERRED_TASKS: &str = "(defn double ((port u16)) (* port 2))

(defn square (x) (* x x))

(deftask pinned (port) (println (double port)))

(deftask hail (who) (println who))

(deftask area (side) (println (square side)))
";
const SIGNATURE_LOOKUP: &str = "(use prelude/task task-signature)

(deftask known ((port u16) :times 2 :quiet false) (println port times quiet))

(deftask open (who) (println who))
";
const OPEN_LOCAL: &str = "(defn main ()\n  (local x (parse \"1\"))\n  (println \"ok\"))\n";

#[test]
fn task_parameter_that_nothing_fixes_takes_a_string() {
    let task = Sandbox::with_tasks("task_open_parameter")
        .with_file("tasks/inferred.knv", INFERRED_TASKS)
        .compiler(&["task", "hail", "ana"]);
    assert_eq!(task.assert_exit_code(0).stdout(), "ana\n");
}

#[test]
fn task_parameter_keeps_the_type_inference_gives_it() {
    let sandbox = Sandbox::with_tasks("task_pinned_parameter")
        .with_file("tasks/inferred.knv", INFERRED_TASKS);
    let doubled = sandbox.compiler(&["task", "pinned", "21"]);
    let too_big = sandbox.compiler(&["task", "pinned", "70000"]);
    assert_eq!(
        [
            doubled.assert_exit_code(0).stdout(),
            too_big.assert_exit_code(2).stderr()
        ],
        [
            "42\n",
            "error: task `pinned` cannot read `70000` as `<port>`\n"
        ]
    );
}

#[test]
fn task_parameter_whose_body_needs_more_than_a_string_is_a_type_error() {
    let task = Sandbox::with_tasks("task_bound_string")
        .with_file("tasks/inferred.knv", INFERRED_TASKS)
        .compiler(&["task", "area", "3"]);
    insta::assert_snapshot!(task.assert_exit_code(1).stderr());
}

#[test]
fn task_signature_shows_each_task_type_as_inferred() {
    let sandbox =
        Sandbox::with_tasks("task_signature").with_file("tasks/lookup.knv", SIGNATURE_LOOKUP);
    let known = sandbox.compiler(&["eval", "tasks/lookup.knv", "(task-signature \"known\")"]);
    let open = sandbox.compiler(&["eval", "tasks/lookup.knv", "(task-signature \"open\")"]);
    insta::assert_snapshot!(format!(
        "{}{}",
        known.assert_exit_code(0).stdout(),
        open.assert_exit_code(0).stdout()
    ));
}

#[test]
fn check_keeps_an_open_local_outside_tasks_an_error() {
    let check = Sandbox::new("open_local")
        .with_file("open.knv", OPEN_LOCAL)
        .compiler(&["check", "open.knv"]);
    insta::assert_snapshot!(check.assert_exit_code(1).stderr());
}

const TRAIT_MODULE: &str = "(export greet-text loud loud-text)

(defn greet-text ((name (ref str))) (println name))

(deftrait loud (loud-text (fn ((ref self)) string)))

(impl loud string (loud-text (s) (str s \"!\")))

(defn main () (println \"main\"))
";
const ONE_TASK_CHOICES: &str = "(use main greet-text loud-text)

(defn square (x) (* x x))

(deftask total ((xs (vec i64))) (println (len xs)))

(deftask hi (who) (greet-text who))

(deftask add ((a i64)) (+ a 1))

(deftask docless () \"only a docstring\")

(deftask d (:port u16 -1) (println port))

(deftask area (side) (println (square side)))

(deftask shout (who) (println (loud-text who)))

(deftask mixed (who :times 2) (println who \" \" (+ times 1)))
";
const STRAY_TASK_MODULE: &str = "(export x)\n\n(def x 1)\n\n(deftask sneaky () (println \"s\"))\n";
const STRAY_TASK_USER: &str = "(use stray x)\n\n(deftask t () (println x))\n";
const SHARED_BROKEN: &str = "(export helper)\n\n(defn helper () (println (+ 1 \"x\")))\n";

impl Sandbox {
    fn with_one_task(test_name: &str, task: &str) -> Self {
        Self::new(test_name)
            .with_file(&Language::project_file_name(), TASK_PROJECT)
            .with_file("src/main.knv", TRAIT_MODULE)
            .with_file(&format!("tasks/{task}.knv"), &Self::only_task(task))
    }

    fn only_task(task: &str) -> String {
        let header = ONE_TASK_CHOICES
            .split("\n\n(deftask")
            .next()
            .expect("the tasks start with uses");
        let body = ONE_TASK_CHOICES
            .split("\n\n")
            .find(|block| block.starts_with(&format!("(deftask {task} ")))
            .expect("the task exists");
        format!("{header}\n\n{body}\n")
    }
}

#[test]
fn task_errors_point_at_the_parameter_or_the_task_never_at_generated_code() {
    let reports: Vec<String> = ["total", "hi", "add", "docless", "d", "area"]
        .iter()
        .map(|task| {
            Sandbox::with_one_task(&format!("task_located_{task}"), task)
                .compiler(&["task", task])
                .assert_exit_code(1)
                .stderr()
        })
        .collect();
    assert!(reports.iter().all(|report| !report.contains("<task")));
    insta::assert_snapshot!(reports.join("---\n"));
}

#[test]
fn task_string_parameter_reaches_a_user_trait_that_string_implements() {
    let task =
        Sandbox::with_one_task("task_user_trait", "shout").compiler(&["task", "shout", "ana"]);
    assert_eq!(task.assert_exit_code(0).stdout(), "ana!\n");
}

#[test]
fn task_open_parameter_beside_a_defaulted_flag_keeps_the_flag_a_number() {
    let task = Sandbox::with_one_task("task_open_beside_default", "mixed")
        .compiler(&["task", "mixed", "bo", "--times", "3"]);
    assert_eq!(task.assert_exit_code(0).stdout(), "bo 4\n");
}

#[test]
fn check_without_a_file_rejects_what_task_rejects() {
    let check = Sandbox::with_one_task("check_rejects_task", "add").compiler(&["check"]);
    insta::assert_snapshot!(check.assert_exit_code(1).stderr());
}

#[test]
fn check_without_a_file_reports_an_error_shared_by_entry_and_task_once() {
    let check = Sandbox::new("check_shared_error")
        .with_file(&Language::project_file_name(), TASK_PROJECT)
        .with_file("src/shared.knv", SHARED_BROKEN)
        .with_file(
            "src/main.knv",
            "(use shared helper)\n\n(defn main () (helper))\n",
        )
        .with_file(
            "tasks/t.knv",
            "(use shared helper)\n\n(deftask t () (helper))\n",
        )
        .compiler(&["check"]);
    assert_eq!(
        check
            .assert_exit_code(1)
            .stderr()
            .matches("mismatched types")
            .count(),
        1
    );
}

#[test]
fn task_cannot_reach_tests_when_the_entry_sits_at_the_root() {
    let sandbox = Sandbox::new("task_root_entry")
        .with_file(
            &Language::project_file_name(),
            "(project rootreach :entry \"main.knv\")\n",
        )
        .with_file(
            "main.knv",
            "(export shout)\n\n(def shout \"hey\")\n\n(defn main () (println shout))\n",
        )
        .with_file(
            "tests/support.knv",
            "(export helper)\n\n(defn helper () (println \"t\"))\n",
        )
        .with_file(
            "tasks/u.knv",
            "(use main shout)\n\n(deftask u () (println shout))\n",
        );
    let root_module = sandbox.compiler(&["task", "u"]);
    let tests_module = sandbox
        .with_file(
            "tasks/t.knv",
            "(use tests/support helper)\n\n(deftask t () (helper))\n",
        )
        .compiler(&["task", "t"]);
    assert_eq!(root_module.assert_exit_code(0).stdout(), "hey\n");
    insta::assert_snapshot!(tests_module.assert_exit_code(1).stderr());
}

#[test]
fn task_parses_flag_shapes_the_way_command_lines_write_them() {
    let sandbox = Sandbox::with_tasks("task_flag_shapes");
    let outcomes: Vec<String> = [
        vec!["task", "greet", "ana", "--times=3"],
        vec!["task", "greet", "ana", "--loud=true"],
        vec!["task", "greet", "--", "--ana"],
        vec!["task", "greet", "ana", "--times", "1", "--times", "2"],
        vec!["task", "greet", "--help"],
    ]
    .iter()
    .map(|arguments| {
        let outcome = sandbox.compiler(arguments);
        format!(
            "{:?}\n{}{}",
            outcome.output.status.code(),
            outcome.stdout(),
            outcome.stderr()
        )
    })
    .collect();
    insta::assert_snapshot!(outcomes.join("---\n"));
}

#[test]
fn task_refuses_a_task_file_that_takes_a_compiler_module_path() {
    let task = Sandbox::with_tasks("task_compiler_module_paths")
        .with_file(
            "tasks/kernel.knv",
            "(deftask from-kernel () (println \"ran\"))\n",
        )
        .with_file(
            "tasks/prelude.knv",
            "(deftask from-prelude () (println \"ran\"))\n",
        )
        .compiler(&["task"]);
    insta::assert_snapshot!(task.assert_exit_code(1).stderr());
}

#[test]
fn task_deftask_outside_tasks_is_an_error_at_the_form() {
    let task = Sandbox::with_tasks("task_stray")
        .with_file("src/stray.knv", STRAY_TASK_MODULE)
        .with_file("tasks/stray-user.knv", STRAY_TASK_USER)
        .compiler(&["task"]);
    insta::assert_snapshot!(task.assert_exit_code(1).stderr());
}

#[test]
fn task_without_tasks_directory_names_it_relative_from_a_subdirectory() {
    let task = Sandbox::new("task_no_directory_subdirectory")
        .with_file(&Language::project_file_name(), TASK_PROJECT)
        .with_file("src/main.knv", TASK_MODULE)
        .compiler_in("src", &["task"]);
    insta::assert_snapshot!(task.assert_exit_code(2).stderr());
}

#[test]
fn task_file_needs_no_import_of_deftask() {
    let task = Sandbox::new("task_no_import")
        .with_file(&Language::project_file_name(), TASK_PROJECT)
        .with_file("src/main.knv", TASK_MODULE)
        .with_file(
            "tasks/plain.knv",
            "(deftask plain () (println \"plain\"))\n",
        )
        .compiler(&["task", "plain"]);
    assert_eq!(task.assert_exit_code(0).stdout(), "plain\n");
}

const OWN_DEFINE_TASK: &str = "(defn define-task (x) (+ x 1))

(deftask one () (println (define-task 1)))

(deftask two () (println (define-task 2)))
";

#[test]
fn task_expansion_leaves_the_name_define_task_to_the_program() {
    let sandbox = Sandbox::new("task_own_define_task")
        .with_file(&Language::project_file_name(), TASK_PROJECT)
        .with_file("src/main.knv", TASK_MODULE)
        .with_file("tasks/own.knv", OWN_DEFINE_TASK);
    let own = sandbox.compiler(&["task", "two"]);
    let unbound = sandbox
        .with_file(
            "tasks/bare.knv",
            "(deftask three () (println (define-task 3)))\n",
        )
        .compiler(&["task", "three"]);
    assert_eq!(own.assert_exit_code(0).stdout(), "3\n");
    assert!(
        unbound
            .assert_exit_code(1)
            .stderr()
            .starts_with("error[L0201]: cannot find `define-task` in this scope")
    );
}

#[test]
fn emitted_task_program() {
    let sandbox = Sandbox::with_tasks("emitted_task_program");
    sandbox
        .compiler(&["task", "greet", "ana"])
        .assert_exit_code(0);
    let program = sandbox.contents("target/knave/chores/tasks/greet/main.rs");
    insta::assert_snapshot!(program);
}

const PARAMETER_TASKS: &str = "(defn double ((port u16)) (* port 2))

(deftask shown (who :times 2) (println who \" \" times))

(deftask doubled (who port) (println who \" \" (double port)))

(deftask build-2024 () (println \"built\"))

(deftask fails () (err \"boom\"))

(deftask told (who :times 1 :loud false)
  \"Says hello.

  Says it a few times.\"
  (println who \" \" times \" \" loud))
";

impl Sandbox {
    fn with_parameter_tasks(test_name: &str) -> Self {
        Self::new(test_name)
            .with_file(&Language::project_file_name(), TASK_PROJECT)
            .with_file("src/main.knv", TASK_MODULE)
            .with_file("tasks/round.knv", PARAMETER_TASKS)
    }
}

#[test]
fn task_open_parameter_beside_a_display_only_defaulted_flag_keeps_its_default_type() {
    let task =
        Sandbox::with_parameter_tasks("task_display_default").compiler(&["task", "shown", "bo"]);
    assert_eq!(task.assert_exit_code(0).stdout(), "bo 2\n");
}

#[test]
fn task_open_parameter_beside_a_concrete_parameter_leaves_the_concrete_one_alone() {
    let task = Sandbox::with_parameter_tasks("task_concrete_beside_open")
        .compiler(&["task", "doubled", "bo", "21"]);
    assert_eq!(task.assert_exit_code(0).stdout(), "bo 42\n");
}

#[test]
fn task_name_may_hold_a_word_that_starts_with_a_digit() {
    let task = Sandbox::with_parameter_tasks("task_digit_word").compiler(&["task", "build-2024"]);
    assert_eq!(task.assert_exit_code(0).stdout(), "built\n");
}

#[test]
fn check_without_a_file_accepts_a_task_that_only_fails() {
    let sandbox = Sandbox::with_parameter_tasks("check_only_fails");
    sandbox.compiler(&["check"]).assert_exit_code(0);
    let task = sandbox.compiler(&["task", "fails"]);
    assert_eq!(task.assert_exit_code(1).stderr(), "boom\n");
}

#[test]
fn task_with_an_open_error_type_is_refused_at_its_name_under_task_and_check() {
    let sandbox = Sandbox::new("task_open_error")
        .with_file(&Language::project_file_name(), TASK_PROJECT)
        .with_file("src/main.knv", TASK_MODULE)
        .with_file("tasks/k.knv", "(deftask k () (ok (do)))\n");
    let task = sandbox
        .compiler(&["task", "k"])
        .assert_exit_code(1)
        .stderr();
    let check = sandbox.compiler(&["check"]).assert_exit_code(1).stderr();
    assert_eq!(task, check);
    insta::assert_snapshot!(task);
}

#[test]
fn task_help_prints_the_usage_and_the_docstring() {
    let task =
        Sandbox::with_parameter_tasks("task_help_docstring").compiler(&["task", "told", "--help"]);
    insta::assert_snapshot!(task.assert_exit_code(0).stdout());
}

#[test]
fn task_reports_a_switch_value_and_a_flag_without_value_clearly() {
    let sandbox = Sandbox::with_parameter_tasks("task_switch_and_flag_values");
    let switch_value = sandbox.compiler(&["task", "told", "bo", "--loud", "true"]);
    let flag_then_flag = sandbox.compiler(&["task", "told", "bo", "--times", "--loud"]);
    insta::assert_snapshot!(format!(
        "{}---\n{}",
        switch_value.assert_exit_code(2).stderr(),
        flag_then_flag.assert_exit_code(2).stderr()
    ));
}

#[test]
fn task_definition_errors_have_their_own_codes() {
    let reports: Vec<String> = [
        ("help_flag", "(deftask hv (:help \"x\") (println help))\n"),
        ("docless", "(deftask docless () \"only a docstring\")\n"),
    ]
    .iter()
    .map(|(name, source)| {
        Sandbox::new(&format!("task_definition_{name}"))
            .with_file(&Language::project_file_name(), TASK_PROJECT)
            .with_file("src/main.knv", TASK_MODULE)
            .with_file("tasks/t.knv", source)
            .compiler(&["task"])
            .assert_exit_code(1)
            .stderr()
    })
    .collect();
    insta::assert_snapshot!(reports.join("---\n"));
}

#[test]
fn deftask_in_an_entry_module_is_an_error_under_run_and_check() {
    let sandbox = Sandbox::new("deftask_in_main")
        .with_file(&Language::project_file_name(), TASK_PROJECT)
        .with_file(
            "src/main.knv",
            "(deftask hidden () (println \"h\"))\n\n(defn main () (println \"m\"))\n",
        );
    let check = sandbox.compiler(&["check"]).assert_exit_code(1).stderr();
    let run = sandbox.compiler(&["run"]).assert_exit_code(1).stderr();
    assert_eq!(check, run);
    insta::assert_snapshot!(check);
}

#[test]
fn task_cannot_reach_tests_through_a_dotted_entry_a_parent_path_or_a_link() {
    let support = "(export helper)\n\n(defn helper () (println \"from tests\"))\n";
    let dotted = Sandbox::new("task_tests_dotted")
        .with_file(
            &Language::project_file_name(),
            "(project dotted :entry \"./main.knv\")\n",
        )
        .with_file("main.knv", "(defn main () (println \"m\"))\n")
        .with_file("tests/support.knv", support)
        .with_file(
            "tasks/t.knv",
            "(use tests/support helper)\n\n(deftask t () (helper))\n",
        )
        .compiler(&["task", "t"]);
    let parent = Sandbox::new("task_tests_parent")
        .with_file(&Language::project_file_name(), TASK_PROJECT)
        .with_file("src/main.knv", TASK_MODULE)
        .with_file("tests/support.knv", support)
        .with_file(
            "tasks/t.knv",
            "(use ../tests/support helper)\n\n(deftask t () (helper))\n",
        )
        .compiler(&["task", "t"]);
    let linked_sandbox = Sandbox::new("task_tests_link")
        .with_file(&Language::project_file_name(), TASK_PROJECT)
        .with_file("src/main.knv", TASK_MODULE)
        .with_file("tests/support.knv", support)
        .with_file(
            "tasks/t.knv",
            "(use link/support helper)\n\n(deftask t () (helper))\n",
        );
    std::os::unix::fs::symlink("../tests", linked_sandbox.directory.join("src/link"))
        .expect("the sandbox accepts a symbolic link");
    let linked = linked_sandbox.compiler(&["task", "t"]);
    let reports = [dotted, parent, linked].map(|outcome| outcome.assert_exit_code(1).stderr());
    insta::assert_snapshot!(reports.join("---\n"));
}

#[test]
fn task_module_hiding_a_source_module_names_the_tasks_directory() {
    let task = Sandbox::new("task_hides_source")
        .with_file(&Language::project_file_name(), TASK_PROJECT)
        .with_file("src/main.knv", TASK_MODULE)
        .with_file("src/util.knv", "(export x)\n\n(def x 1)\n")
        .with_file("tasks/util.knv", "(export x)\n\n(def x 2)\n")
        .with_file(
            "tasks/t.knv",
            "(use util x)\n\n(deftask t () (println x))\n",
        )
        .compiler(&["task", "t"]);
    insta::assert_snapshot!(task.assert_exit_code(0).stderr());
}

#[test]
fn task_file_may_still_import_deftask_from_prelude_task() {
    let task = Sandbox::new("task_explicit_import")
        .with_file(&Language::project_file_name(), TASK_PROJECT)
        .with_file("src/main.knv", TASK_MODULE)
        .with_file(
            "tasks/t.knv",
            "(use prelude/task deftask)\n\n(deftask t () (println \"t\"))\n",
        )
        .compiler(&["task", "t"]);
    assert_eq!(task.assert_exit_code(0).stdout(), "t\n");
}

#[test]
fn deftask_in_an_entry_module_is_an_error_beside_a_tasks_directory() {
    let check = Sandbox::with_tasks("deftask_in_main_beside_tasks")
        .with_file(
            "src/main.knv",
            "(deftask hidden () (println \"h\"))\n\n(defn main () (println \"m\"))\n",
        )
        .compiler(&["check"]);
    assert!(
        check
            .assert_exit_code(1)
            .stderr()
            .starts_with("error[L0359]: `deftask` defines a task only in a file under `tasks/`")
    );
}

#[test]
fn task_cannot_reach_another_task_module_through_the_source_root() {
    let reports: Vec<String> = [false, true]
        .iter()
        .map(|with_tests| {
            let sandbox = Sandbox::new(&format!("task_through_source_root_{with_tests}"))
                .with_file(
                    &Language::project_file_name(),
                    "(project rootent :entry \"main.knv\")\n",
                )
                .with_file("main.knv", "(defn main () (println \"m\"))\n")
                .with_file("tasks/util.knv", "(export h)\n\n(def h \"h\")\n")
                .with_file(
                    "tasks/t.knv",
                    "(use tasks/util h)\n\n(deftask t () (println h))\n",
                );
            let sandbox = if *with_tests {
                sandbox.with_file("tests/kept.knv", "(def kept 1)\n")
            } else {
                sandbox
            };
            sandbox
                .compiler(&["task", "t"])
                .assert_exit_code(1)
                .stderr()
        })
        .collect();
    assert_eq!(reports[0], reports[1]);
    insta::assert_snapshot!(reports[0]);
}

#[test]
fn task_with_a_bounded_open_success_is_refused_at_its_name() {
    let task = Sandbox::new("task_bounded_success")
        .with_file(&Language::project_file_name(), TASK_PROJECT)
        .with_file("src/main.knv", TASK_MODULE)
        .with_file(
            "tasks/p.knv",
            "(deftask p ()\n  (let-try (x (parse \"1\"))\n    (ok x)))\n",
        )
        .compiler(&["task", "p"]);
    insta::assert_snapshot!(task.assert_exit_code(1).stderr());
}

#[test]
fn task_help_keeps_the_docstring_indentation_relative_to_its_text() {
    let task = Sandbox::new("task_help_indentation")
        .with_file(&Language::project_file_name(), TASK_PROJECT)
        .with_file("src/main.knv", TASK_MODULE)
        .with_file(
            "tasks/t.knv",
            "(deftask t ()\n  \"Runs it.\n\n  For example:\n\n      knave task t\"\n  (println \"t\"))\n",
        )
        .compiler(&["task", "t", "--help"]);
    assert_eq!(
        task.assert_exit_code(0).stdout(),
        "usage: t\n\nRuns it.\n\nFor example:\n\n    knave task t\n"
    );
}

#[test]
fn task_file_whose_name_cannot_be_a_module_is_a_usage_error() {
    let task = Sandbox::with_tasks("task_unnamable_file")
        .with_file("tasks/Has Space.knv", "(deftask spaced () (println 1))\n")
        .compiler(&["task"]);
    insta::assert_snapshot!(task.assert_exit_code(2).stderr());
}

#[test]
fn task_tree_keeps_its_subdirectories_and_skips_a_link_that_leaves_it() {
    let sandbox = Sandbox::with_tasks("task_tree_links")
        .with_file(
            "tests/support.knv",
            "(export helper)\n\n(defn helper () (println \"t\"))\n",
        )
        .with_file(
            "tasks/shared/text.knv",
            "(export word)\n\n(def word \"shared\")\n",
        )
        .with_file(
            "tasks/uses.knv",
            "(use shared/text word)\n\n(deftask shared () (println word))\n",
        );
    std::os::unix::fs::symlink("../tests", sandbox.directory.join("tasks/link"))
        .expect("the sandbox accepts a symbolic link");
    let shared = sandbox.compiler(&["task", "shared"]);
    let linked = sandbox
        .with_file(
            "tasks/linked.knv",
            "(use link/support helper)\n\n(deftask linked () (helper))\n",
        )
        .compiler(&["task", "linked"]);
    assert_eq!(shared.assert_exit_code(0).stdout(), "shared\n");
    let report = linked.assert_exit_code(1).stderr();
    assert!(!report.contains("<tasks>"));
    assert!(report.contains(" --> tasks/linked.knv:1:6"));
}
