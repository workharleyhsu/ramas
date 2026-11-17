//! Fuzzy name search over the tree.

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use crate::tree::{NodeId, Tree};

/// Fuzzy matcher for one query.
pub struct Finder {
    pattern: Pattern,
    matcher: Matcher,
    buf: Vec<char>,
}

impl Finder {
    /// Compiles `query` (smart case: case-insensitive unless it has capitals).
    pub fn new(query: &str) -> Self {
        Finder {
            pattern: Pattern::parse(query, CaseMatching::Smart, Normalization::Smart),
            matcher: Matcher::new(Config::DEFAULT),
            buf: Vec::new(),
        }
    }

    /// Score of `name`, `None` when it does not match.
    pub fn score(&mut self, name: &str) -> Option<u32> {
        self.pattern
            .score(Utf32Str::new(name, &mut self.buf), &mut self.matcher)
    }

    /// Char positions of `name` that match, for highlighting.
    pub fn indices(&mut self, name: &str) -> Vec<u32> {
        let mut out = Vec::new();
        self.pattern.indices(
            Utf32Str::new(name, &mut self.buf),
            &mut self.matcher,
            &mut out,
        );
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Every live non-root node whose name matches, best first; ties keep
    /// depth-first tree order.
    pub fn find_all(&mut self, tree: &Tree) -> Vec<NodeId> {
        let mut hits: Vec<(u32, usize, NodeId)> = Vec::new();
        for (order, id) in tree.descendants_preorder(NodeId::ROOT).enumerate() {
            let name = tree.name(id).to_string_lossy();
            if let Some(score) = self.score(&name) {
                hits.push((score, order, id));
            }
        }
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        hits.into_iter().map(|(_, _, id)| id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::test_util::{dir, file};
    use std::ffi::OsStr;

    fn fixture() -> Tree {
        let mut t = Tree::new("/r", dir());
        let docs = t.add_child(NodeId::ROOT, OsStr::new("Documents"), &dir());
        t.add_child(docs, OsStr::new("report_2024.pdf"), &file(10));
        t.add_child(docs, OsStr::new("notes.txt"), &file(10));
        let deep = t.add_child(docs, OsStr::new("archive"), &dir());
        t.add_child(deep, OsStr::new("old_report.pdf"), &file(10));
        t.add_child(NodeId::ROOT, OsStr::new("Music"), &dir());
        t.aggregate();
        t
    }

    fn names(t: &Tree, ids: &[NodeId]) -> Vec<String> {
        ids.iter()
            .map(|&i| t.name(i).to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn fuzzy_matches_subsequences() {
        let t = fixture();
        let hits = Finder::new("rprt").find_all(&t);
        let n = names(&t, &hits);
        assert_eq!(n.len(), 2);
        assert!(n.contains(&"report_2024.pdf".to_string()));
        assert!(n.contains(&"old_report.pdf".to_string()));
    }

    #[test]
    fn smart_case() {
        let t = fixture();
        assert_eq!(Finder::new("music").find_all(&t).len(), 1);
        assert_eq!(Finder::new("MUSIC").find_all(&t).len(), 0);
    }

    #[test]
    fn exact_name_ranks_first() {
        let t = fixture();
        let hits = Finder::new("notes").find_all(&t);
        assert_eq!(names(&t, &hits)[0], "notes.txt");
        assert!(Finder::new("zzzz").find_all(&t).is_empty());
    }

    #[test]
    fn indices_for_highlight() {
        let mut f = Finder::new("nts");
        assert_eq!(f.indices("notes.txt"), vec![0, 2, 4]);
    }
}
