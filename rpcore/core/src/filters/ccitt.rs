//! Native CCITT T.6 MSB-first bitstream, run-length codes and 2D modes.
//! Tables 1–3 of T.6 (1988), shared primitives for JBIG2 and PDF fax.
//! Scoped Group 4 strip reconstruction; no PDF filter integration.
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
    /// Advance over sub-byte alignment and an exactly bounded byte block.
    /// Failure leaves the cursor unchanged; no byte beyond the bit end is read.
    pub(crate) fn take_aligned_bytes(&mut self,n:usize)->Result<&'a[u8]>{
        let start=self.pos.div_ceil(8);
        let end=start.checked_add(n).ok_or(Error::Limit("CCITT aligned bytes"))?;
        let bits=end.checked_mul(8).ok_or(Error::Limit("CCITT aligned bits"))?;
        if bits>self.end{return Err(Error::Invalid("CCITT truncated aligned bytes"));}
        let block=self.data.get(start..end).ok_or(Error::Invalid("CCITT aligned span"))?;
        self.pos=bits;Ok(block)
    }
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


pub const MAX_MMR_PIXELS:u64=100_000_000;
pub const MAX_MMR_BYTES:usize=16*1024*1024;
pub const MAX_MMR_WORK:u64=200_000_000;
#[derive(Debug,PartialEq,Eq)]
pub struct MmrImage{
    pub width:u32,pub height:u32,pub stride:usize,pub packed:Vec<u8>,
    pub mode_counts:[u64;9],
    pub encoded_bits:usize,
    pub eofb_present:bool,
}
fn charge(left:&mut u64,n:u64)->Result<()>{
    *left=left.checked_sub(n).ok_or(Error::Limit("CCITT MMR work"))?;
    Ok(())
}
fn reference_pair(refs:&[u32],x:u32,black:bool,initial:bool,width:u32)->(u32,u32){
    // Actual changes only. Even index changes white->black, odd black->white.
    // At start, a0 is imaginary white to the left of pixel zero.
    let mut i=refs.partition_point(|&p|if initial{p<x}else{p<=x});
    if i%2!=usize::from(black){i+=1;}
    (refs.get(i).copied().unwrap_or(width),refs.get(i+1).copied().unwrap_or(width))
}
fn paint(row:&mut[u8],start:u32,end:u32,black:bool){
    if black{for x in start..end{row[x as usize/8]|=128>>(x%8);}}
}
/// Decode one MSB-first Group 4 strip with a white initial reference row,
/// known dimensions, exact EOFB, and at most seven zero byte-padding bits.
/// Returned pixels are MSB-first black=1, unused row bits zero. No byte
/// alignment between rows, uncompressed extension, multi-strip TIFF parser,
/// error recovery or PDF-specific EndOfBlock policy is implemented here.
pub fn decode_mmr(data:&[u8],width:u32,height:u32,work_limit:u64)->Result<MmrImage>{
    decode_mmr_inner(data,width,height,work_limit,false,false)
}
/// T.88 6.2.6: for a single bitmap with a known byte count, EOFB
/// is optional. Accept either exact row data plus up to seven alignment
/// bits, or exact EOFB plus up to seven alignment bits. Alignment bits
/// are ignored (T.88 does not prescribe their value). Reject any other
/// full trailing bytes, a deliberately strict subset of optional EOFB.
pub fn decode_mmr_known_length(data:&[u8],width:u32,height:u32,work_limit:u64)->Result<MmrImage>{
    decode_mmr_inner(data,width,height,work_limit,true,false)
}
/// Decode one explicitly terminated, byte-aligned plane at the beginning
/// of a concatenated T.88 Gray-plane stream. EOFB mandatory; arbitrary
/// alignment bits allowed. The next plane is untouched. Work includes only
/// this plane's consumed bits, not the remaining suffix.
pub(crate) fn decode_mmr_plane_prefix(data:&[u8],width:u32,height:u32,work_limit:u64)->Result<MmrImage>{
    decode_mmr_inner(data,width,height,work_limit,false,true)
}
fn decode_mmr_inner(data:&[u8],width:u32,height:u32,work_limit:u64,optional_eofb:bool,prefix:bool)->Result<MmrImage>{
    if width==0||height==0{return Err(Error::Invalid("CCITT MMR dimensions"));}
    let area=u64::from(width)*u64::from(height);
    if width>MAX_RUN||height>MAX_RUN||area>MAX_MMR_PIXELS{return Err(Error::Limit("CCITT MMR dimensions"));}
    let stride=(width as usize).div_ceil(8);
    let len=stride.checked_mul(height as usize).ok_or(Error::Limit("CCITT MMR bytes"))?;
    if len>MAX_MMR_BYTES{return Err(Error::Limit("CCITT MMR bytes"));}
    let bit_len=data.len().checked_mul(8).ok_or(Error::Limit("CCITT MMR input"))?;
    let mut left=work_limit.min(MAX_MMR_WORK);
    // Prefix path bounds the reader by remaining work; charge actual
    // consumption after each row mode and the mandatory marker/alignment.
    // Legacy complete-image paths retain their exact input-bit charge.
    charge(&mut left,area*2)?;
    let mut reader=BitReader::new(data,if prefix{bit_len.min(left as usize)}else{bit_len})?;
    if !prefix{charge(&mut left,bit_len as u64)?;}
    let mut packed=Vec::new();
    packed.try_reserve_exact(len).map_err(|_|Error::Limit("CCITT MMR allocation"))?;
    packed.resize(len,0);
    let mut refs=Vec::<u32>::new();
    refs.try_reserve_exact(width as usize).map_err(|_|Error::Limit("CCITT MMR reference allocation"))?;
    let mut mode_counts=[0u64;9];
    for row in packed.chunks_exact_mut(stride){
        let mut x=0u32;let mut black=false;let mut initial=true;
        while x<width{
            charge(&mut left,32)?;
            let start=reader.position();
            let (b1,b2)=reference_pair(&refs,x,black,initial,width);
            match reader.mode()?{
                Mode::Pass=>{
                    if b2<=x{return Err(Error::Invalid("CCITT MMR pass progress"));}
                    paint(row,x,b2,black);x=b2;mode_counts[0]+=1;
                }
                Mode::Horizontal=>{
                    let a=x+reader.run(black,width-x)?;
                    let b=a+reader.run(!black,width-a)?;
                    if b<=x{return Err(Error::Invalid("CCITT MMR horizontal progress"));}
                    paint(row,x,a,black);paint(row,a,b,!black);x=b;mode_counts[1]+=1;
                }
                Mode::Vertical(delta)=>{
                    let a=i64::from(b1)+i64::from(delta);
                    if a<i64::from(x)||a>i64::from(width)||(a==i64::from(x)&&!initial){
                        return Err(Error::Invalid("CCITT MMR vertical position"));
                    }
                    let a=a as u32;
                    paint(row,x,a,black);x=a;black=!black;
                    mode_counts[(i16::from(delta)+5)as usize]+=1;
                }
                Mode::EndOfBlock=>return Err(Error::Invalid("CCITT MMR early EOFB")),
            }
            if prefix{charge(&mut left,(reader.position()-start) as u64)?;}
            initial=false;
        }
        // Rebuild canonical changing elements from actual pixels: zero
        // length horizontal runs cannot create duplicate reference changes.
        refs.clear();let mut previous=false;
        for x in 0..width{
            let bit=row[x as usize/8]&(128>>(x%8))!=0;
            if bit!=previous{refs.push(x);previous=bit;}
        }
    }
    let marker_start=reader.position();
    let eofb_present=if optional_eofb&&reader.remaining()<=7{false}else{
        if reader.mode()?!=Mode::EndOfBlock{return Err(Error::Invalid("CCITT MMR missing EOFB"));}
        true
    };
    let encoded_bits=reader.position();
    if prefix{
        let aligned=encoded_bits.div_ceil(8)*8;
        charge(&mut left,(aligned-marker_start) as u64)?;
        reader.take_aligned_bytes(0)?;
    }else{
        if reader.remaining()>7{return Err(Error::Invalid("CCITT MMR trailing bytes"));}
        while reader.remaining()!=0{
            let bit=reader.bit()?;
            if !optional_eofb&&bit!=0{return Err(Error::Invalid("CCITT MMR nonzero padding"));}
        }
    }
    Ok(MmrImage{width,height,stride,packed,mode_counts,encoded_bits,eofb_present})
}

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

    // Independent Pillow 12.3.0/libtiff 4.7.1 Group 4 single strips,
    // TIFF PhotometricInterpretation=1, default FillOrder=1; reference
    // samples read from original bitmaps and independently checked via
    // pdfminer CCITTFaxDecoder(reversed=True). No runtime library dependency.
    fn hex(s:&str)->Vec<u8>{
        s.as_bytes().chunks_exact(2).map(|b|u8::from_str_radix(std::str::from_utf8(b).unwrap(),16).unwrap()).collect()
    }
    fn mmr_case(w:u32,h:u32,code:&str,want:&str)->MmrImage{
        let b=hex(code);let a=decode_mmr(&b,w,h,MAX_MMR_WORK).unwrap();
        assert_eq!((a.width,a.height,a.stride),(w,h,(w as usize).div_ceil(8)));
        assert_eq!(a.packed,hex(want));
        assert_eq!(a,decode_mmr(&b,w,h,MAX_MMR_WORK).unwrap());
        a
    }
    #[test]fn mmr_white_black_reference(){
        mmr_case(13,4,"f0010010","0000000000000000");
        mmr_case(13,4,"26a09f80080080","fff8fff8fff8fff8");
    }
    #[test]fn mmr_shifted_reference(){
        mmr_case(19,9,"304095b1b76c12b636ed82530436e0020020",
            "1f80c00fc00007e00003f0c001f80000fc00007ec01f80000fc000");
    }
    #[test]fn mmr_dense_reference(){
        let a=mmr_case(17,7,"26a8ba308be0961b4d88864d63905845d2045f4ac260924102d2a4105e924105b0010010",
            "913300e0e180000000e0e1809133004a5280aaaa00");
        let b=mmr_case(16,5,"26a88e88e88e88e88e88e82569249249249524924924924ad2492492492a492492492494004004",
            "aaaa5555aaaa5555aaaa");
        assert!(a.mode_counts[0]+b.mode_counts[0]>0);
        assert!(a.mode_counts[1]+b.mode_counts[1]>0);
    }
    #[test]fn mmr_narrow_reference(){
        mmr_case(1,7,"aed760020020","00808000808000");
    }
    #[test]fn mmr_long_makeup_reference(){
        let a=decode_mmr(&hex("26a03e1920c9193501f0c90648008008"),3001,3,MAX_MMR_WORK).unwrap();
        assert_eq!(a.stride,376);
        for (y,row) in a.packed.chunks_exact(376).enumerate(){
            for x in 0..3001{assert_eq!(row[x/8]&(128>>(x%8))!=0,y!=1&&x<2800);}
            assert_eq!(row[375]&127,0);
        }
    }
    #[test]fn mmr_strict_eofb_and_padding(){
        let data=hex("f0010010");
        for end in 0..data.len(){assert!(decode_mmr(&data[..end],13,4,MAX_MMR_WORK).is_err());}
        let mut b=data.clone();b.push(0);assert!(decode_mmr(&b,13,4,MAX_MMR_WORK).is_err());
        let mut b=data.clone();*b.last_mut().unwrap()|=1;assert!(decode_mmr(&b,13,4,MAX_MMR_WORK).is_err());
        assert!(decode_mmr(&data,13,3,MAX_MMR_WORK).is_err());
        assert!(decode_mmr(&data,13,5,MAX_MMR_WORK).is_err());
    }
    #[test]fn mmr_bounds_and_progress(){
        let data=hex("f0010010");
        assert!(matches!(decode_mmr(&data,13,4,0),Err(Error::Limit(_))));
        assert!(matches!(decode_mmr(&data,13,4,13*4*2+32+4*32-1),Err(Error::Limit(_))));
        assert!(decode_mmr(&data,13,4,13*4*2+32+4*32).is_ok());
        assert!(decode_mmr(&data,0,4,MAX_MMR_WORK).is_err());
        assert!(decode_mmr(&data,MAX_RUN+1,1,MAX_MMR_WORK).is_err());
        assert!(decode_mmr(&data,100_000,100_000,MAX_MMR_WORK).is_err());
        // H with white0 and black0; V+1 beyond a white reference row;
        // EOFB before any pixels.
        for b in [pack("001001101010000110111"),pack("011"),pack("000000000001000000000001")]{
            assert!(decode_mmr(&b,13,1,MAX_MMR_WORK).is_err());
        }
    }

    #[test]fn mmr_known_length_optional_eofb(){
        // Four white rows: four V0 bits. With and without EOFB.
        for data in [vec![0xf0],vec![0xff],hex("f0010010"),hex("f001001f")]{
            let a=decode_mmr_known_length(&data,13,4,MAX_MMR_WORK).unwrap();
            assert_eq!(a.packed,vec![0;8]);assert_eq!(a.eofb_present,data.len()==4);
            assert_eq!(a.encoded_bits,if a.eofb_present{28}else{4});
        }
        assert!(decode_mmr(&[0xf0],13,4,MAX_MMR_WORK).is_err());
        assert!(decode_mmr(&hex("f001001f"),13,4,MAX_MMR_WORK).is_err());
        assert!(decode_mmr(&hex("f0010010"),13,4,MAX_MMR_WORK).unwrap().eofb_present);
        let a=decode_mmr_known_length(&[255],1,8,MAX_MMR_WORK).unwrap();
        assert_eq!(a.encoded_bits,8);assert!(!a.eofb_present);
    }
    #[test]fn mmr_known_length_rejects_extra_and_partial_rows(){
        for s in ["f000","f001","f00100","f001001000","00","e0"]{
            assert!(decode_mmr_known_length(&hex(s),13,4,MAX_MMR_WORK).is_err(),"{s}");
        }
        assert!(decode_mmr_known_length(&[255],1,9,MAX_MMR_WORK).is_err());
        assert!(decode_mmr_known_length(&[],13,4,MAX_MMR_WORK).is_err());
        assert!(decode_mmr_known_length(&[0xf0],13,4,0).is_err());
    }
}
