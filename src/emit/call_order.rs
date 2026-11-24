use crate::core::{ParsedTemplate, Piece};
use crate::types::{ChangedRoot, TypedArgument};

pub struct CallOrder<'a> {
    template: &'a str,
    arguments: &'a [TypedArgument],
}

impl<'a> CallOrder<'a> {
    const AUTOREF_PLACES: [char; 2] = ['.', '['];
    const ASSIGNMENTS: [&'static str; 6] = ["=", "+=", "-=", "*=", "/=", "%="];
    const EQUALITY: &'static str = "==";

    pub fn of(template: &'a str, arguments: &'a [TypedArgument]) -> Self {
        Self {
            template,
            arguments,
        }
    }

    pub fn assert_kept(&self) {
        debug_assert!(
            self.is_kept(),
            "template `{}` lends `mut` a place that a later argument reads, and Rust reads its arguments in order",
            self.template
        );
    }

    fn is_kept(&self) -> bool {
        let parsed =
            ParsedTemplate::parse(self.template).expect("the expansion stage checked the template");
        self.lent_and_read_later()
            .all(|position| Self::is_autoref_place(parsed.pieces(), position))
    }

    fn lent_and_read_later(&self) -> impl Iterator<Item = usize> + '_ {
        self.arguments
            .iter()
            .enumerate()
            .filter_map(|(position, argument)| {
                Some((position, ChangedRoot::of_argument(argument)?))
            })
            .filter(|(position, root)| {
                self.arguments
                    .iter()
                    .skip(position + 1)
                    .any(|argument| argument.value.mentions_local(*root))
            })
            .map(|(position, _)| position)
    }

    fn is_autoref_place(pieces: &[Piece], position: usize) -> bool {
        pieces
            .iter()
            .enumerate()
            .filter(|(_, piece)| {
                matches!(piece, Piece::Placeholder { index, .. } if *index == position)
            })
            .all(|(at, _)| match pieces.get(at + 1) {
                Some(Piece::Literal(after)) => Self::places(after.trim_start()),
                _ => false,
            })
    }

    fn places(after: &str) -> bool {
        after.starts_with(Self::AUTOREF_PLACES) || Self::assigns(after)
    }

    fn assigns(after: &str) -> bool {
        !after.starts_with(Self::EQUALITY)
            && Self::ASSIGNMENTS
                .iter()
                .any(|operator| after.starts_with(operator))
    }
}

#[cfg(test)]
mod tests {
    use super::CallOrder;
    use crate::emit::test_program::TestProgram;

    struct OrderFixture;

    impl OrderFixture {
        const READ_LATER: &'static str = "(def grow! (rust \"{0}.push({1})\" (fn ((mut (vec int)) int) unit)))\n\n(defn main ()\n  (local v (vec 1))\n  (local grown (grow! (mut v) (len v)))\n  (println (len v)))";
        const READ_EARLIER: &'static str = "(def grow! (rust \"{1}.push({0})\" (fn (int (mut (vec int))) unit)))\n\n(defn main ()\n  (local v (vec 1))\n  (local grown (grow! (len v) (mut v)))\n  (println (len v)))";
        const READ_NOWHERE: &'static str = "(def grow! (rust \"{0}.push({1})\" (fn ((mut (vec int)) int) unit)))\n\n(defn main ()\n  (local v (vec 1))\n  (local grown (grow! (mut v) 3))\n  (println (len v)))";

        fn kept(source: &'static str, template: &str) -> bool {
            let program = TestProgram::new(source);
            let call = program.last_let_value("main");
            CallOrder::of(template, TestProgram::arguments(call)).is_kept()
        }

        fn assert_kept(source: &'static str, template: &str) {
            let program = TestProgram::new(source);
            let call = program.last_let_value("main");
            CallOrder::of(template, TestProgram::arguments(call)).assert_kept();
        }
    }

    #[test]
    fn a_mut_receiver_lets_a_later_argument_read_its_place() {
        assert!(OrderFixture::kept(
            OrderFixture::READ_LATER,
            "{0}.push({1})"
        ));
    }

    #[test]
    fn an_indexed_mut_place_lets_a_later_argument_read_it() {
        assert!(OrderFixture::kept(OrderFixture::READ_LATER, "{0}[0] = {1}"));
    }

    #[test]
    fn an_assigned_mut_place_lets_its_value_read_it() {
        assert!(OrderFixture::kept(OrderFixture::READ_LATER, "*{0} += {1}"));
    }

    #[test]
    fn a_compared_mut_place_is_not_an_assignment() {
        assert!(!OrderFixture::kept(
            OrderFixture::READ_LATER,
            "Vec::len({0}) == {1}"
        ));
    }

    #[test]
    fn a_mut_argument_outside_the_receiver_breaks_the_order_when_another_argument_reads_it() {
        assert!(!OrderFixture::kept(
            OrderFixture::READ_LATER,
            "Vec::push({0}, {1})"
        ));
    }

    #[test]
    fn a_marked_placeholder_is_read_like_any_other() {
        assert!(!OrderFixture::kept(
            OrderFixture::READ_LATER,
            "Vec::push({0}, {&1})"
        ));
    }

    #[test]
    fn a_reader_before_the_mut_argument_keeps_the_order() {
        assert!(OrderFixture::kept(
            OrderFixture::READ_EARLIER,
            "Vec::push({1}, {0})"
        ));
    }

    #[test]
    fn a_mut_argument_that_no_other_argument_reads_keeps_the_order() {
        assert!(OrderFixture::kept(
            OrderFixture::READ_NOWHERE,
            "Vec::push({0}, {1})"
        ));
    }

    #[test]
    #[should_panic(expected = "lends `mut` a place that a later argument reads")]
    fn a_broken_order_fails_the_assertion() {
        OrderFixture::assert_kept(OrderFixture::READ_LATER, "Vec::push({0}, {1})");
    }
}
