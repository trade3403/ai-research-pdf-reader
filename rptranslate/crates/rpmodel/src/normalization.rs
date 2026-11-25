//! Independent bounded reader for SentencePiece compiled normalization maps.
//! Only the default prefix/trim/escape whitespace profile is implemented.
use std::borrow::Cow;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Format,
    Bounds,
    Utf8,
    Limit,
}

const MAX_MAP: usize = 16 * 1024 * 1024;
const MAX_INPUT: usize = 64 * 1024;
const MAX_OUTPUT: usize = 1024 * 1024;
const MAX_PREFIX: usize = 256;
const SPACE: char = '▁';

pub struct CharsMap<'a> {
    trie: Cow<'a, [u8]>,
    pool: Cow<'a, str>,
}

impl<'a> CharsMap<'a> {
    pub fn parse(blob: &'a [u8]) -> Result<Self, Error> {
        if blob.len() > MAX_MAP {
            return Err(Error::Limit);
        }
        let (header, body) = blob.split_at_checked(4).ok_or(Error::Format)?;
        let size = u32::from_le_bytes(header.try_into().map_err(|_| Error::Format)?) as usize;
        if size == 0 || !size.is_multiple_of(1024) {
            return Err(Error::Format);
        }
        let (trie, pool) = body.split_at_checked(size).ok_or(Error::Bounds)?;
        if pool.last() != Some(&0) {
            return Err(Error::Format);
        }
        let map = Self {
            trie: Cow::Borrowed(trie),
            pool: Cow::Borrowed(std::str::from_utf8(pool).map_err(|_| Error::Utf8)?),
        };
        let root = map.unit(0)?;
        if root & 0x8000_00ff != 0 {
            return Err(Error::Format);
        }
        map.unit(offset(root))?;
        Ok(map)
    }

    /// Keep a verified map independently of its source ModelProto bytes.
    pub fn into_owned(self) -> CharsMap<'static> {
        CharsMap {
            trie: Cow::Owned(self.trie.into_owned()),
            pool: Cow::Owned(self.pool.into_owned()),
        }
    }

    fn unit(&self, i: usize) -> Result<u32, Error> {
        let start = i.checked_mul(4).ok_or(Error::Bounds)?;
        let end = start.checked_add(4).ok_or(Error::Bounds)?;
        let bytes = self.trie.get(start..end).ok_or(Error::Bounds)?;
        Ok(u32::from_le_bytes(
            bytes.try_into().map_err(|_| Error::Bounds)?,
        ))
    }

    fn replacement(&self, leaf: u32) -> Result<&str, Error> {
        if leaf & 0x8000_0000 == 0 {
            return Err(Error::Format);
        }
        let index = (leaf & 0x7fff_ffff) as usize;
        let tail = self.pool.get(index..).ok_or(Error::Bounds)?;
        let end = tail
            .as_bytes()
            .iter()
            .take(MAX_OUTPUT + 1)
            .position(|&b| b == 0);
        match end {
            Some(n) => tail.get(..n).ok_or(Error::Utf8),
            None if tail.len() > MAX_OUTPUT => Err(Error::Limit),
            None => Err(Error::Format),
        }
    }

    /// Longest matching UTF-8 byte prefix, falling back to one Unicode scalar.
    fn prefix<'b>(&'b self, input: &'b str) -> Result<(&'b str, usize), Error> {
        let mut base = offset(self.unit(0)?);
        let mut longest = None;
        for (i, b) in input.bytes().enumerate() {
            if i == MAX_PREFIX {
                return Err(Error::Limit);
            }
            let index = base ^ usize::from(b);
            let unit = self.unit(index)?;
            if unit & 0x8000_00ff != u32::from(b) {
                break;
            }
            base = index ^ offset(unit);
            if unit & 0x100 != 0 {
                if !input.is_char_boundary(i + 1) {
                    return Err(Error::Utf8);
                }
                longest = Some((self.replacement(self.unit(base)?)?, i + 1));
            }
        }
        if let Some(found) = longest {
            return Ok(found);
        }
        let n = input.chars().next().ok_or(Error::Format)?.len_utf8();
        Ok((&input[..n], n))
    }

    pub fn normalize(&self, input: &str) -> Result<String, Error> {
        if input.len() > MAX_INPUT {
            return Err(Error::Limit);
        }
        let mut rest = input;
        let mut output = String::new();
        let mut previous_space = true;
        if !rest.is_empty() {
            output.push(SPACE);
        }
        while !rest.is_empty() {
            let (mut text, used) = self.prefix(rest)?;
            if previous_space {
                text = text.trim_start_matches(' ');
            }
            if !text.is_empty() {
                previous_space = text.ends_with(' ');
                for ch in text.chars() {
                    let escaped = if ch == ' ' { SPACE } else { ch };
                    if output.len() + escaped.len_utf8() > MAX_OUTPUT {
                        return Err(Error::Limit);
                    }
                    output.push(escaped);
                }
            }
            rest = rest.get(used..).ok_or(Error::Utf8)?;
        }
        output.truncate(output.trim_end_matches(SPACE).len());
        Ok(output)
    }
}

fn offset(unit: u32) -> usize {
    ((unit >> 10) << ((unit & 0x200) >> 6)) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        // a -> A; ab -> Z. The longer rule must win.
        let mut units = [0_u32; 256];
        units[0] = 1 << 10;
        units[96] = (1 << 10) | 0x100 | u32::from(b'a');
        units[97] = 0x8000_0000;
        units[3] = (1 << 10) | 0x100 | u32::from(b'b');
        units[2] = 0x8000_0002;
        let mut data = 1024_u32.to_le_bytes().to_vec();
        for u in units {
            data.extend_from_slice(&u.to_le_bytes());
        }
        data.extend_from_slice(b"A\0Z\0");
        data
    }

    #[test]
    fn longest_rule_and_unicode() {
        let bytes = sample();
        let map = CharsMap::parse(&bytes).unwrap();
        assert_eq!(map.normalize("  a ab 学  ").unwrap(), "▁A▁Z▁学");
        assert_eq!(map.normalize("").unwrap(), "");
        assert_eq!(map.normalize("  ").unwrap(), "");
    }

    #[test]
    fn owned_map_survives_source_release() {
        let map = {
            let bytes = sample();
            CharsMap::parse(&bytes).unwrap().into_owned()
        };
        assert_eq!(map.normalize("  a ab 学  ").unwrap(), "▁A▁Z▁学");
        assert_eq!(map.normalize("").unwrap(), "");
    }

    #[test]
    fn malformed_maps() {
        let mut bytes = sample();
        bytes.pop();
        assert_eq!(CharsMap::parse(&bytes).err(), Some(Error::Format));
        let mut bytes = sample();
        bytes[4..8].copy_from_slice(&(1024_u32 << 10).to_le_bytes());
        assert_eq!(CharsMap::parse(&bytes).err(), Some(Error::Bounds));
        let mut bytes = sample();
        bytes[392..396].copy_from_slice(&0x8000_ffff_u32.to_le_bytes());
        let map = CharsMap::parse(&bytes).unwrap();
        assert_eq!(map.normalize("a"), Err(Error::Bounds));
    }
}
