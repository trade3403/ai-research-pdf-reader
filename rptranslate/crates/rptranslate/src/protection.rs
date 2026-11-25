//! Conservative English scientific text protection. No external dependencies.
//! Explicit math, numeric literals, selected units/formulas and caller terms.
//! Restoration checks the exact marker sequence; it cannot certify semantics.
use std::{collections::BTreeSet, ops::Range};

const MAX_TEXT: usize = 64 * 1024;
const MAX_OUTPUT: usize = 256 * 1024;
const MAX_SPANS: usize = 1024;
const MAX_TERMS: usize = 128;
const PREFIX: &str = "[[RPX:";
const CHEMICALS: &[&str] = &["H2O", "CO2", "NaCl", "H2SO4", "NH3", "CH4", "O2", "N2"];
const UNITS: &[&str] = &[
    "mol/L", "mg/mL", "m/s", "kg/m3", "°C", "°F", "℃", "μm", "µm", "μL", "µL", "mM", "μM", "µM",
    "mmol", "mol", "kPa", "MPa", "Pa", "GHz", "MHz", "kHz", "Hz", "kg", "mg", "g", "mL", "L", "nm",
    "mm", "cm", "km", "m", "ms", "s", "min", "h", "K", "V", "mV", "A", "W", "%",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Limit,
    ReservedMarker,
    Glossary,
    UnclosedMath,
    InvalidNumber,
    MarkerMismatch,
    SegmentCount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Formula,
    Number,
    Quantity,
    Chemical,
    Term,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub kind: Kind,
    pub original: String,
    pub replacement: String,
}

#[derive(Debug)]
pub struct Plan {
    source: String,
    spans: Vec<Span>,
}

fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn left_boundary(s: &str, at: usize) -> bool {
    !s[..at].chars().next_back().is_some_and(word)
}

fn right_boundary(s: &str, at: usize) -> bool {
    !s[at..].chars().next().is_some_and(word)
}

fn escaped(s: &str, at: usize) -> bool {
    !s.as_bytes()[..at]
        .iter()
        .rev()
        .take_while(|&&b| b == b'\\')
        .count()
        .is_multiple_of(2)
}

fn math_end(s: &str, start: usize) -> Result<Option<usize>, Error> {
    if escaped(s, start) {
        return Ok(None);
    }
    let rest = &s[start..];
    let (open, close) = if rest.starts_with("$$") {
        ("$$", "$$")
    } else if rest.starts_with('$') {
        ("$", "$")
    } else if rest.starts_with(r"\(") {
        (r"\(", r"\)")
    } else if rest.starts_with(r"\[") {
        (r"\[", r"\]")
    } else {
        return Ok(None);
    };
    let content = start + open.len();
    for (offset, _) in s[content..].match_indices(close) {
        let at = content + offset;
        if !escaped(s, at) {
            if at == content {
                return Err(Error::UnclosedMath);
            }
            return Ok(Some(at + close.len()));
        }
    }
    Err(Error::UnclosedMath)
}

fn digits(b: &[u8], mut at: usize) -> usize {
    while b.get(at).is_some_and(u8::is_ascii_digit) {
        at += 1;
    }
    at
}

/// Sign, decimal, exponent e/E, optional scientific multiplication by 10^n.
/// ASCII numeric grammar only; unmatched exponent is rejected rather than cut.
fn number_end(s: &str, start: usize) -> Result<Option<usize>, Error> {
    if !left_boundary(s, start) {
        return Ok(None);
    }
    let b = s.as_bytes();
    let mut at = start;
    if let Some(c @ ('+' | '-' | '−')) = s[at..].chars().next() {
        at += c.len_utf8();
    }
    let initial = at;
    at = digits(b, at);
    let integer = at > initial;
    let mut fraction = false;
    if b.get(at) == Some(&b'.') && b.get(at + 1).is_some_and(u8::is_ascii_digit) {
        let end = digits(b, at + 1);
        fraction = true;
        at = end;
    }
    if !integer && !fraction {
        return Ok(None);
    }
    if matches!(b.get(at), Some(b'e' | b'E')) {
        let exp = at + 1;
        let mut begin = exp;
        if let Some(c @ ('+' | '-' | '−')) = s[begin..].chars().next() {
            begin += c.len_utf8();
        }
        let end = digits(b, begin);
        if end == begin {
            return Err(Error::InvalidNumber);
        }
        at = end;
    } else {
        let tail = &s[at..];
        let spaces = tail.len() - tail.trim_start_matches(' ').len();
        let mul = at + spaces;
        if s[mul..].starts_with('×') {
            let rest = &s[mul + '×'.len_utf8()..];
            let trimmed = rest.trim_start_matches(' ');
            if let Some(exp) = trimmed.strip_prefix("10^") {
                let mut begin = s.len() - exp.len();
                if let Some(c @ ('+' | '-' | '−')) = s[begin..].chars().next() {
                    begin += c.len_utf8();
                }
                let end = digits(b, begin);
                if end == begin {
                    return Err(Error::InvalidNumber);
                }
                at = end;
            }
        }
    }
    Ok(Some(at))
}

fn unit_end(s: &str, at: usize) -> Option<usize> {
    let tail = &s[at..];
    let start = at + tail.len() - tail.trim_start_matches([' ', '\t']).len();
    UNITS
        .iter()
        .filter(|&&u| s[start..].starts_with(u) && right_boundary(s, start + u.len()))
        .map(|u| start + u.len())
        .max()
}

fn push(
    spans: &mut Vec<Span>,
    source: &str,
    start: usize,
    end: usize,
    kind: Kind,
    replacement: Option<&str>,
) -> Result<(), Error> {
    if spans.len() >= MAX_SPANS {
        return Err(Error::Limit);
    }
    spans.push(Span {
        range: start..end,
        kind,
        original: source[start..end].into(),
        replacement: replacement.unwrap_or(&source[start..end]).into(),
    });
    Ok(())
}

impl Plan {
    /// Case-sensitive, exact glossary matching with Unicode word boundaries;
    /// longest match wins. Explicit math and quantities take precedence.
    pub fn prepare(source: &str, glossary: &[Term]) -> Result<Self, Error> {
        if source.len() > MAX_TEXT || glossary.len() > MAX_TERMS {
            return Err(Error::Limit);
        }
        if source.contains(PREFIX) {
            return Err(Error::ReservedMarker);
        }
        let mut seen = BTreeSet::new();
        for term in glossary {
            if term.source.is_empty()
                || term.target.is_empty()
                || term.source.len() > 256
                || term.target.len() > 256
                || term.source.trim() != term.source
                || term.target.trim() != term.target
                || term.source.chars().any(char::is_control)
                || term.target.chars().any(char::is_control)
                || term.source.contains(PREFIX)
                || term.target.contains(PREFIX)
                || !seen.insert(term.source.as_str())
            {
                return Err(Error::Glossary);
            }
        }
        let mut spans = Vec::new();
        let mut at = 0;
        while at < source.len() {
            if let Some(end) = math_end(source, at)? {
                push(&mut spans, source, at, end, Kind::Formula, None)?;
                at = end;
                continue;
            }
            if let Some(end) = number_end(source, at)? {
                if let Some(end) = unit_end(source, end) {
                    push(&mut spans, source, at, end, Kind::Quantity, None)?;
                    at = end;
                    continue;
                }
                if right_boundary(source, end) {
                    push(&mut spans, source, at, end, Kind::Number, None)?;
                    at = end;
                    continue;
                }
            }
            if left_boundary(source, at) {
                if let Some(name) = CHEMICALS
                    .iter()
                    .filter(|&&name| {
                        source[at..].starts_with(name) && right_boundary(source, at + name.len())
                    })
                    .max_by_key(|name| name.len())
                {
                    let end = at + name.len();
                    push(&mut spans, source, at, end, Kind::Chemical, None)?;
                    at = end;
                    continue;
                }
                if let Some(term) = glossary
                    .iter()
                    .filter(|t| {
                        source[at..].starts_with(&t.source)
                            && right_boundary(source, at + t.source.len())
                    })
                    .max_by_key(|t| t.source.len())
                {
                    let end = at + term.source.len();
                    push(&mut spans, source, at, end, Kind::Term, Some(&term.target))?;
                    at = end;
                    continue;
                }
            }
            at += source[at..].chars().next().ok_or(Error::Limit)?.len_utf8();
        }
        Ok(Self {
            source: source.into(),
            spans,
        })
    }

    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    /// UTF-8 text gaps, in order, including empty gaps between adjacent spans.
    /// A caller can translate only these and keep protected content out of
    /// the model entirely; sentence context may be lost by such segmentation.
    pub fn gaps(&self) -> Vec<&str> {
        let mut result = Vec::with_capacity(self.spans.len() + 1);
        let mut at = 0;
        for span in &self.spans {
            result.push(&self.source[at..span.range.start]);
            at = span.range.end;
        }
        result.push(&self.source[at..]);
        result
    }

    fn marker(index: usize) -> String {
        format!("{PREFIX}{index:04}]]")
    }

    /// Alternative whole-sentence placeholder representation. Not guaranteed
    /// to survive arbitrary translation models; restore must validate it.
    pub fn masked(&self) -> String {
        let mut out = String::new();
        let gaps = self.gaps();
        for (i, gap) in gaps.into_iter().enumerate() {
            out.push_str(gap);
            if i < self.spans.len() {
                out.push_str(&Self::marker(i));
            }
        }
        out
    }

    fn bounded_push(out: &mut String, value: &str) -> Result<(), Error> {
        if value.len() > MAX_OUTPUT.saturating_sub(out.len()) {
            return Err(Error::Limit);
        }
        out.push_str(value);
        Ok(())
    }

    /// Restore exactly one occurrence of each expected marker, in order.
    /// Reject missing, repeated, reordered or unknown reserved markers.
    pub fn restore(&self, translated: &str) -> Result<String, Error> {
        if translated.len() > MAX_OUTPUT {
            return Err(Error::Limit);
        }
        let mut rest = translated;
        let mut out = String::new();
        for (i, span) in self.spans.iter().enumerate() {
            let at = rest.find(PREFIX).ok_or(Error::MarkerMismatch)?;
            let marker = Self::marker(i);
            let suffix = rest[at..]
                .strip_prefix(&marker)
                .ok_or(Error::MarkerMismatch)?;
            Self::bounded_push(&mut out, &rest[..at])?;
            Self::bounded_push(&mut out, &span.replacement)?;
            rest = suffix;
        }
        if rest.contains(PREFIX) {
            return Err(Error::MarkerMismatch);
        }
        Self::bounded_push(&mut out, rest)?;
        Ok(out)
    }

    /// Restore translated gaps with no placeholder transport through a model.
    pub fn assemble(&self, translated_gaps: &[&str]) -> Result<String, Error> {
        if translated_gaps.len() != self.spans.len() + 1 {
            return Err(Error::SegmentCount);
        }
        let mut out = String::new();
        for (i, gap) in translated_gaps.iter().enumerate() {
            if gap.contains(PREFIX) {
                return Err(Error::ReservedMarker);
            }
            Self::bounded_push(&mut out, gap)?;
            if let Some(span) = self.spans.get(i) {
                Self::bounded_push(&mut out, &span.replacement)?;
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn term(source: &str, target: &str) -> Term {
        Term {
            source: source.into(),
            target: target.into(),
        }
    }

    #[test]
    fn exact_scientific_spans_and_utf8_offsets() {
        let input = r"测量: $E=mc^2$, −1.25e-3 mol/L, 25 °C, H2O, 80% and \[x^2\].";
        let p = Plan::prepare(input, &[]).unwrap();
        let actual: Vec<_> = p
            .spans()
            .iter()
            .map(|s| (s.kind, s.original.as_str()))
            .collect();
        assert_eq!(
            actual,
            [
                (Kind::Formula, "$E=mc^2$"),
                (Kind::Quantity, "−1.25e-3 mol/L"),
                (Kind::Quantity, "25 °C"),
                (Kind::Chemical, "H2O"),
                (Kind::Quantity, "80%"),
                (Kind::Formula, r"\[x^2\]"),
            ]
        );
        for span in p.spans() {
            assert_eq!(&input[span.range.clone()], span.original);
        }
        assert_eq!(p.restore(&p.masked()).unwrap(), input);
        assert_eq!(p.assemble(&p.gaps()).unwrap(), input);
    }

    #[test]
    fn exponent_decimal_and_unit_boundaries() {
        let p = Plan::prepare("3.2 × 10^−5 mM; .5; +2; 10 mg/mL; 7 ms; 8 model; x2", &[]).unwrap();
        let actual: Vec<_> = p.spans().iter().map(|s| s.original.as_str()).collect();
        assert_eq!(
            actual,
            ["3.2 × 10^−5 mM", ".5", "+2", "10 mg/mL", "7 ms", "8"]
        );
    }

    #[test]
    fn math_delimiters_and_escaped_dollar() {
        let s = r"Cost \$5; $x+1$, \(a+b\), \[z\], $x\$y$.";
        let p = Plan::prepare(s, &[]).unwrap();
        assert_eq!(
            p.spans().iter().filter(|s| s.kind == Kind::Formula).count(),
            4
        );
        assert_eq!(p.restore(&p.masked()).unwrap(), s);
        assert_eq!(
            Plan::prepare("broken $x", &[]).err(),
            Some(Error::UnclosedMath)
        );
        assert_eq!(
            Plan::prepare(r"broken \[x", &[]).err(),
            Some(Error::UnclosedMath)
        );
        assert_eq!(Plan::prepare("3e+", &[]).err(), Some(Error::InvalidNumber));
    }

    #[test]
    fn longest_term_and_boundary() {
        let terms = [
            term("neural", "神经"),
            term("neural network", "神经网络"),
            term("model", "模型"),
        ];
        let p = Plan::prepare("A neural network model; models, neural.", &terms).unwrap();
        assert_eq!(p.spans().len(), 3);
        assert_eq!(
            p.restore(&p.masked()).unwrap(),
            "A 神经网络 模型; models, 神经."
        );
        assert_eq!(
            p.assemble(&["一种", " ", "；models，", "。"]).unwrap(),
            "一种神经网络 模型；models，神经。"
        );
    }

    #[test]
    fn strict_marker_validation() {
        let p = Plan::prepare("1 mg and 2 mg", &[]).unwrap();
        assert_eq!(
            p.restore("甲[[RPX:0000]]乙[[RPX:0001]]").unwrap(),
            "甲1 mg乙2 mg"
        );
        for bad in [
            "[[RPX:0000]]",
            "[[RPX:0001]][[RPX:0000]]",
            "[[RPX:0000]][[RPX:0000]][[RPX:0001]]",
            "[[RPX:0000]][[RPX:0001]][[RPX:0002]]",
            "[[RPX:0000]][[RPX:0001]",
            "[[rpx:0000]][[RPX:0001]]",
        ] {
            assert_eq!(p.restore(bad), Err(Error::MarkerMismatch), "{bad}");
        }
        assert_eq!(p.assemble(&["only one"]), Err(Error::SegmentCount));
        assert_eq!(
            p.assemble(&["", "[[RPX:0000]]", ""]),
            Err(Error::ReservedMarker)
        );
    }

    #[test]
    fn resource_and_dictionary_limits() {
        assert_eq!(
            Plan::prepare("[[RPX:0000]]", &[]).err(),
            Some(Error::ReservedMarker)
        );
        assert_eq!(
            Plan::prepare(&"x".repeat(MAX_TEXT + 1), &[]).err(),
            Some(Error::Limit)
        );
        assert_eq!(
            Plan::prepare(&"1 ".repeat(MAX_SPANS + 1), &[]).err(),
            Some(Error::Limit)
        );
        for terms in [
            vec![term("x", ""), term("x", "y")],
            vec![term("x", "a"), term("x", "b")],
            vec![term("x", "[[RPX:0000]]")],
        ] {
            assert_eq!(Plan::prepare("x", &terms).err(), Some(Error::Glossary));
        }
        let p = Plan::prepare("plain", &[]).unwrap();
        assert_eq!(p.restore(&"x".repeat(MAX_OUTPUT + 1)), Err(Error::Limit));
        assert_eq!(p.restore("正常文本").unwrap(), "正常文本");
    }
}
