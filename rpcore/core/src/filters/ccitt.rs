//! Native CCITT T.6 MSB-first bitstream, run-length codes and 2D modes.
//! Tables 1–3 of T.6 (1988), shared primitives for JBIG2 and PDF fax.
//! This stage does not reconstruct scanlines or integrate PDF filters.
use crate::{Error,Result,MAX_BYTES};
pub const MAX_RUN:u32=1_048_576;
pub const MAX_BITS:usize=200_000_000;

const WHITE:[&str;104]=[
    "00110101","000111","0111","1000","1011","1100","1110","1111",
    "10011","10100","00111","01000","001000","000011","110100","110101",
    "101010","101011","0100111","0001100","0001000","0010111","0000011","0000100",
    "0101000","0101011","0010011","0100100","0011000","00000010","00000011","00011010",
    "00011011","00010010","00010011","00010100","00010101","00010110","00010111","00101000",
    "00101001","00101010","00101011","00101100","00101101","00000100","00000101","00001010",
    "00001011","01010010","01010011","01010100","01010101","00100100","00100101","01011000",
    "01011001","01011010","01011011","01001010","01001011","00110010","00110011","00110100",
    "11011","10010","010111","0110111","00110110","00110111","01100100","01100101",
    "01101000","01100111","011001100","011001101","011010010","011010011","011010100","011010101",
    "011010110","011010111","011011000","011011001","011011010","011011011","010011000","010011001",
    "010011010","011000","010011011","00000001000","00000001100","00000001101","000000010010","000000010011",
    "000000010100","000000010101","000000010110","000000010111","000000011100","000000011101","000000011110","000000011111",
];
const BLACK:[&str;104]=[
    "0000110111","010","11","10","011","0011","0010","00011",
    "000101","000100","0000100","0000101","0000111","00000100","00000111","000011000",
    "0000010111","0000011000","0000001000","00001100111","00001101000","00001101100","00000110111","00000101000",
    "00000010111","00000011000","000011001010","000011001011","000011001100","000011001101","000001101000","000001101001",
    "000001101010","000001101011","000011010010","000011010011","000011010100","000011010101","000011010110","000011010111",
    "000001101100","000001101101","000011011010","000011011011","000001010100","000001010101","000001010110","000001010111",
    "000001100100","000001100101","000001010010","000001010011","000000100100","000000110111","000000111000","000000100111",
    "000000101000","000001011000","000001011001","000000101011","000000101100","000001011010","000001100110","000001100111",
    "0000001111","000011001000","000011001001","000001011011","000000110011","000000110100","000000110101","0000001101100",
    "0000001101101","0000001001010","0000001001011","0000001001100","0000001001101","0000001110010","0000001110011","0000001110100",
    "0000001110101","0000001110110","0000001110111","0000001010010","0000001010011","0000001010100","0000001010101","0000001011010",
    "0000001011011","0000001100100","0000001100101","00000001000","00000001100","00000001101","000000010010","000000010011",
    "000000010100","000000010101","000000010110","000000010111","000000011100","000000011101","000000011110","000000011111",
];

// Fixed compile-time prefix trees: no per-run allocation or linear scan.
#[derive(Clone,Copy)]
struct Node{next:[u16;2],value:i16}
const EMPTY:Node=Node{next:[0;2],value:-1};
const fn tree(codes:&[&str;104])->[Node;256]{
    let mut nodes=[EMPTY;256];let mut used=1usize;let mut i=0usize;
    while i<codes.len(){
        let s=codes[i].as_bytes();let mut p=0usize;let mut j=0usize;
        while j<s.len(){
            assert!(nodes[p].value<0);
            assert!(s[j]==b'0'||s[j]==b'1');
            let k=(s[j]-b'0')as usize;
            if nodes[p].next[k]==0{
                assert!(used<nodes.len());
                nodes[p].next[k]=used as u16;used+=1;
            }
            p=nodes[p].next[k]as usize;j+=1;
        }
        assert!(nodes[p].value<0&&nodes[p].next[0]==0&&nodes[p].next[1]==0);
        nodes[p].value=if i<64{i as i16}else{((i-63)*64)as i16};
        i+=1;
    }
    nodes
}
static WHITE_TREE:[Node;256]=tree(&WHITE);
static BLACK_TREE:[Node;256]=tree(&BLACK);

pub struct BitReader<'a>{data:&'a[u8],pos:usize,end:usize}
impl<'a> BitReader<'a>{
    pub fn new(data:&'a[u8],bit_len:usize)->Result<Self>{
        if data.len()>MAX_BYTES||bit_len>MAX_BITS{return Err(Error::Limit("CCITT input"));}
        if bit_len>data.len().saturating_mul(8){return Err(Error::Invalid("CCITT bit length"));}
        Ok(Self{data,pos:0,end:bit_len})
    }
    pub fn position(&self)->usize{self.pos}
    pub fn remaining(&self)->usize{self.end-self.pos}
    pub fn bit(&mut self)->Result<u8>{
        if self.pos>=self.end{return Err(Error::Invalid("CCITT truncated bits"));}
        let b=(self.data[self.pos/8]>>(7-self.pos%8))&1;
        self.pos+=1;Ok(b)
    }
    fn codeword(&mut self,black:bool)->Result<u32>{
        let nodes=if black{&BLACK_TREE}else{&WHITE_TREE};
        let mut p=0usize;
        for _ in 0..13{
            let k=self.bit()? as usize;
            p=nodes[p].next[k]as usize;
            if p==0{return Err(Error::Invalid("CCITT run prefix"));}
            if nodes[p].value>=0{return Ok(nodes[p].value as u32);}
        }
        Err(Error::Invalid("CCITT run code"))
    }
    /// A run ends only after a terminating code (including zero).
    /// A caller-provided remaining row width bounds makeup accumulation.
    /// Any error may advance the reader; callers must abort the decode.
    pub fn run(&mut self,black:bool,remaining_width:u32)->Result<u32>{
        if remaining_width>MAX_RUN{return Err(Error::Limit("CCITT run width"));}
        let mut total=0u32;
        loop{
            let n=self.codeword(black)?;
            total=total.checked_add(n).ok_or(Error::Limit("CCITT run"))?;
            if total>remaining_width{return Err(Error::Invalid("CCITT run exceeds row"));}
            if n<64{return Ok(total);}
        }
    }
    /// T.6 2D mode and exact two-EOL EOFB recognition. Extensions and
    /// uncompressed mode are intentionally unsupported.
    pub fn mode(&mut self)->Result<Mode>{
        if self.bit()?==1{return Ok(Mode::Vertical(0));}
        if self.bit()?==1{return Ok(Mode::Vertical(if self.bit()?==1{1}else{-1}));}
        if self.bit()?==1{return Ok(Mode::Horizontal);}
        if self.bit()?==1{return Ok(Mode::Pass);}
        if self.bit()?==1{return Ok(Mode::Vertical(if self.bit()?==1{2}else{-2}));}
        if self.bit()?==1{return Ok(Mode::Vertical(if self.bit()?==1{3}else{-3}));}
        if self.bit()?==1{return Err(Error::Unsupported("CCITT extension mode"));}
        // Seven zeroes consumed. First EOL is eleven zeroes then one.
        for _ in 0..4{if self.bit()?!=0{return Err(Error::Invalid("CCITT mode prefix"));}}
        if self.bit()?!=1{return Err(Error::Invalid("CCITT EOL"));}
        for _ in 0..11{if self.bit()?!=0{return Err(Error::Invalid("CCITT EOFB"));}}
        if self.bit()?!=1{return Err(Error::Invalid("CCITT EOFB"));}
        Ok(Mode::EndOfBlock)
    }
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Mode{Pass,Horizontal,Vertical(i8),EndOfBlock}

#[cfg(test)]
mod tests{
    use super::*;
    fn pack(s:&str)->Vec<u8>{
        let mut b=vec![0;s.len().div_ceil(8)];
        for (i,c) in s.bytes().enumerate(){if c==b'1'{b[i/8]|=128>>(i%8);}}
        b
    }
    fn fingerprint(a:&[&str;104])->u32{
        let mut h=2166136261u32;
        for s in a{
            let v=u16::from_str_radix(s,2).unwrap();
            for b in [s.len()as u8,(v>>8)as u8,v as u8]{
                h=(h^u32::from(b)).wrapping_mul(16777619);
            }
        }
        h
    }
    #[test]fn ccitt_normative_table_fingerprints(){
        // Offline independent pdfminer.six 20251230 reference table
        // fingerprints (length, code high byte, code low byte).
        assert_eq!(fingerprint(&WHITE),0xbadb79b4);
        assert_eq!(fingerprint(&BLACK),0xdeae6bca);
    }
    #[test]fn ccitt_all_codewords_and_prefixes(){
        for (black,a) in [(false,&WHITE),(true,&BLACK)]{
            for (i,s) in a.iter().enumerate(){
                let data=pack(s);let mut r=BitReader::new(&data,s.len()).unwrap();
                assert_eq!(r.codeword(black).unwrap(),if i<64{i as u32}else{((i-63)*64)as u32});
                assert_eq!(r.remaining(),0);
                for n in 0..s.len(){
                    let mut r=BitReader::new(&data,n).unwrap();assert!(r.codeword(black).is_err());
                }
            }
        }
    }
    #[test]fn ccitt_all_makeup_terminating_pairs(){
        for (black,a) in [(false,&WHITE),(true,&BLACK)]{
            for i in 64..104{for j in 0..64{
                let s=format!("{}{}",a[i],a[j]);let data=pack(&s);
                let want=((i-63)*64+j)as u32;
                let mut r=BitReader::new(&data,s.len()).unwrap();
                assert_eq!(r.run(black,want).unwrap(),want);
                assert_eq!(r.remaining(),0);
                let mut r=BitReader::new(&data,s.len()).unwrap();
                assert!(r.run(black,want-1).is_err());
            }}
        }
    }
    #[test]fn ccitt_literal_runs_and_bit_alignment(){
        // White 0, black 3, white 64+2, black 2560+2560+7.
        // Construct explicit independently checked codewords to avoid
        // visual ambiguity in the concatenated representation.
        let bits=["00110101","10","11011","0111","000000011111","000000011111","00011"].concat();
        let data=pack(&bits);let mut r=BitReader::new(&data,bits.len()).unwrap();
        for (black,want) in [(false,0),(true,3),(false,66),(true,5127)]{
            assert_eq!(r.run(black,want).unwrap(),want);
        }
        assert_eq!(r.position(),bits.len());
    }
    #[test]fn ccitt_modes_and_eofb(){
        let modes=[("0001",Mode::Pass),("001",Mode::Horizontal),("1",Mode::Vertical(0)),
            ("011",Mode::Vertical(1)),("010",Mode::Vertical(-1)),("000011",Mode::Vertical(2)),
            ("000010",Mode::Vertical(-2)),("0000011",Mode::Vertical(3)),("0000010",Mode::Vertical(-3)),
            ("000000000001000000000001",Mode::EndOfBlock)];
        let s=modes.iter().map(|(b,_)|*b).collect::<Vec<_>>().concat();
        let data=pack(&s);let mut r=BitReader::new(&data,s.len()).unwrap();
        for (b,m) in modes{
            assert_eq!(r.mode().unwrap(),m);
            let d=pack(b);for n in 0..b.len(){assert!(BitReader::new(&d,n).unwrap().mode().is_err());}
        }
        assert_eq!(r.remaining(),0);
    }
    #[test]fn ccitt_malformed_and_bounds(){
        assert!(BitReader::new(&[0],9).is_err());
        assert!(matches!(BitReader::new(&[],MAX_BITS+1),Err(Error::Limit(_))));
        for s in ["0000000000000","000000000001"]{
            let d=pack(s);for black in [false,true]{
                assert!(BitReader::new(&d,s.len()).unwrap().run(black,64).is_err());
            }
        }
        for s in ["000000000001","000000000001000000000000","000000010000"]{
            let d=pack(s);assert!(BitReader::new(&d,s.len()).unwrap().mode().is_err());
        }
        let d=pack("0000001");assert!(matches!(BitReader::new(&d,7).unwrap().mode(),Err(Error::Unsupported(_))));
        let d=pack("11011");assert!(BitReader::new(&d,5).unwrap().run(false,64).is_err());
        assert!(matches!(BitReader::new(&d,5).unwrap().run(false,MAX_RUN+1),Err(Error::Limit(_))));
    }
}
