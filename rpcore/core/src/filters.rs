//! Bounded independent PDF ASCIIHex and RunLength decoders.
use crate::{Error, Object, MAX_BYTES};

fn push(out: &mut Vec<u8>, data: &[u8], limit: usize) -> Result<(), Error> {
    if data.len() > limit.saturating_sub(out.len()) { return Err(Error::Limit("decoded stream")); }
    out.extend_from_slice(data);
    Ok(())
}
fn ascii_hex(input: &[u8], limit: usize) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    let mut high = None;
    for &b in input {
        if crate::ws(b) { continue; }
        if b == b'>' {
            if let Some(h) = high { push(&mut out, &[h * 16], limit)?; }
            return Ok(out);
        }
        let n = crate::hex(b)?;
        if let Some(h) = high.take() { push(&mut out, &[h * 16 + n], limit)?; }
        else { high = Some(n); }
    }
    Err(Error::Invalid("ASCIIHex missing EOD"))
}
fn run_length(input: &[u8], limit: usize) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while let Some(&n) = input.get(i) {
        i += 1;
        match n {
            128 => return Ok(out),
            0..=127 => {
                let end = i.checked_add(n as usize + 1).ok_or(Error::Limit("RunLength"))?;
                push(&mut out, input.get(i..end).ok_or(Error::Invalid("RunLength literal"))?, limit)?;
                i = end;
            }
            _ => {
                let b = *input.get(i).ok_or(Error::Invalid("RunLength repeat"))?;
                i += 1;
                let repeated = [b; 128];
                push(&mut out, &repeated[..257-n as usize], limit)?;
            }
        }
    }
    Err(Error::Invalid("RunLength missing EOD"))
}
pub fn decode(stream: &Object, limit: usize) -> Result<Vec<u8>, Error> {
    let Object::Stream {dict,data} = stream else { return Err(Error::Invalid("stream expected")); };
    let limit = limit.min(MAX_BYTES);
    if let Some(v) = dict.get(b"DecodeParms".as_slice()) {
        if *v != Object::Null { return Err(Error::Unsupported("DecodeParms")); }
    }
    let filters = match dict.get(b"Filter".as_slice()) {
        None => { if data.len() > limit { return Err(Error::Limit("decoded stream")); } return Ok(data.clone()); }
        Some(Object::Name(n)) => vec![n.clone()],
        Some(Object::Array(a)) if a.len() <= 8 => a.iter().map(|v| match v { Object::Name(n) => Ok(n.clone()), _ => Err(Error::Invalid("filter name")) }).collect::<Result<Vec<_>,_>>()?,
        _ => return Err(Error::Unsupported("filter structure")),
    };
    let mut out = data.clone();
    for filter in filters {
        out = match filter.as_slice() {
            b"ASCIIHexDecode" | b"AHx" => ascii_hex(&out,limit)?,
            b"RunLengthDecode" | b"RL" => run_length(&out,limit)?,
            _ => return Err(Error::Unsupported("stream filter")),
        };
    }
    if out.len() > limit { return Err(Error::Limit("decoded stream")); }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hex_and_limits() {
        assert_eq!(ascii_hex(b"41 4f F>",3).unwrap(),[65,79,240]);
        assert!(ascii_hex(b"4142>",1).is_err());
        assert!(ascii_hex(b"41",8).is_err());
    }
    #[test]
    fn runs_and_limits() {
        assert_eq!(run_length(&[1,b'A',b'B',254,b'C',128],5).unwrap(),b"ABCCC");
        assert!(run_length(&[129,0,128],127).is_err());
        assert!(run_length(&[2,0],10).is_err());
    }
}
