//! Bounded lexical interpretation of PDF page content, independent of rendering.
use crate::{Document, Error, Object, PageInfo, Parser, MAX_BYTES};

pub const MAX_CONTENT_BYTES: usize = 16 * 1024 * 1024;
const MAX_STREAMS: usize = 1024;
const MAX_OPERATIONS: usize = 100_000;
const MAX_OPERANDS: usize = 256;

#[derive(Debug, Clone, PartialEq)]
pub struct Operation {
    pub operator: Vec<u8>,
    pub operands: Vec<Object>,
}

impl Document<'_> {
    /// Decode a page's Contents in array order. The limit includes separators.
    /// Streams must end at token boundaries; inline images are not interpreted.
    pub fn page_content(&self, page: &PageInfo, limit: usize) -> Result<Vec<u8>, Error> {
        let Object::Dict(node) = self.resolve(page.object_id.0, page.object_id.1)? else {
            return Err(Error::Invalid("page dictionary"));
        };
        if !matches!(node.get(b"Type".as_slice()), Some(Object::Name(n)) if n == b"Page") {
            return Err(Error::Invalid("page type"));
        }
        let Some(contents) = node.get(b"Contents".as_slice()) else { return Ok(Vec::new()); };
        let limit = limit.min(MAX_CONTENT_BYTES).min(MAX_BYTES);
        let resolved = self.dereference(contents)?;
        let mut out = Vec::new();
        match resolved {
            Object::Null => {}
            Object::Stream { .. } => return crate::filters::decode(&resolved, limit),
            Object::Array(items) => {
                if items.len() > MAX_STREAMS { return Err(Error::Limit("page content streams")); }
                for (i, item) in items.iter().enumerate() {
                    if i != 0 {
                        if out.len() >= limit { return Err(Error::Limit("page content")); }
                        out.push(b'\n');
                    }
                    if !matches!(item, Object::Ref(_, _)) {
                        return Err(Error::Invalid("content stream reference"));
                    }
                    let stream = self.dereference(item)?;
                    let data = crate::filters::decode(&stream, limit - out.len())?;
                    out.extend_from_slice(&data);
                }
            }
            _ => return Err(Error::Invalid("page Contents")),
        }
        Ok(out)
    }

    pub fn page_operations(&self, page: &PageInfo, limit: usize) -> Result<Vec<Operation>, Error> {
        parse(&self.page_content(page, limit)?)
    }
}

/// Parse operands and operators without claiming to execute graphics semantics.
/// Unknown operators are preserved for a later interpreter. Inline images are
/// rejected because their binary payload cannot be safely tokenized as words.
pub fn parse(bytes: &[u8]) -> Result<Vec<Operation>, Error> {
    if bytes.len() > MAX_CONTENT_BYTES { return Err(Error::Limit("page content")); }
    let mut p = Parser::new(bytes, 0);
    let mut operations = Vec::new();
    let mut operands = Vec::new();
    loop {
        p.skip();
        let Some(&b) = bytes.get(p.pos) else { break; };
        let object = match b {
            b'/' | b'(' | b'<' | b'[' | b'0'..=b'9' | b'+' | b'-' | b'.' => true,
            _ => {
                let saved = p.pos;
                let word = p.word()?;
                p.pos = saved;
                matches!(word, b"true" | b"false" | b"null")
            }
        };
        if object {
            if operands.len() >= MAX_OPERANDS { return Err(Error::Limit("content operands")); }
            let value = p.object(0)?;
            reject_references(&value)?;
            operands.push(value);
        } else {
            let operator = p.word()?;
            if matches!(operator, b"BI" | b"ID" | b"EI") {
                return Err(Error::Unsupported("inline image"));
            }
            if operations.len() >= MAX_OPERATIONS { return Err(Error::Limit("content operations")); }
            if operator.len() > 64 || !operator.iter().all(|c| c.is_ascii_graphic()) {
                return Err(Error::Invalid("content operator"));
            }
            operations.push(Operation { operator: operator.to_vec(), operands: std::mem::take(&mut operands) });
        }
    }
    if !operands.is_empty() { return Err(Error::Invalid("dangling content operands")); }
    Ok(operations)
}

fn reject_references(value: &Object) -> Result<(), Error> {
    match value {
        Object::Ref(_, _) | Object::Stream { .. } => Err(Error::Invalid("indirect content operand")),
        Object::Array(a) => { for v in a { reject_references(v)?; } Ok(()) }
        Object::Dict(d) => { for v in d.values() { reject_references(v)?; } Ok(()) }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lexical_operations() {
        let ops = parse(b"q 1 0 0 1 10 20 cm % comment\nBT /F1 12 Tf [(Hi) -20 <21>] TJ ET Q").unwrap();
        assert_eq!(ops.iter().map(|o| o.operator.as_slice()).collect::<Vec<_>>(),
            vec![b"q".as_slice(), b"cm", b"BT", b"Tf", b"TJ", b"ET", b"Q"]);
        assert_eq!(ops[1].operands.len(), 6);
        assert_eq!(ops[3].operands[0], Object::Name(b"F1".to_vec()));
        assert_eq!(ops[4].operands[0], Object::Array(vec![Object::String(b"Hi".to_vec()), Object::Number(-20.0), Object::String(vec![33])]));
    }
    #[test]
    fn invalid_and_limits() {
        assert!(parse(b"1 2").is_err());
        assert!(parse(b"1 0 R Do").is_err());
        assert!(parse(b"[1 0 R] TJ").is_err());
        assert!(parse(b"BI /W 1 ID xyz EI").is_err());
        assert!(parse(b"]").is_err());
        assert!(matches!(parse(&b"0 ".repeat(MAX_OPERANDS + 1)), Err(Error::Limit(_))));
        assert!(matches!(parse(&b"q ".repeat(MAX_OPERATIONS + 1)), Err(Error::Limit(_))));
    }
}
