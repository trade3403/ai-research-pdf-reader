//! PNG encoder with independent CRC32, Adler32 and stored DEFLATE blocks.
use crate::{Error,Result};
fn crc32(bytes:&[u8])->u32{
    let mut c=0xffffffffu32;
    for &b in bytes{c^=u32::from(b);for _ in 0..8{c=(c>>1)^(0xedb88320u32 & 0u32.wrapping_sub(c&1));}}
    !c
}
fn chunk(out:&mut Vec<u8>,kind:&[u8;4],data:&[u8]){
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());let start=out.len();
    out.extend_from_slice(kind);out.extend_from_slice(data);let crc=crc32(&out[start..]);out.extend_from_slice(&crc.to_be_bytes());
}
pub(super) fn encode(w:usize,h:usize,rgba:&[u8])->Result<Vec<u8>>{
    if w==0||h==0||w>super::MAX_SIDE||h>super::MAX_SIDE||w*h>super::MAX_PIXELS||rgba.len()!=w*h*4{return Err(Error::Invalid("PNG bitmap"));}
    let mut raw=Vec::with_capacity(h*(1+w*4));
    for row in rgba.chunks_exact(w*4){raw.push(0);raw.extend_from_slice(row);}
    let mut z=vec![0x78,0x01];let(mut a,mut b)=(1u32,0u32);
    let count=raw.len().div_ceil(65535);
    for(i,block)in raw.chunks(65535).enumerate(){
        let n=block.len() as u16;z.push(u8::from(i+1==count));z.extend_from_slice(&n.to_le_bytes());z.extend_from_slice(&(!n).to_le_bytes());z.extend_from_slice(block);
        for &v in block{a=(a+u32::from(v))%65521;b=(b+a)%65521;}
    }
    z.extend_from_slice(&((b<<16)|a).to_be_bytes());
    let mut out=b"\x89PNG\r\n\x1a\n".to_vec();let mut ihdr=Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());ihdr.extend_from_slice(&(h as u32).to_be_bytes());ihdr.extend_from_slice(&[8,6,0,0,0]);
    chunk(&mut out,b"IHDR",&ihdr);chunk(&mut out,b"IDAT",&z);chunk(&mut out,b"IEND",&[]);Ok(out)
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn crc_reference(){assert_eq!(crc32(b"123456789"),0xcbf43926);}
}
