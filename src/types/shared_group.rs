use super::ty::TypeVar;

pub(super) struct SharedGroup {
    root: TypeVar,
    members: Vec<SharedMember>,
}

pub(super) struct SharedMember {
    pub(super) var: TypeVar,
    pub(super) positions: Vec<usize>,
}

impl SharedGroup {
    pub(super) fn new(root: TypeVar) -> Self {
        Self {
            root,
            members: Vec::new(),
        }
    }

    pub(super) fn root(&self) -> TypeVar {
        self.root
    }

    pub(super) fn members(&self) -> &[SharedMember] {
        &self.members
    }

    pub(super) fn is_linked(&self) -> bool {
        self.members.len() > 1
    }

    pub(super) fn positions(&self) -> impl Iterator<Item = usize> + Clone + '_ {
        self.members
            .iter()
            .flat_map(|member| member.positions.iter().copied())
    }

    pub(super) fn contains(&self, position: usize) -> bool {
        self.members
            .iter()
            .any(|member| member.positions.contains(&position))
    }

    pub(super) fn add_position(&mut self, var: TypeVar, position: usize) {
        match self.members.iter_mut().find(|member| member.var == var) {
            Some(member) => member.positions.push(position),
            None => self.members.push(SharedMember {
                var,
                positions: vec![position],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::passing_fixture::PassingFixture;

    #[test]
    fn positions_of_one_variable_join_one_member() {
        let group = PassingFixture::new().shared_group(&[0, 0]);
        assert_eq!(group.members().len(), 1);
        assert_eq!(group.positions().collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn a_group_of_one_variable_is_not_linked() {
        assert!(!PassingFixture::new().shared_group(&[0, 0]).is_linked());
    }

    #[test]
    fn a_group_of_two_variables_is_linked() {
        assert!(PassingFixture::new().shared_group(&[0, 1]).is_linked());
    }

    #[test]
    fn a_group_contains_only_its_positions() {
        let group = PassingFixture::new().shared_group(&[0, 1]);
        assert!(group.contains(1));
        assert!(!group.contains(2));
    }
}
