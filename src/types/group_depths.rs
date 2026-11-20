use super::written_argument::WrittenArgument;

pub(super) struct GroupDepths {
    arguments: Vec<WrittenArgument>,
}

impl GroupDepths {
    pub(super) fn new(arguments: Vec<WrittenArgument>) -> Self {
        Self { arguments }
    }

    pub(super) fn arguments(&self) -> &[WrittenArgument] {
        &self.arguments
    }

    pub(super) fn first(&self) -> &WrittenArgument {
        self.arguments.first().expect("a group has a position")
    }

    pub(super) fn depths_agree(&self) -> bool {
        (self.arguments.windows(2)).all(|pair| pair[0].depth == pair[1].depth)
    }

    pub(super) fn referents_agree(&self) -> bool {
        let referent = self.first().resolved.innermost_referent();
        (self.arguments.iter()).all(|argument| argument.resolved.innermost_referent() == referent)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Type;
    use crate::types::passing_fixture::PassingFixture;

    #[test]
    fn equal_depths_agree() {
        let int = PassingFixture::int();
        assert!(PassingFixture::group_depths(&[(int.clone(), 1), (int, 1)]).depths_agree());
    }

    #[test]
    fn different_depths_disagree() {
        let int = PassingFixture::int();
        assert!(!PassingFixture::group_depths(&[(int.clone(), 0), (int, 1)]).depths_agree());
    }

    #[test]
    fn references_to_one_referent_agree() {
        let int = PassingFixture::int();
        assert!(
            PassingFixture::group_depths(&[(Type::shared(int.clone()), 1), (int, 0)])
                .referents_agree()
        );
    }

    #[test]
    fn different_referents_disagree() {
        let depths = PassingFixture::group_depths(&[
            (PassingFixture::int(), 0),
            (PassingFixture::string(), 0),
        ]);
        assert!(!depths.referents_agree());
    }

    #[test]
    fn the_first_argument_is_the_first_position() {
        let int = PassingFixture::int();
        assert_eq!(
            PassingFixture::group_depths(&[(int.clone(), 2), (int, 0)])
                .first()
                .depth,
            2
        );
    }
}
