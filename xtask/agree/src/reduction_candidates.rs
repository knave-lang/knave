use std::ops::Range;

use knave_expand::reader::{TokenKind, TokenScan};

pub struct ReductionCandidates {
    text: String,
    lists: Vec<ListSpan>,
}

struct ListSpan {
    range: Range<usize>,
    children: Vec<Range<usize>>,
}

impl ListSpan {
    fn replacements<'a>(&'a self, text: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        std::iter::once("").chain(self.children.iter().map(|child| &text[child.clone()]))
    }
}

impl ReductionCandidates {
    pub fn of(text: &str) -> Self {
        let mut lists = Self::lists(text);
        lists.sort_by_key(|list| std::cmp::Reverse(list.range.len()));
        Self {
            text: text.to_string(),
            lists,
        }
    }

    pub fn texts(&self) -> impl Iterator<Item = String> + '_ {
        self.lists.iter().flat_map(move |list| {
            list.replacements(&self.text)
                .map(move |by| self.replaced(&list.range, by))
        })
    }

    fn replaced(&self, range: &Range<usize>, by: &str) -> String {
        let joined = format!(
            "{}{by}{}",
            &self.text[..range.start],
            &self.text[range.end..]
        );
        joined
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| format!("{line}\n"))
            .collect()
    }

    fn lists(text: &str) -> Vec<ListSpan> {
        let mut open: Vec<(usize, Vec<Range<usize>>)> = Vec::new();
        let mut lists = Vec::new();
        for token in TokenScan::new(text) {
            match token.kind {
                TokenKind::Open => open.push((token.range.start, Vec::new())),
                TokenKind::Close => {
                    let Some((start, children)) = open.pop() else {
                        continue;
                    };
                    let range = start..token.range.end;
                    if let Some((_, siblings)) = open.last_mut() {
                        siblings.push(range.clone());
                    }
                    lists.push(ListSpan { range, children });
                }
                _ => {}
            }
        }
        lists
    }
}

#[cfg(test)]
mod tests {
    use super::ReductionCandidates;

    #[test]
    fn removes_each_list_or_puts_one_of_its_lists_in_its_place_largest_first() {
        let candidates: Vec<String> = ReductionCandidates::of("(a (b) (c (d)))").texts().collect();
        assert_eq!(
            candidates,
            [
                "",
                "(b)\n",
                "(c (d))\n",
                "(a (b) )\n",
                "(a (b) (d))\n",
                "(a  (c (d)))\n",
                "(a (b) (c ))\n",
            ]
        );
    }

    #[test]
    fn drops_the_lines_a_removal_leaves_blank() {
        let candidates: Vec<String> = ReductionCandidates::of("(defn main ()\n  (a)\n  (b))\n")
            .texts()
            .collect();
        assert!(candidates.contains(&"(defn main ()\n  (b))\n".to_string()));
    }
}
