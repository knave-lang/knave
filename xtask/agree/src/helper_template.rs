use crate::local_scopes::LocalScopes;
use crate::value_kind::ValueKind;

#[derive(Clone, Copy, Debug)]
pub enum HelperArgument {
    Local(ValueKind),
    Borrowed(ValueKind),
    Function(&'static HelperTemplate),
    Expression,
}

impl HelperArgument {
    fn is_passable_from(self, locals: &LocalScopes) -> bool {
        match self {
            Self::Local(kind) | Self::Borrowed(kind) => locals.holds(kind),
            Self::Function(_) | Self::Expression => true,
        }
    }

    fn passed_helper(self) -> Option<&'static HelperTemplate> {
        match self {
            Self::Function(template) => Some(template),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct HelperSignature {
    arguments: &'static [HelperArgument],
    result: Option<ValueKind>,
}

#[derive(Debug)]
pub struct HelperTemplate {
    pub name: &'static str,
    pub definition: &'static str,
    pub calls: Option<&'static HelperTemplate>,
    signature: HelperSignature,
}

impl HelperSignature {
    const fn returning(arguments: &'static [HelperArgument], result: ValueKind) -> Self {
        Self {
            arguments,
            result: Some(result),
        }
    }

    const fn changing(arguments: &'static [HelperArgument]) -> Self {
        Self {
            arguments,
            result: None,
        }
    }
}

impl HelperTemplate {
    const VECTOR: HelperArgument = HelperArgument::Local(ValueKind::NUMBERS);
    const CAR: HelperArgument = HelperArgument::Local(ValueKind::CAR);
    const NUMBER: HelperArgument = HelperArgument::Expression;
    const FIGURE: HelperArgument = HelperArgument::Local(ValueKind::FIGURE);
    const BORROWED_MAP: HelperArgument = HelperArgument::Borrowed(ValueKind::MAP);
    const BORROWED_VECTOR: HelperArgument = HelperArgument::Borrowed(ValueKind::NUMBERS);
    const ONE_VECTOR: &'static [HelperArgument] = &[Self::VECTOR];
    const TWO_VECTORS_AND_A_NUMBER: &'static [HelperArgument] =
        &[Self::VECTOR, Self::VECTOR, Self::NUMBER];
    const VECTOR_AND_NUMBER: &'static [HelperArgument] = &[Self::VECTOR, Self::NUMBER];

    const FIRST_OF: Self = Self::new(
        "first-of",
        "(defn first-of (v) (at v 0))",
        HelperSignature::returning(Self::ONE_VECTOR, ValueKind::ELEMENT_BORROW),
    );
    const EITHER: Self = Self::new(
        "either",
        "(defn either (a b k) (if (> k 0) (at a 0) (at b 0)))",
        HelperSignature::returning(Self::TWO_VECTORS_AND_A_NUMBER, ValueKind::ELEMENT_BORROW),
    );
    const EITHER_SWAPPED: Self = Self::new(
        "either-swapped",
        "(defn either-swapped (a b k) (either b a k))",
        HelperSignature::returning(Self::TWO_VECTORS_AND_A_NUMBER, ValueKind::ELEMENT_BORROW),
    )
    .calling(&Self::EITHER);
    const MAKE_OF: Self = Self::new(
        "make-of",
        "(defn make-of (c) (:make c))",
        HelperSignature::returning(&[Self::CAR], ValueKind::MAKE_BORROW),
    );
    const MAKE_WHEN: Self = Self::new(
        "make-when",
        "(defn make-when (c d k) (if (> k 0) (:make c) (:make d)))",
        HelperSignature::returning(
            &[Self::CAR, Self::CAR, Self::NUMBER],
            ValueKind::MAKE_BORROW,
        ),
    );
    const HELD_FIRST: Self = Self::new(
        "held-first",
        "(defn held-first (v)\n  (local r (at v 0))\n  r)",
        HelperSignature::returning(Self::ONE_VECTOR, ValueKind::ELEMENT_BORROW),
    );
    const LOOP_FIRST: Self = Self::new(
        "loop-first",
        "(defn loop-first (v)\n  (loop (i 0) (if (< i 1) (recur (+ i 1)) (at v 0))))",
        HelperSignature::returning(Self::ONE_VECTOR, ValueKind::ELEMENT_BORROW),
    );
    const GROW_THEN_FIRST: Self = Self::new(
        "grow-then-first!",
        "(defn grow-then-first! (v k)\n  (if (> k 0) (push! v k) (do))\n  (at v 0))",
        HelperSignature::returning(Self::VECTOR_AND_NUMBER, ValueKind::ELEMENT_BORROW),
    );
    const READ_THEN_GROW: Self = Self::new(
        "read-then-grow!",
        "(defn read-then-grow! (v)\n  (local r (at v 0))\n  (local x (deref r))\n  (if (> x 0) (push! v 1) (do))\n  x)",
        HelperSignature::returning(Self::ONE_VECTOR, ValueKind::NUMBER),
    );
    const GROW_THEN_READ: Self = Self::new(
        "grow-then-read!",
        "(defn grow-then-read! (v)\n  (local r (at v 0))\n  (loop (i 0) (if (< i 1) (do (push! v 1) (recur (+ i 1))) (do)))\n  (deref r))",
        HelperSignature::returning(Self::ONE_VECTOR, ValueKind::NUMBER),
    );
    const GROW: Self = Self::new(
        "grow!",
        "(defn grow! (v n) (push! v n))",
        HelperSignature::changing(Self::VECTOR_AND_NUMBER),
    );
    const SUM_TO: Self = Self::new(
        "sum-to",
        "(defn sum-to (n) (if (> n 0) (+ n (sum-to (- n 1))) 0))",
        HelperSignature::returning(&[Self::NUMBER], ValueKind::NUMBER),
    );
    const COUNT_DOWN: Self = Self::new(
        "count-down",
        "(defn count-down (n) (if (> n 0) (count-up (- n 1)) 0))
(defn count-up (n) (if (> n 0) (+ 1 (count-down (- n 1))) 0))",
        HelperSignature::returning(&[Self::NUMBER], ValueKind::NUMBER),
    );
    const LARGER: Self = Self::new(
        "larger",
        "(defn larger (a b) (if (> a b) a b))",
        HelperSignature::returning(&[Self::NUMBER, Self::NUMBER], ValueKind::NUMBER),
    );
    const LARGER_MAKE: Self = Self::new(
        "larger-make",
        "(defn larger-make (c d k)\n  (if (> (larger k 0) 0) (larger (:make c) (:make d)) (larger (:make d) (:make c))))",
        HelperSignature::returning(
            &[Self::CAR, Self::CAR, Self::NUMBER],
            ValueKind::MAKE_BORROW,
        ),
    )
    .calling(&Self::LARGER);
    const BIGGER_FIGURE: Self = Self::new(
        "bigger-figure",
        "(defn bigger-figure (a b) (if (> (size a) (size b)) a b))",
        HelperSignature::returning(&[Self::FIGURE, Self::FIGURE], ValueKind::FIGURE),
    );
    const ENTRY_OF: Self = Self::new(
        "entry-of",
        "(type entry-of (fn ((ref (sorted-map i64 i64)) i64) (option (ref i64))))\n(defn entry-of (m k) (get m k))",
        HelperSignature::returning(&[Self::BORROWED_MAP, Self::NUMBER], ValueKind::MAP_ENTRY),
    );
    const HEAD: Self = Self::new(
        "head",
        "(defn head ((v (ref (vec i64)))) (at v 0))",
        HelperSignature::returning(&[Self::BORROWED_VECTOR], ValueKind::ELEMENT_BORROW),
    );
    const APPLY_TO: Self = Self::new(
        "apply-to",
        "(defn apply-to (f v) (f v))",
        HelperSignature::returning(
            &[HelperArgument::Function(&Self::HEAD), Self::BORROWED_VECTOR],
            ValueKind::ELEMENT_BORROW,
        ),
    );
    const STORE_YEAR: Self = Self::new(
        "store-year!",
        "(defn store-year! (v c) (push! v (:year c)))",
        HelperSignature::changing(&[Self::VECTOR, Self::CAR]),
    );

    pub const ALL: [&'static HelperTemplate; 20] = [
        &Self::FIRST_OF,
        &Self::EITHER,
        &Self::EITHER_SWAPPED,
        &Self::MAKE_OF,
        &Self::MAKE_WHEN,
        &Self::HELD_FIRST,
        &Self::LOOP_FIRST,
        &Self::GROW_THEN_FIRST,
        &Self::READ_THEN_GROW,
        &Self::GROW_THEN_READ,
        &Self::GROW,
        &Self::STORE_YEAR,
        &Self::SUM_TO,
        &Self::COUNT_DOWN,
        &Self::LARGER,
        &Self::LARGER_MAKE,
        &Self::BIGGER_FIGURE,
        &Self::ENTRY_OF,
        &Self::HEAD,
        &Self::APPLY_TO,
    ];

    const fn new(name: &'static str, definition: &'static str, signature: HelperSignature) -> Self {
        Self {
            name,
            definition,
            calls: None,
            signature,
        }
    }

    const fn calling(self, callee: &'static HelperTemplate) -> Self {
        Self {
            calls: Some(callee),
            ..self
        }
    }

    pub fn dependencies(&self) -> impl Iterator<Item = &'static HelperTemplate> + '_ {
        self.calls.into_iter().chain(
            self.arguments()
                .iter()
                .filter_map(|argument| argument.passed_helper()),
        )
    }

    pub fn is_callable_from(&self, locals: &LocalScopes) -> bool {
        self.arguments()
            .iter()
            .all(|argument| argument.is_passable_from(locals))
    }

    pub fn arguments(&self) -> &'static [HelperArgument] {
        self.signature.arguments
    }

    pub fn result(&self) -> Option<ValueKind> {
        self.signature.result
    }
}

#[cfg(test)]
mod tests {
    use super::HelperTemplate;
    use crate::local_scopes::LocalScopes;
    use crate::value_kind::ValueKind;

    struct HelperFixture;

    impl HelperFixture {
        fn callable_from(locals: &LocalScopes) -> Vec<&'static str> {
            HelperTemplate::ALL
                .into_iter()
                .filter(|template| template.is_callable_from(locals))
                .map(|template| template.name)
                .collect()
        }
    }

    #[test]
    fn calls_a_helper_only_when_a_local_of_each_argument_kind_is_in_scope() {
        let mut locals = LocalScopes::new();
        let without_a_map = HelperFixture::callable_from(&locals);
        assert!(without_a_map.contains(&"sum-to"));
        assert!(!without_a_map.contains(&"entry-of"));
        locals.bind("m", ValueKind::MAP);
        assert!(HelperFixture::callable_from(&locals).contains(&"entry-of"));
    }
}
