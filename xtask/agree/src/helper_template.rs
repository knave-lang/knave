use crate::value_kind::ValueKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelperArgument {
    Local(ValueKind),
    Expression,
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
    const STORE_YEAR: Self = Self::new(
        "store-year!",
        "(defn store-year! (v c) (push! v (:year c)))",
        HelperSignature::changing(&[Self::VECTOR, Self::CAR]),
    );

    pub const ALL: [&'static HelperTemplate; 12] = [
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

    pub fn arguments(&self) -> &'static [HelperArgument] {
        self.signature.arguments
    }

    pub fn result(&self) -> Option<ValueKind> {
        self.signature.result
    }
}
