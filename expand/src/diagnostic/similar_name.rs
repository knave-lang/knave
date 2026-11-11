use std::rc::Rc;

pub struct SimilarName;

impl SimilarName {
    const LENGTH_PER_ALLOWED_EDIT: usize = 3;

    pub fn closest(target: &str, candidates: &[Rc<str>]) -> Option<Rc<str>> {
        let length = target.chars().count();
        let allowed_edits = (length / Self::LENGTH_PER_ALLOWED_EDIT).max(1);
        let shares_a_character = |distance: usize| distance < length;
        candidates
            .iter()
            .filter(|candidate| candidate.as_ref() != target)
            .filter(|candidate| length.abs_diff(candidate.chars().count()) <= allowed_edits)
            .map(|candidate| (Self::edit_distance(target, candidate), candidate))
            .filter(|(distance, _)| *distance <= allowed_edits && shares_a_character(*distance))
            .min_by(|(left_distance, left), (right_distance, right)| {
                left_distance.cmp(right_distance).then(left.cmp(right))
            })
            .map(|(_, candidate)| Rc::clone(candidate))
    }

    fn edit_distance(left: &str, right: &str) -> usize {
        let left: Vec<char> = left.chars().collect();
        let right: Vec<char> = right.chars().collect();
        let mut rows = EditRows::new(right.len());
        for row in 1..=left.len() {
            rows.start(row);
            for column in 1..=right.len() {
                let cost = Self::cheapest_edit(&rows, &left, &right, row, column);
                rows.current[column] = cost;
            }
        }
        rows.current[right.len()]
    }

    fn cheapest_edit(
        rows: &EditRows,
        left: &[char],
        right: &[char],
        row: usize,
        column: usize,
    ) -> usize {
        let substitution =
            rows.previous[column - 1] + usize::from(left[row - 1] != right[column - 1]);
        let deletion = rows.previous[column] + 1;
        let insertion = rows.current[column - 1] + 1;
        let best = substitution.min(deletion).min(insertion);
        let is_transposition = row > 1
            && column > 1
            && left[row - 1] == right[column - 2]
            && left[row - 2] == right[column - 1];
        if is_transposition {
            best.min(rows.two_back[column - 2] + 1)
        } else {
            best
        }
    }
}

struct EditRows {
    two_back: Vec<usize>,
    previous: Vec<usize>,
    current: Vec<usize>,
}

impl EditRows {
    fn new(width: usize) -> Self {
        Self {
            two_back: vec![0; width + 1],
            previous: vec![0; width + 1],
            current: (0..=width).collect(),
        }
    }

    fn start(&mut self, row: usize) {
        std::mem::swap(&mut self.two_back, &mut self.previous);
        std::mem::swap(&mut self.previous, &mut self.current);
        self.current[0] = row;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<Rc<str>> {
        list.iter().map(|name| Rc::from(*name)).collect()
    }

    #[test]
    fn suggests_a_name_one_edit_away() {
        let candidates = names(&["point-x", "point-y", "println"]);
        assert_eq!(
            SimilarName::closest("pointx", &candidates).as_deref(),
            Some("point-x")
        );
    }

    #[test]
    fn counts_swapped_letters_as_one_edit() {
        let candidates = names(&["width", "height"]);
        assert_eq!(
            SimilarName::closest("widht", &candidates).as_deref(),
            Some("width")
        );
    }

    #[test]
    fn suggests_nothing_for_a_name_sharing_no_character() {
        let candidates = names(&["x", "y"]);
        assert_eq!(SimilarName::closest("*", &candidates), None);
    }

    #[test]
    fn suggests_nothing_when_every_name_is_far() {
        let candidates = names(&["defstruct", "println"]);
        assert_eq!(SimilarName::closest("area", &candidates), None);
    }
}
