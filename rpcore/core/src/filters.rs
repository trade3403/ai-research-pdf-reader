//! Bounded independent PDF stream decoders.
mod deflate;
mod predictor;
pub mod jpeg;
pub mod jpx;
pub mod jbig2;
pub mod ccitt;
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
fn ascii85(input: &[u8], limit: usize) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    let (mut value, mut count) = (0u64, 0usize);
    let mut i = 0;
    while let Some(&c) = input.get(i) {
        i += 1;
        if crate::ws(c) { continue; }
        match c {
            b'~' => {
                if input.get(i) != Some(&b'>') { return Err(Error::Invalid("ASCII85 EOD")); }
                if count == 1 { return Err(Error::Invalid("ASCII85 final group")); }
                if count > 1 {
                    for _ in count..5 { value = value * 85 + 84; }
                    let bytes = u32::try_from(value).map_err(|_|Error::Invalid("ASCII85 overflow"))?.to_be_bytes();
                    push(&mut out,&bytes[..count-1],limit)?;
                }
                return Ok(out);
            }
            b'z' if count == 0 => push(&mut out,&[0;4],limit)?,
            b'!'..=b'u' => {
                value = value*85 + u64::from(c-b'!');
                count += 1;
                if count == 5 {
                    push(&mut out,&u32::try_from(value).map_err(|_|Error::Invalid("ASCII85 overflow"))?.to_be_bytes(),limit)?;
                    value = 0;
                    count = 0;
                }
            }
            _ => return Err(Error::Invalid("ASCII85 character")),
        }
    }
    Err(Error::Invalid("ASCII85 missing EOD"))
}
/// The final output is bounded by limit; intermediate filter stages use the
/// global MAX_BYTES ceiling (not the caller's final output limit).
pub fn decode(stream: &Object, limit: usize) -> Result<Vec<u8>, Error> {
    let Object::Stream {dict,data} = stream else { return Err(Error::Invalid("stream expected")); };
    let limit = limit.min(MAX_BYTES);
    if data.len() > MAX_BYTES { return Err(Error::Limit("stream input")); }
    let filters: Vec<&[u8]> = match dict.get(b"Filter".as_slice()) {
        None | Some(Object::Null) => Vec::new(),
        Some(Object::Name(n)) => vec![n.as_slice()],
        Some(Object::Array(a)) if a.len() <= 8 => a.iter().map(|v| match v { Object::Name(n) => Ok(n.as_slice()), _ => Err(Error::Invalid("filter name")) }).collect::<Result<Vec<_>,_>>()?,
        _ => return Err(Error::Unsupported("filter structure")),
    };
    let params: Vec<Option<&Object>> = match dict.get(b"DecodeParms".as_slice()) {
        None | Some(Object::Null) => vec![None;filters.len()],
        Some(v @ Object::Dict(_)) if filters.len()==1 => vec![Some(v)],
        Some(Object::Array(a)) if a.len()==filters.len() => a.iter().map(Some).collect(),
        _ => return Err(Error::Unsupported("DecodeParms structure")),
    };
    if filters.is_empty() {
        if data.len() > limit { return Err(Error::Limit("decoded stream")); }
        return Ok(data.clone());
    }
    let mut out = std::borrow::Cow::Borrowed(data.as_slice());
    for (i,(filter,param)) in filters.iter().zip(params).enumerate() {
        let cap = if i+1 == filters.len() { limit } else { MAX_BYTES };
        let decoded = if matches!(*filter,b"FlateDecode"|b"Fl") {
            let p = predictor::Predictor::new(param)?;
            p.apply(deflate::decode(&out,p.encoded_limit(cap))?,cap)?
        } else {
            if !matches!(param,None|Some(Object::Null)) { return Err(Error::Unsupported("filter DecodeParms")); }
            match *filter {
                b"ASCIIHexDecode" | b"AHx" => ascii_hex(&out,cap)?,
                b"ASCII85Decode" | b"A85" => ascii85(&out,cap)?,
                b"RunLengthDecode" | b"RL" => run_length(&out,cap)?,
                _ => return Err(Error::Unsupported("stream filter")),
            }
        };
        out = std::borrow::Cow::Owned(decoded);
    }
    Ok(out.into_owned())
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
    fn ascii85_groups() {
        assert_eq!(ascii85(b"z 87cURD]j7BEbo80~>",16).unwrap(),b"\0\0\0\0Hello world!");
        assert_eq!(ascii85(b"!!~>",1).unwrap(),[0]);
        for invalid in [b"!~>".as_slice(),b"uuuuu~>",b"!z~>",b"abc",b"~x"] { assert!(ascii85(invalid,100).is_err()); }
        assert!(ascii85(b"z~>",3).is_err());
    }
    #[test]
    fn runs_and_limits() {
        assert_eq!(run_length(&[1,b'A',b'B',254,b'C',128],5).unwrap(),b"ABCCC");
        assert!(run_length(&[129,0,128],127).is_err());
        assert!(run_length(&[2,0],10).is_err());
    }
}
