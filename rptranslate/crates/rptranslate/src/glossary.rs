//! Bounded, dependency-free scientific glossary trie.
//! Explicit domain selection; no statistical or contextual disambiguation.
use std::collections::BTreeMap;

const MAX_TSV: usize = 1024 * 1024;
const MAX_ROWS: usize = 4096;
const MAX_KEY: usize = 128;
const MAX_TARGET: usize = 256;
const BUILTIN: &str = include_str!("glossary.tsv");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Domain {
    General,
    Geotechnical,
    Biology,
}

impl Domain {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "general" => Some(Self::General),
            "geotechnical" => Some(Self::Geotechnical),
            "biology" => Some(Self::Biology),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    InvalidRow(usize),
    Conflict(usize),
}

#[derive(Default)]
struct Node {
    children: BTreeMap<char, usize>,
    target: Option<String>,
}

pub struct Glossary {
    nodes: Vec<Node>,
    entries: usize,
    domain: Domain,
}

fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn normalized(c: char) -> char {
    match c {
        ' ' | '\t' => ' ',
        '‐' | '‑' | '–' => '-',
        _ => c.to_ascii_lowercase(),
    }
}

fn key(source: &str) -> Option<String> {
    if source.len() > MAX_KEY || source.trim() != source {
        return None;
    }
    let mut out = String::new();
    for c in source.chars().map(normalized) {
        if !(c.is_ascii_alphanumeric() || c == ' ' || c == '-') {
            return None;
        }
        if c != ' ' || !out.ends_with(' ') {
            out.push(c);
        }
    }
    if !out.starts_with(|c: char| c.is_ascii_alphabetic())
        || !out.ends_with(|c: char| c.is_ascii_alphanumeric())
    {
        return None;
    }
    Some(out)
}

impl Glossary {
    pub fn builtin(domain: Domain) -> Result<Self, Error> {
        Self::from_tsv(domain, BUILTIN)
    }

    /// Each non-comment row is domain, English alias, Chinese target (TSV).
    /// General entries apply to all domains. Conflicting normalized keys
    /// within a domain, or across general and a selected domain, are errors.
    pub fn from_tsv(domain: Domain, tsv: &str) -> Result<Self, Error> {
        if tsv.len() > MAX_TSV {
            return Err(Error::Limit);
        }
        let mut all = BTreeMap::new();
        let mut rows = 0;
        for (index, line) in tsv.lines().enumerate() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            rows += 1;
            if rows > MAX_ROWS {
                return Err(Error::Limit);
            }
            let line_no = index + 1;
            let mut fields = line.split('\t');
            let scope = fields.next().and_then(Domain::parse);
            let source = fields.next().and_then(key);
            let target = fields.next();
            let (Some(scope), Some(source), Some(target)) = (scope, source, target) else {
                return Err(Error::InvalidRow(line_no));
            };
            if fields.next().is_some()
                || target.is_empty()
                || target.len() > MAX_TARGET
                || target.trim() != target
                || target.chars().any(char::is_control)
                || target.contains("[[RPX:")
            {
                return Err(Error::InvalidRow(line_no));
            }
            let k = (scope, source);
            if all
                .insert(k, (target, line_no))
                .is_some_and(|old| old.0 != target)
            {
                return Err(Error::Conflict(line_no));
            }
        }
        // Check every scope, even when the caller selects a different one.
        for ((scope, source), &(target, line_no)) in &all {
            let common = all.get(&(Domain::General, source.clone()));
            if *scope != Domain::General && common.is_some_and(|&(v, _)| v != target) {
                return Err(Error::Conflict(line_no));
            }
        }
        let mut result = Self {
            nodes: vec![Node::default()],
            entries: 0,
            domain,
        };
        for ((scope, source), (target, _)) in all {
            if scope == Domain::General || scope == domain {
                result.insert(&source, target);
            }
        }
        Ok(result)
    }

    fn insert(&mut self, source: &str, target: &str) {
        let mut at = 0;
        for c in source.chars() {
            let next = if let Some(&next) = self.nodes[at].children.get(&c) {
                next
            } else {
                let next = self.nodes.len();
                self.nodes.push(Node::default());
                self.nodes[at].children.insert(c, next);
                next
            };
            at = next;
        }
        if self.nodes[at].target.is_none() {
            self.entries += 1;
            self.nodes[at].target = Some(target.into());
        }
    }

    pub fn len(&self) -> usize {
        self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries == 0
    }

    pub fn domain(&self) -> Domain {
        self.domain
    }

    /// Match at a UTF-8 boundary, choosing the longest complete alias.
    /// Case, horizontal spaces and three Unicode hyphens are normalized.
    /// No stemming: plural variants must be explicit dictionary rows.
    pub(crate) fn match_at<'a>(&'a self, source: &str, at: usize) -> Option<(usize, &'a str)> {
        let tail = source.get(at..)?;
        if source[..at].chars().next_back().is_some_and(word) {
            return None;
        }
        let mut node = 0;
        let mut best = None;
        let mut space = false;
        for (offset, raw) in tail.char_indices() {
            let c = normalized(raw);
            if c == ' ' && space {
                continue;
            }
            space = c == ' ';
            let Some(&next) = self.nodes[node].children.get(&c) else {
                break;
            };
            node = next;
            let end = at + offset + raw.len_utf8();
            if let Some(target) = self.nodes[node]
                .target
                .as_deref()
                .filter(|_| !source[end..].chars().next().is_some_and(word))
            {
                best = Some((end, target));
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_homonyms_and_general() {
        let geo = Glossary::builtin(Domain::Geotechnical).unwrap();
        let bio = Glossary::builtin(Domain::Biology).unwrap();
        let general = Glossary::builtin(Domain::General).unwrap();
        assert_eq!(geo.match_at("STRAIN", 0), Some((6, "应变")));
        assert_eq!(bio.match_at("strain", 0), Some((6, "菌株")));
        assert_eq!(general.match_at("strain", 0), None);
        assert_eq!(geo.match_at("uncertainty", 0), Some((11, "不确定性")));
        assert_eq!(geo.domain(), Domain::Geotechnical);
        assert!(!geo.is_empty());
        assert!(geo.len() > general.len());
    }

    #[test]
    fn longest_case_space_and_hyphen() {
        let g = Glossary::builtin(Domain::Geotechnical).unwrap();
        let s = "Maximum  STRAIN, non‑linear site response";
        assert_eq!(g.match_at(s, 0), Some((15, "最大应变")));
        let at = s.find("non").unwrap();
        assert_eq!(g.match_at(s, at), Some((s.len(), "非线性场地响应")));
        assert_eq!(g.match_at("input motions", 0), Some((13, "输入地震动")));
    }

    #[test]
    fn boundary_and_unlisted_forms() {
        let g = Glossary::builtin(Domain::Geotechnical).unwrap();
        for s in ["restrain", "strained", "strain_rate", "strain2", "strain量"] {
            assert_eq!(g.match_at(s, 0), None, "{s}");
        }
        assert_eq!(g.match_at("restrain", 2), None);
        assert_eq!(g.match_at("éstrain", 1), None);
        assert_eq!(g.match_at("éstrain", 2), None);
        assert_eq!(g.match_at("strain.", 0), Some((6, "应变")));
        assert_eq!(g.match_at("site\nresponse", 0), None);
    }

    #[test]
    fn normalized_conflicts_fail_even_in_inactive_scope() {
        let t = "geotechnical\tStrain\t应变\ngeotechnical\tstrain\t错误";
        assert!(matches!(
            Glossary::from_tsv(Domain::General, t),
            Err(Error::Conflict(2))
        ));
        let t = "general\tstrain\t甲\nbiology\tstrain\t乙";
        assert!(matches!(
            Glossary::from_tsv(Domain::General, t),
            Err(Error::Conflict(2))
        ));
        let t = "geotechnical\tSite  Response\t甲\ngeotechnical\tsite response\t乙";
        assert!(matches!(
            Glossary::from_tsv(Domain::Geotechnical, t),
            Err(Error::Conflict(2))
        ));
    }

    #[test]
    fn strict_rows_limits_and_deduplication() {
        for t in [
            "unknown\tstrain\t应变",
            "general\tstrain\t",
            "general\tbad_key\t甲",
            "general\tstrain\t[[RPX:0000]]",
            "general\tstrain\t甲\textra",
        ] {
            assert!(matches!(
                Glossary::from_tsv(Domain::General, t),
                Err(Error::InvalidRow(1))
            ));
        }
        let t = "general\tstrain\t应变\ngeneral\tSTRAIN\t应变";
        assert_eq!(Glossary::from_tsv(Domain::General, t).unwrap().len(), 1);
        assert!(matches!(
            Glossary::from_tsv(Domain::General, &"x".repeat(MAX_TSV + 1)),
            Err(Error::Limit)
        ));
    }
}
