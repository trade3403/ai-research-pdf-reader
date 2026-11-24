//! Independent, bounded zlib/DEFLATE decoder (RFC 1950 and RFC 1951).
use crate::Error;

struct Bits<'a> { data: &'a [u8], pos: usize }
impl Bits<'_> {
    fn read(&mut self, n: usize) -> Result<usize, Error> {
        if n > 16 || n > self.data.len().saturating_mul(8).saturating_sub(self.pos) {
            return Err(Error::Invalid("truncated deflate"));
        }
        let mut v = 0;
        for i in 0..n {
            v |= usize::from((self.data[self.pos / 8] >> (self.pos % 8)) & 1) << i;
            self.pos += 1;
        }
        Ok(v)
    }
    fn align(&mut self) { self.pos = (self.pos + 7) & !7; }
}
struct Huffman { counts: [usize; 16], symbols: Vec<usize> }
impl Huffman {
    fn new(lengths: &[usize], allow_single: bool, allow_empty: bool) -> Result<Self, Error> {
        let mut counts = [0; 16];
        for &n in lengths {
            if n > 15 { return Err(Error::Invalid("Huffman length")); }
            if n != 0 { counts[n] += 1; }
        }
        let total: usize = counts.iter().sum();
        if total == 0 && allow_empty { return Ok(Self { counts, symbols: Vec::new() }); }
        let mut left = 1i32;
        for &n in &counts[1..] {
            left = left * 2 - n as i32;
            if left < 0 { return Err(Error::Invalid("oversubscribed Huffman tree")); }
        }
        if left != 0 && !(allow_single && total == 1 && counts[1] == 1) {
            return Err(Error::Invalid("incomplete Huffman tree"));
        }
        let mut symbols = Vec::with_capacity(total);
        for n in 1..=15 {
            for (i, &len) in lengths.iter().enumerate() {
                if len == n { symbols.push(i); }
            }
        }
        Ok(Self { counts, symbols })
    }
    fn symbol(&self, bits: &mut Bits<'_>) -> Result<usize, Error> {
        if self.symbols.is_empty() { return Err(Error::Invalid("empty Huffman tree")); }
        let (mut code, mut first, mut index) = (0, 0, 0);
        for n in 1..=15 {
            code = (code << 1) | bits.read(1)?;
            let count = self.counts[n];
            if code >= first && code - first < count {
                return Ok(self.symbols[index + code - first]);
            }
            index += count;
            first = (first + count) << 1;
        }
        Err(Error::Invalid("Huffman code"))
    }
}
fn fixed() -> Result<(Huffman, Huffman), Error> {
    let mut lengths = [8; 288];
    lengths[144..256].fill(9);
    lengths[256..280].fill(7);
    Ok((Huffman::new(&lengths, false, false)?, Huffman::new(&[5;32], false, false)?))
}
fn dynamic(bits: &mut Bits<'_>) -> Result<(Huffman, Huffman), Error> {
    const ORDER: [usize;19] = [16,17,18,0,8,7,9,6,10,5,11,4,12,3,13,2,14,1,15];
    let nl = bits.read(5)? + 257;
    let nd = bits.read(5)? + 1;
    let nc = bits.read(4)? + 4;
    if nl > 286 { return Err(Error::Invalid("literal code count")); }
    let mut lengths = [0;19];
    for &i in &ORDER[..nc] { lengths[i] = bits.read(3)?; }
    let alphabet = Huffman::new(&lengths, false, false)?;
    let mut codes = Vec::with_capacity(nl + nd);
    while codes.len() < nl + nd {
        let (value, count) = match alphabet.symbol(bits)? {
            n @ 0..=15 => (n, 1),
            16 => (*codes.last().ok_or(Error::Invalid("Huffman repeat"))?, bits.read(2)? + 3),
            17 => (0, bits.read(3)? + 3),
            18 => (0, bits.read(7)? + 11),
            _ => return Err(Error::Invalid("Huffman alphabet")),
        };
        if count > nl + nd - codes.len() { return Err(Error::Invalid("Huffman repeat range")); }
        codes.resize(codes.len() + count, value);
    }
    if codes[256] == 0 { return Err(Error::Invalid("missing end-of-block")); }
    Ok((Huffman::new(&codes[..nl], true, false)?, Huffman::new(&codes[nl..], true, true)?))
}
const LENGTH: [usize;29] = [3,4,5,6,7,8,9,10,11,13,15,17,19,23,27,31,35,43,51,59,67,83,99,115,131,163,195,227,258];
const LEXTRA: [usize;29] = [0,0,0,0,0,0,0,0,1,1,1,1,2,2,2,2,3,3,3,3,4,4,4,4,5,5,5,5,0];
const DIST: [usize;30] = [1,2,3,4,5,7,9,13,17,25,33,49,65,97,129,193,257,385,513,769,1025,1537,2049,3073,4097,6145,8193,12289,16385,24577];
fn compressed(bits: &mut Bits<'_>, lit: &Huffman, dist: &Huffman, out: &mut Vec<u8>, limit: usize, window: usize) -> Result<(), Error> {
    loop {
        match lit.symbol(bits)? {
            n @ 0..=255 => {
                if out.len() >= limit { return Err(Error::Limit("decoded stream")); }
                out.push(n as u8);
            }
            256 => return Ok(()),
            n @ 257..=285 => {
                let i = n - 257;
                let count = LENGTH[i] + bits.read(LEXTRA[i])?;
                let d = dist.symbol(bits)?;
                if d >= DIST.len() { return Err(Error::Invalid("distance code")); }
                let offset = DIST[d] + bits.read(if d < 4 { 0 } else { d / 2 - 1 })?;
                if offset > out.len() || offset > window { return Err(Error::Invalid("deflate distance")); }
                if count > limit.saturating_sub(out.len()) { return Err(Error::Limit("decoded stream")); }
                for _ in 0..count { out.push(out[out.len() - offset]); }
            }
            _ => return Err(Error::Invalid("literal code")),
        }
    }
}
fn adler(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &v in data { a = (a + u32::from(v)) % 65521; b = (b + a) % 65521; }
    (b << 16) | a
}
pub(super) fn decode(input: &[u8], limit: usize) -> Result<Vec<u8>, Error> {
    if input.len() < 6 { return Err(Error::Invalid("zlib header")); }
    let (cmf, flg) = (input[0], input[1]);
    if cmf & 15 != 8 || cmf >> 4 > 7 || (u16::from(cmf) * 256 + u16::from(flg)) % 31 != 0 {
        return Err(Error::Invalid("zlib header"));
    }
    if flg & 32 != 0 { return Err(Error::Unsupported("zlib preset dictionary")); }
    let window = 1usize << (usize::from(cmf >> 4) + 8);
    let end = input.len() - 4;
    let mut bits = Bits { data: &input[2..end], pos: 0 };
    let mut out = Vec::new();
    let mut blocks = 0usize;
    loop {
        blocks += 1;
        if blocks > 100_000 { return Err(Error::Limit("deflate blocks")); }
        let last = bits.read(1)? != 0;
        match bits.read(2)? {
            0 => {
                bits.align();
                let len = bits.read(16)?;
                if bits.read(16)? != len ^ 65535 { return Err(Error::Invalid("stored block length")); }
                if len > limit.saturating_sub(out.len()) { return Err(Error::Limit("decoded stream")); }
                let start = bits.pos / 8;
                let end = start + len;
                out.extend_from_slice(bits.data.get(start..end).ok_or(Error::Invalid("truncated stored block"))?);
                bits.pos = end * 8;
            }
            1 => { let (l,d) = fixed()?; compressed(&mut bits,&l,&d,&mut out,limit,window)?; }
            2 => { let (l,d) = dynamic(&mut bits)?; compressed(&mut bits,&l,&d,&mut out,limit,window)?; }
            _ => return Err(Error::Invalid("reserved deflate block")),
        }
        if last { break; }
    }
    bits.align();
    if bits.pos / 8 != bits.data.len() { return Err(Error::Invalid("trailing deflate bytes")); }
    if adler(&out).to_be_bytes() != input[end..] { return Err(Error::Invalid("zlib Adler-32")); }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn hex(s: &str) -> Vec<u8> { s.as_bytes().chunks_exact(2).map(|p| crate::hex(p[0]).unwrap()*16+crate::hex(p[1]).unwrap()).collect() }
    #[test]
    fn stored_fixed_dynamic() {
        let stored = hex("7801010300fcff616263024d0127");
        assert_eq!(decode(&stored,3).unwrap(),b"abc");
        let fixed = hex("789c4b4c4a4ec4860072e00931");
        assert_eq!(decode(&fixed,24).unwrap(),b"abc".repeat(8));
        let dynamic = hex("789cedcbd10983301400c055de00a593b884c4200f8c9124eedf417af77f5b1fb5453ef36d71f4ab8f98b9626f757da2f47bd6b2ea7a47ec473e394bde67d42bd737365114455114455114455114455114455114455114455114455114455114455114455114ff35fe001fab7a21");
        assert_eq!(decode(&dynamic,11400).unwrap(), b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(200));
        assert!(matches!(decode(&dynamic,11399), Err(Error::Limit(_))));
    }
    #[test]
    fn malformed() {
        let data = hex("789c4b4c4a4ec4860072e00931");
        for i in 0..data.len() { assert!(decode(&data[..i],100).is_err()); }
        let mut bad = data.clone(); *bad.last_mut().unwrap() ^= 1;
        assert!(decode(&bad,100).is_err());
        let mut bad = data.clone(); bad.insert(bad.len()-4, 0);
        assert!(decode(&bad,100).is_err());
        assert!(Huffman::new(&[1,1,1],false,false).is_err());
        assert!(Huffman::new(&[2,2],false,false).is_err());
    }
}
