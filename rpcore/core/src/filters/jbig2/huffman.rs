//! Native JBIG2 canonical Huffman integer decoding (T.88 B.3/B.4).
//! Rows in specification order; zero-length rows disabled. No type 53 parser.
use crate::{Error,Result};
use super::integer::Integer;
use crate::filters::ccitt::BitReader;

pub const MAX_PREFIX:u8=32;
pub const MAX_ROWS:usize=4096;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Range {
    Normal { base:i64, extra:u8 },
    Lower { base:i64 },
    Upper { base:i64 },
    Oob,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Row { pub prefix_len:u8, pub range:Range }
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Code { pub bits:u32, pub len:u8 }

#[derive(Default)]
struct Node { child:[Option<usize>;2], row:Option<usize> }

pub struct Table {
    rows:Vec<Row>,
    codes:Vec<Option<Code>>,
    nodes:Vec<Node>,
}

impl Table {
    pub fn new(rows:&[Row])->Result<Self>{
        if rows.is_empty(){return Err(Error::Invalid("JBIG2 empty Huffman table"));}
        if rows.len()>MAX_ROWS{return Err(Error::Limit("JBIG2 Huffman rows"));}
        let mut count=[0u64;33];
        let mut specials=[false;3];
        let mut node_cap=1usize;
        for row in rows {
            if row.prefix_len>MAX_PREFIX{return Err(Error::Limit("JBIG2 Huffman prefix length"));}
            let (base,extra,special)=match row.range {
                Range::Normal{base,extra}=>(Some(base),extra,None),
                Range::Lower{base}=>(Some(base),32,Some(0usize)),
                Range::Upper{base}=>(Some(base),32,Some(1usize)),
                Range::Oob=>(None,0,Some(2usize)),
            };
            if extra>32{return Err(Error::Limit("JBIG2 Huffman range length"));}
            if let Some(i)=special{
                if specials[i]{return Err(Error::Invalid("JBIG2 duplicate Huffman special row"));}
                specials[i]=true;
            }
            if row.prefix_len==0{continue;}
            if let Some(v)=base{
                if i32::try_from(v).is_err(){return Err(Error::Invalid("JBIG2 Huffman base range"));}
            }
            count[usize::from(row.prefix_len)]+=1;
            node_cap+=usize::from(row.prefix_len);
        }
        if node_cap==1{return Err(Error::Invalid("JBIG2 no enabled Huffman rows"));}
        // u64 retains the exclusive endpoint of the 32-bit code space.
        let mut next=[0u64;33];
        for len in 1..=usize::from(MAX_PREFIX){
            next[len]=(next[len-1]+count[len-1])*2;
            if next[len]+count[len]>(1u64<<len){
                return Err(Error::Invalid("JBIG2 oversubscribed Huffman table"));
            }
        }
        let mut codes=vec![None;rows.len()];
        let mut nodes=Vec::new();
        nodes.try_reserve_exact(node_cap).map_err(|_|Error::Limit("JBIG2 Huffman tree"))?;
        nodes.push(Node::default());
        // Preserve original row order among equal prefix lengths.
        for (i,row) in rows.iter().enumerate(){
            let len=row.prefix_len;
            if len==0{continue;}
            let n=usize::from(len);
            let bits=u32::try_from(next[n]).map_err(|_|Error::Limit("JBIG2 Huffman code"))?;
            next[n]+=1;
            codes[i]=Some(Code{bits,len});
            let mut p=0usize;
            for shift in (0..len).rev(){
                if nodes[p].row.is_some(){return Err(Error::Invalid("JBIG2 Huffman prefix collision"));}
                let k=((bits>>shift)&1)as usize;
                p=match nodes[p].child[k]{
                    Some(child)=>child,
                    None=>{
                        let child=nodes.len();
                        nodes.push(Node::default());
                        nodes[p].child[k]=Some(child);
                        child
                    }
                };
            }
            if nodes[p].row.is_some()||nodes[p].child.iter().any(Option::is_some){
                return Err(Error::Invalid("JBIG2 Huffman code collision"));
            }
            nodes[p].row=Some(i);
        }
        Ok(Self{rows:rows.to_vec(),codes,nodes})
    }
    pub fn codes(&self)->&[Option<Code>]{&self.codes}
    /// At most 32 prefix and 32 range bits per value. The shared reader
    /// bounds total input bits. Errors may advance it; abort on error.
    pub fn decode(&self,reader:&mut BitReader<'_>)->Result<Integer>{
        let mut p=0usize;
        for _ in 0..MAX_PREFIX{
            let bit=usize::from(reader.bit()?);
            p=self.nodes[p].child[bit].ok_or(Error::Invalid("JBIG2 unused Huffman prefix"))?;
            if let Some(i)=self.nodes[p].row{
                let (base,extra,subtract)=match self.rows[i].range{
                    Range::Normal{base,extra}=>(base,extra,false),
                    Range::Lower{base}=>(base,32,true),
                    Range::Upper{base}=>(base,32,false),
                    Range::Oob=>return Ok(Integer::Oob),
                };
                let mut offset=0i64;
                for _ in 0..extra{offset=(offset<<1)|i64::from(reader.bit()?);}
                let value=if subtract{base.checked_sub(offset)}else{base.checked_add(offset)}
                    .ok_or(Error::Limit("JBIG2 Huffman integer overflow"))?;
                if i32::try_from(value).is_err(){return Err(Error::Invalid("JBIG2 Huffman integer range"));}
                return Ok(Integer::Value(value));
            }
        }
        Err(Error::Invalid("JBIG2 Huffman prefix"))
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn row(prefix_len:u8,base:i64,extra:u8)->Row{
        Row{prefix_len,range:Range::Normal{base,extra}}
    }
    fn code(bits:u32,len:u8)->Option<Code>{Some(Code{bits,len})}
    #[test]
    fn huffman_canonical_stable_order(){
        let t=Table::new(&[row(3,10,0),row(0,99,0),row(1,20,0),row(3,30,0),row(2,40,0)]).unwrap();
        assert_eq!(t.codes(),&[code(6,3),None,code(0,1),code(7,3),code(2,2)]);
        let mut r=BitReader::new(&[0xcf,0],9).unwrap();
        for v in [10,20,30,40]{assert_eq!(t.decode(&mut r).unwrap(),Integer::Value(v));}
        assert_eq!(r.remaining(),0);
    }
    #[test]
    fn huffman_annex_b1_reference(){
        let t=Table::new(&[row(1,0,4),row(2,16,8),row(3,272,16),
            Row{prefix_len:0,range:Range::Lower{base:-1}},
            Row{prefix_len:3,range:Range::Upper{base:65808}}]).unwrap();
        assert_eq!(t.codes(),&[code(0,1),code(2,2),code(6,3),None,code(7,3)]);
        // Python packed explicit published prefixes independently of this table.
        let bytes=[0x03,0xe0,0x0b,0xff,0x00,0x00,0x6f,0xff,0xfe,0x00,0x00,0x00,0x01,0xdf,0xff,0xbf,0xbb,0xc0];
        let mut r=BitReader::new(&bytes,138).unwrap();
        for v in [0,15,16,271,272,65807,65808,2147483647]{assert_eq!(t.decode(&mut r).unwrap(),Integer::Value(v));}
        assert_eq!(r.remaining(),0);
    }
    #[test]
    fn huffman_annex_b3_reference(){
        let t=Table::new(&[row(8,-256,8),row(1,0,0),row(2,1,0),row(3,2,0),row(4,3,3),row(5,11,6),
            Row{prefix_len:8,range:Range::Lower{base:-257}},
            Row{prefix_len:7,range:Range::Upper{base:75}},
            Row{prefix_len:6,range:Range::Oob}]).unwrap();
        assert_eq!(t.codes(),&[code(254,8),code(0,1),code(2,2),code(6,3),code(14,4),code(30,5),code(255,8),code(126,7),code(62,6)]);
        let bytes=[0xfe,0x00,0xfe,0xff,0x7d,0xfe,0x00,0x00,0x00,0x01,0xfe,0xff,0xff,0xfd,0xff,0xf8,0x00,0x00,0x00,0x00];
        let mut r=BitReader::new(&bytes,158).unwrap();
        for v in [Integer::Value(-256),Integer::Value(-1),Integer::Value(0),Integer::Oob,Integer::Value(-257),Integer::Value(-2147483648),Integer::Value(75)]{
            assert_eq!(t.decode(&mut r).unwrap(),v);
        }
        assert_eq!(r.remaining(),0);
    }
    #[test]
    fn huffman_incomplete_and_truncated(){
        let t=Table::new(&[row(2,5,3)]).unwrap();
        assert!(t.decode(&mut BitReader::new(&[0x80],1).unwrap()).is_err());
        for n in 0..5{assert!(t.decode(&mut BitReader::new(&[0],n).unwrap()).is_err());}
        assert_eq!(t.decode(&mut BitReader::new(&[0x38],5).unwrap()).unwrap(),Integer::Value(12));
    }
    #[test]
    fn huffman_invalid_tables(){
        assert!(Table::new(&[]).is_err());
        assert!(Table::new(&[row(0,0,0)]).is_err());
        assert!(Table::new(&[row(1,0,0),row(1,1,0),row(2,2,0)]).is_err());
        assert!(Table::new(&[row(33,0,0)]).is_err());
        assert!(Table::new(&[row(1,0,33)]).is_err());
        assert!(Table::new(&vec![row(13,0,0);MAX_ROWS+1]).is_err());
        let oob=Row{prefix_len:1,range:Range::Oob};
        assert!(Table::new(&[oob,oob]).is_err());
        assert!(Table::new(&[row(1,i64::MAX,0)]).is_err());
    }
    #[test]
    fn huffman_signed_range_limits(){
        for range in [Range::Lower{base:i64::from(i32::MIN)},Range::Upper{base:i64::from(i32::MAX)}]{
            let t=Table::new(&[Row{prefix_len:1,range}]).unwrap();
            let zero=[0;5];
            let v=match range{Range::Lower{base}|Range::Upper{base}=>base,_=>unreachable!()};
            assert_eq!(t.decode(&mut BitReader::new(&zero,33).unwrap()).unwrap(),Integer::Value(v));
            assert!(t.decode(&mut BitReader::new(&[0,0,0,0,128],33).unwrap()).is_err());
            assert!(t.decode(&mut BitReader::new(&[127,255,255,255,128],33).unwrap()).is_err());
        }
    }
    #[test]
    fn huffman_maximum_prefix_and_offset(){
        let t=Table::new(&[row(32,i64::from(i32::MIN),32),row(32,42,0)]).unwrap();
        assert_eq!(t.codes(),&[code(0,32),code(1,32)]);
        let bytes=[0,0,0,0,255,255,255,255,0,0,0,1];
        let mut r=BitReader::new(&bytes,96).unwrap();
        assert_eq!(t.decode(&mut r).unwrap(),Integer::Value(i64::from(i32::MAX)));
        assert_eq!(t.decode(&mut r).unwrap(),Integer::Value(42));
        assert_eq!(r.remaining(),0);
    }
}


/// ITU-T T.88 (02/2000) B.5, tables B.1 through B.15 (A through O).
/// In the fixed registry, omitted extension rows are represented as disabled.
/// Numeric bases and range widths follow the encoding column, not a printed
/// endpoint: B.7's -64 range has five offset bits and ends at -33.
pub mod standard {
    use super::*;
    const fn normal(prefix_len:u8,base:i64,extra:u8)->Row{Row{prefix_len,range:Range::Normal{base,extra}}}
    const fn lower(prefix_len:u8,base:i64)->Row{Row{prefix_len,range:Range::Lower{base}}}
    const fn upper(prefix_len:u8,base:i64)->Row{Row{prefix_len,range:Range::Upper{base}}}
    const fn oob(prefix_len:u8)->Row{Row{prefix_len,range:Range::Oob}}

const A:&[Row]=&[
    normal(1,0,4),
    normal(2,16,8),
    normal(3,272,16),
    lower(0,-1),
    upper(3,65808),
];

const B:&[Row]=&[
    normal(1,0,0),
    normal(2,1,0),
    normal(3,2,0),
    normal(4,3,3),
    normal(5,11,6),
    lower(0,-1),
    upper(6,75),
    oob(6),
];

const C:&[Row]=&[
    normal(8,-256,8),
    normal(1,0,0),
    normal(2,1,0),
    normal(3,2,0),
    normal(4,3,3),
    normal(5,11,6),
    lower(8,-257),
    upper(7,75),
    oob(6),
];

const D:&[Row]=&[
    normal(1,1,0),
    normal(2,2,0),
    normal(3,3,0),
    normal(4,4,3),
    normal(5,12,6),
    lower(0,0),
    upper(5,76),
];

const E:&[Row]=&[
    normal(7,-255,8),
    normal(1,1,0),
    normal(2,2,0),
    normal(3,3,0),
    normal(4,4,3),
    normal(5,12,6),
    lower(7,-256),
    upper(6,76),
];

const F:&[Row]=&[
    normal(5,-2048,10),
    normal(4,-1024,9),
    normal(4,-512,8),
    normal(4,-256,7),
    normal(5,-128,6),
    normal(5,-64,5),
    normal(4,-32,5),
    normal(2,0,7),
    normal(3,128,7),
    normal(3,256,8),
    normal(4,512,9),
    normal(4,1024,10),
    lower(6,-2049),
    upper(6,2048),
];

const G:&[Row]=&[
    normal(4,-1024,9),
    normal(3,-512,8),
    normal(4,-256,7),
    normal(5,-128,6),
    normal(5,-64,5),
    normal(4,-32,5),
    normal(4,0,5),
    normal(5,32,5),
    normal(5,64,6),
    normal(4,128,7),
    normal(3,256,8),
    normal(3,512,9),
    normal(3,1024,10),
    lower(5,-1025),
    upper(5,2048),
];

const H:&[Row]=&[
    normal(8,-15,3),
    normal(9,-7,1),
    normal(8,-5,1),
    normal(9,-3,0),
    normal(7,-2,0),
    normal(4,-1,0),
    normal(2,0,1),
    normal(5,2,0),
    normal(6,3,0),
    normal(3,4,4),
    normal(6,20,1),
    normal(4,22,4),
    normal(4,38,5),
    normal(5,70,6),
    normal(5,134,7),
    normal(6,262,7),
    normal(7,390,8),
    normal(6,646,10),
    lower(9,-16),
    upper(9,1670),
    oob(2),
];

const I:&[Row]=&[
    normal(8,-31,4),
    normal(9,-15,2),
    normal(8,-11,2),
    normal(9,-7,1),
    normal(7,-5,1),
    normal(4,-3,1),
    normal(3,-1,1),
    normal(3,1,1),
    normal(5,3,1),
    normal(6,5,1),
    normal(3,7,5),
    normal(6,39,2),
    normal(4,43,5),
    normal(4,75,6),
    normal(5,139,7),
    normal(5,267,8),
    normal(6,523,8),
    normal(7,779,9),
    normal(6,1291,11),
    lower(9,-32),
    upper(9,3339),
    oob(2),
];

const J:&[Row]=&[
    normal(7,-21,4),
    normal(8,-5,0),
    normal(7,-4,0),
    normal(5,-3,0),
    normal(2,-2,2),
    normal(5,2,0),
    normal(6,3,0),
    normal(7,4,0),
    normal(8,5,0),
    normal(2,6,6),
    normal(5,70,5),
    normal(6,102,5),
    normal(6,134,6),
    normal(6,198,7),
    normal(6,326,8),
    normal(6,582,9),
    normal(6,1094,10),
    normal(7,2118,11),
    lower(8,-22),
    upper(8,4166),
    oob(2),
];

const K:&[Row]=&[
    normal(1,1,0),
    normal(2,2,1),
    normal(4,4,0),
    normal(4,5,1),
    normal(5,7,1),
    normal(5,9,2),
    normal(6,13,2),
    normal(7,17,2),
    normal(7,21,3),
    normal(7,29,4),
    normal(7,45,5),
    normal(7,77,6),
    lower(0,0),
    upper(7,141),
];

const L:&[Row]=&[
    normal(1,1,0),
    normal(2,2,0),
    normal(3,3,1),
    normal(5,5,0),
    normal(5,6,1),
    normal(6,8,1),
    normal(7,10,0),
    normal(7,11,1),
    normal(7,13,2),
    normal(7,17,3),
    normal(7,25,4),
    normal(8,41,5),
    lower(0,0),
    upper(8,73),
];

const M:&[Row]=&[
    normal(1,1,0),
    normal(3,2,0),
    normal(4,3,0),
    normal(5,4,0),
    normal(4,5,1),
    normal(3,7,3),
    normal(6,15,1),
    normal(6,17,2),
    normal(6,21,3),
    normal(6,29,4),
    normal(6,45,5),
    normal(7,77,6),
    lower(0,0),
    upper(7,141),
];

const N:&[Row]=&[
    normal(3,-2,0),
    normal(3,-1,0),
    normal(1,0,0),
    normal(3,1,0),
    normal(3,2,0),
    lower(0,-3),
    upper(0,3),
];

const O:&[Row]=&[
    normal(7,-24,4),
    normal(6,-8,2),
    normal(5,-4,1),
    normal(4,-2,0),
    normal(3,-1,0),
    normal(1,0,0),
    normal(3,1,0),
    normal(4,2,0),
    normal(5,3,1),
    normal(6,5,2),
    normal(7,9,4),
    lower(7,-25),
    upper(7,25),
];

    const TABLES:[&[Row];15]=[A,B,C,D,E,F,G,H,I,J,K,L,M,N,O];
    /// One-based Annex B table number, not a segment-specific selector.
    pub fn rows(number:u8)->Result<&'static [Row]>{
        let index=number.checked_sub(1).ok_or(Error::Invalid("JBIG2 standard Huffman table number"))?;
        TABLES.get(usize::from(index)).copied().ok_or(Error::Invalid("JBIG2 standard Huffman table number"))
    }
    pub fn table(number:u8)->Result<Table>{Table::new(rows(number)?)}
}

#[cfg(test)]
mod standard_tests{
    use super::{standard,Code,Range};
    use super::Integer;
    use crate::filters::ccitt::BitReader;

    // Explicit prefix column transcribed from T.88 B.1-B.15, separate from
    // production canonical assignment. Empty string represents omitted row.
    const PREFIXES:[&[&str];15]=[
        &["0","10","110","","111"],
        &["0","10","110","1110","11110","","111110","111111"],
        &["11111110","0","10","110","1110","11110","11111111","1111110","111110"],
        &["0","10","110","1110","11110","","11111"],
        &["1111110","0","10","110","1110","11110","1111111","111110"],
        &["11100","1000","1001","1010","11101","11110","1011","00","010","011","1100","1101","111110","111111"],
        &["1000","000","1001","11010","11011","1010","1011","11100","11101","1100","001","010","011","11110","11111"],
        &["11111100","111111100","11111101","111111101","1111100","1010","00","11010","111010","100","111011","1011","1100","11011","11100","111100","1111101","111101","111111110","111111111","01"],
        &["11111100","111111100","11111101","111111101","1111100","1010","010","011","11010","111010","100","111011","1011","1100","11011","11100","111100","1111101","111101","111111110","111111111","00"],
        &["1111010","11111100","1111011","11000","00","11001","110110","1111100","11111101","01","11010","110111","111000","111001","111010","111011","111100","1111101","11111110","11111111","10"],
        &["0","10","1100","1101","11100","11101","111100","1111010","1111011","1111100","1111101","1111110","","1111111"],
        &["0","10","110","11100","11101","111100","1111010","1111011","1111100","1111101","1111110","11111110","","11111111"],
        &["0","100","1100","11100","1101","101","111010","111011","111100","111101","111110","1111110","","1111111"],
        &["100","101","0","110","111","",""],
        &["1111100","111100","11100","1100","100","0","101","1101","11101","111101","1111101","1111110","1111111"],
    ];
    // Offline JS encoder used the published prefix column directly, not the
    // Rust canonical generator. Min/mid/max distinct offsets of each range,
    // clipped to signed i32, plus every OOB. Exact bit lengths exclude padding.
    type Fixture=(&'static [u8],usize,&'static [Option<i64>]);
    const FIXTURES:[Fixture;15]=[
        (&[0x01,0xdf,0x00,0x4f,0xf7,0xfe,0x00,0x00,0xcf,0xff,0xfb,0xff,0xff,0x80,0x00,0x00,0x00,0x73,0xff,0xf7,0xf7,0x7e,0xff,0xfd,0xfd,0xde],207,&[Some(0),Some(7),Some(15),Some(16),Some(143),Some(271),Some(272),Some(33039),Some(65807),Some(65808),Some(1073774727),Some(2147483647)]),
        (&[0x5b,0x87,0x3e,0xfe,0x03,0xcf,0xfb,0xff,0x80,0x00,0x00,0x00,0x3e,0x3f,0xff,0xff,0xda,0xf9,0xff,0xff,0xfe,0xd3,0xf0],180,&[Some(0),Some(1),Some(2),Some(3),Some(6),Some(10),Some(11),Some(42),Some(74),Some(75),Some(1073741861),Some(2147483647),None]),
        (&[0xfe,0x00,0xfe,0x7f,0xfe,0xff,0x5b,0x87,0x3e,0xfe,0x03,0xcf,0xfb,0xff,0xf0,0x00,0x00,0x00,0x0f,0xf3,0xff,0xff,0xf7,0xff,0xf7,0xff,0xff,0xef,0xff,0xc0,0x00,0x00,0x00,0x1f,0x8f,0xff,0xff,0xf6,0xbf,0x3f,0xff,0xff,0xda,0x7c],351,&[Some(-256),Some(-129),Some(-1),Some(0),Some(1),Some(2),Some(3),Some(6),Some(10),Some(11),Some(42),Some(74),Some(-257),Some(-1073741952),Some(-2147483648),Some(75),Some(1073741861),Some(2147483647),None]),
        (&[0x5b,0x87,0x3e,0xfe,0x03,0xcf,0xfb,0xff,0x80,0x00,0x00,0x00,0x7c,0xff,0xff,0xff,0x67,0xef,0xff,0xff,0xf6,0x60],171,&[Some(1),Some(2),Some(3),Some(4),Some(7),Some(11),Some(12),Some(43),Some(75),Some(76),Some(1073741861),Some(2147483647)]),
        (&[0xfc,0x01,0xf9,0xff,0xf7,0xfa,0xdc,0x39,0xf7,0xf0,0x1e,0x7f,0xdf,0xff,0x00,0x00,0x00,0x00,0xfe,0x7f,0xff,0xff,0x01,0xfd,0xff,0xff,0xfc,0x03,0xe0,0x00,0x00,0x00,0x0f,0x8f,0xff,0xff,0xf6,0x7e,0x7f,0xff,0xff,0xb3],336,&[Some(-255),Some(-128),Some(0),Some(1),Some(2),Some(3),Some(4),Some(7),Some(11),Some(12),Some(43),Some(75),Some(-256),Some(-1073741952),Some(-2147483648),Some(76),Some(1073741861),Some(2147483647)]),
        (&[0xe0,0x01,0xc7,0xff,0x9f,0xfc,0x00,0x21,0xff,0x1f,0xf9,0x00,0x97,0xf9,0xff,0xa0,0x14,0xfe,0xbf,0xf4,0x0e,0xbf,0xdf,0xfc,0x0f,0x3f,0xdf,0xb0,0x5b,0xef,0xe0,0x01,0xf9,0xfd,0x00,0x4f,0xd7,0xf6,0x00,0xdf,0xdf,0xfe,0x00,0x31,0xff,0x9f,0xfd,0x00,0x35,0xff,0xdf,0xff,0xe0,0x00,0x00,0x00,0x0f,0x8f,0xff,0xfe,0xff,0xfe,0x7f,0xff,0xf7,0xff,0xfc,0x00,0x00,0x00,0x03,0xf3,0xff,0xff,0xbf,0xff,0xdf,0xff,0xfd,0xff,0xc0],642,&[Some(-2048),Some(-1537),Some(-1025),Some(-1024),Some(-769),Some(-513),Some(-512),Some(-385),Some(-257),Some(-256),Some(-193),Some(-129),Some(-128),Some(-97),Some(-65),Some(-64),Some(-49),Some(-33),Some(-32),Some(-17),Some(-1),Some(0),Some(63),Some(127),Some(128),Some(191),Some(255),Some(256),Some(383),Some(511),Some(512),Some(767),Some(1023),Some(1024),Some(1535),Some(2047),Some(-2049),Some(-1073742848),Some(-2147483648),Some(2048),Some(1073742847),Some(2147483647)]),
        (&[0x80,0x04,0x3f,0xe3,0xfe,0x00,0x03,0xf8,0xff,0x90,0x12,0xfe,0x7f,0xe8,0x0d,0x3f,0xaf,0xf6,0x0d,0xbf,0x7f,0xa0,0x53,0xeb,0xf6,0x0b,0x7d,0xff,0x80,0xe3,0xf9,0xfe,0x81,0xd7,0xfb,0xfe,0x00,0xc7,0xf9,0xfc,0x80,0x17,0xf3,0xfe,0x80,0x09,0xfe,0xbf,0xec,0x00,0x6f,0xfb,0xff,0xfc,0x00,0x00,0x00,0x01,0xe3,0xff,0xff,0xdf,0xff,0x3f,0xff,0xfd,0xff,0xfc,0x00,0x00,0x00,0x03,0xe7,0xff,0xff,0x7f,0xff,0x7f,0xff,0xf7,0xff],648,&[Some(-1024),Some(-769),Some(-513),Some(-512),Some(-385),Some(-257),Some(-256),Some(-193),Some(-129),Some(-128),Some(-97),Some(-65),Some(-64),Some(-49),Some(-33),Some(-32),Some(-17),Some(-1),Some(0),Some(15),Some(31),Some(32),Some(47),Some(63),Some(64),Some(95),Some(127),Some(128),Some(191),Some(255),Some(256),Some(383),Some(511),Some(512),Some(767),Some(1023),Some(1024),Some(1535),Some(2047),Some(-1025),Some(-1073742336),Some(-2147483648),Some(2048),Some(1073742847),Some(2147483647)]),
        (&[0xfc,0x1f,0x8f,0xf3,0xff,0x1f,0xcf,0xeb,0xf7,0xfd,0xf9,0x40,0xeb,0xa8,0x11,0xe7,0xf6,0xef,0x61,0x6f,0x7f,0x80,0xc7,0xe7,0xf6,0x06,0xdf,0xdf,0xfc,0x01,0xc7,0xfc,0xff,0xe0,0x0f,0x1f,0xf9,0xff,0xe8,0x07,0xd7,0xff,0xbf,0xfe,0x80,0x1e,0xbf,0xfe,0xff,0xff,0xe0,0x00,0x00,0x00,0x0f,0xf1,0xff,0xff,0xff,0xc7,0xf9,0xff,0xff,0xff,0xc3,0xfe,0x00,0x00,0x00,0x01,0xff,0x3f,0xff,0xfc,0xbc,0xff,0xbf,0xff,0xfc,0xbc,0xa0],643,&[Some(-15),Some(-12),Some(-8),Some(-7),Some(-6),Some(-5),Some(-4),Some(-3),Some(-2),Some(-1),Some(0),Some(1),Some(2),Some(3),Some(4),Some(11),Some(19),Some(20),Some(21),Some(22),Some(29),Some(37),Some(38),Some(53),Some(69),Some(70),Some(101),Some(133),Some(134),Some(197),Some(261),Some(262),Some(325),Some(389),Some(390),Some(517),Some(645),Some(646),Some(1157),Some(1669),Some(-16),Some(-1073741832),Some(-2147483648),Some(1670),Some(1073742658),Some(2147483647),None]),
        (&[0xfc,0x0f,0xc7,0xfc,0xff,0xe1,0xfc,0x7f,0x9f,0xe9,0xfa,0xfe,0xff,0xd7,0xf7,0xf1,0xf3,0x4a,0xa2,0xb3,0xe9,0xaf,0x4e,0xb0,0x11,0xf3,0xfd,0x9d,0xbd,0xf6,0x0b,0x7d,0xff,0x00,0xc7,0xf3,0xfd,0x80,0xdb,0xfd,0xff,0xe0,0x07,0x1f,0xf9,0xff,0xe0,0x07,0x8f,0xfe,0x7f,0xfd,0x00,0x7d,0x7f,0xfd,0xff,0xfa,0x00,0x3d,0x7f,0xfe,0xff,0xff,0xf0,0x00,0x00,0x00,0x07,0xf8,0xff,0xff,0xff,0xc3,0xfc,0xff,0xff,0xff,0xc1,0xff,0x00,0x00,0x00,0x00,0xff,0x9f,0xff,0xfc,0xbd,0x7f,0xdf,0xff,0xfc,0xbd,0x00],756,&[Some(-31),Some(-24),Some(-16),Some(-15),Some(-14),Some(-12),Some(-11),Some(-10),Some(-8),Some(-7),Some(-6),Some(-5),Some(-4),Some(-3),Some(-2),Some(-1),Some(0),Some(1),Some(2),Some(3),Some(4),Some(5),Some(6),Some(7),Some(22),Some(38),Some(39),Some(40),Some(42),Some(43),Some(58),Some(74),Some(75),Some(106),Some(138),Some(139),Some(202),Some(266),Some(267),Some(394),Some(522),Some(523),Some(650),Some(778),Some(779),Some(1034),Some(1290),Some(1291),Some(2314),Some(3338),Some(-32),Some(-1073741840),Some(-2147483648),Some(3339),Some(1073743493),Some(2147483647),None]),
        (&[0xf4,0x1e,0x9f,0xd7,0xfe,0x7b,0xc0,0x09,0xe7,0x6f,0x9f,0xa8,0x0b,0xef,0xfa,0x06,0x9f,0xaf,0xee,0x0d,0xdf,0xbf,0xf8,0x03,0x87,0xf8,0xff,0x90,0x1c,0xbf,0xe7,0xff,0x40,0x1d,0x3f,0xf5,0xff,0xd8,0x03,0xb7,0xff,0x7f,0xff,0x00,0x0f,0x1f,0xff,0x3f,0xff,0xa0,0x03,0xeb,0xff,0xfb,0xff,0xff,0x80,0x00,0x00,0x00,0x3f,0x8f,0xff,0xff,0xfd,0x7f,0x9f,0xff,0xff,0xfa,0xbf,0xc0,0x00,0x00,0x00,0x3f,0xcf,0xff,0xfd,0xf7,0x3f,0xdf,0xff,0xfb,0xee,0x60],684,&[Some(-21),Some(-14),Some(-6),Some(-5),Some(-4),Some(-3),Some(-2),Some(-1),Some(1),Some(2),Some(3),Some(4),Some(5),Some(6),Some(37),Some(69),Some(70),Some(85),Some(101),Some(102),Some(117),Some(133),Some(134),Some(165),Some(197),Some(198),Some(261),Some(325),Some(326),Some(453),Some(581),Some(582),Some(837),Some(1093),Some(1094),Some(1605),Some(2117),Some(2118),Some(3141),Some(4165),Some(-22),Some(-1073741835),Some(-2147483648),Some(4166),Some(1073743906),Some(2147483647),None]),
        (&[0x4b,0x9a,0xdf,0x1c,0xf4,0xeb,0xdf,0xc3,0xc7,0xcf,0xd1,0xe9,0xf5,0xfb,0x1e,0xdf,0xbf,0xf0,0x3e,0x3f,0xcf,0xfa,0x0f,0xaf,0xfb,0xff,0xc0,0x7e,0x7f,0xf7,0xff,0xc0,0x00,0x00,0x00,0x3f,0x9f,0xff,0xff,0xdc,0xff,0x7f,0xff,0xff,0x72],360,&[Some(1),Some(2),Some(3),Some(4),Some(5),Some(6),Some(7),Some(8),Some(9),Some(10),Some(12),Some(13),Some(14),Some(16),Some(17),Some(18),Some(20),Some(21),Some(24),Some(28),Some(29),Some(36),Some(44),Some(45),Some(60),Some(76),Some(77),Some(108),Some(140),Some(141),Some(1073741894),Some(2147483647)]),
        (&[0x59,0xbc,0xeb,0xbf,0x1e,0x7d,0x7b,0x7b,0xfc,0x3e,0x3f,0x3f,0xa3,0xeb,0xfb,0xff,0x07,0xe7,0xfd,0xff,0xc0,0xfe,0x7f,0xf7,0xff,0xc0,0x00,0x00,0x00,0x3f,0xcf,0xff,0xff,0xf6,0xff,0xdf,0xff,0xff,0xed,0x80],314,&[Some(1),Some(2),Some(3),Some(4),Some(5),Some(6),Some(7),Some(8),Some(9),Some(10),Some(11),Some(12),Some(13),Some(14),Some(16),Some(17),Some(20),Some(24),Some(25),Some(32),Some(40),Some(41),Some(56),Some(72),Some(73),Some(1073741860),Some(2147483647)]),
        (&[0x4c,0xe6,0xb7,0x45,0x77,0xf4,0xeb,0xd9,0xdb,0xdf,0xe0,0xf1,0xf9,0xfd,0x0f,0x5f,0xdf,0xf8,0x1f,0x3f,0xef,0xfe,0x03,0xf3,0xff,0xbf,0xfe,0x00,0x00,0x00,0x01,0xfc,0xff,0xff,0xfe,0xe7,0xfb,0xff,0xff,0xfb,0x90],325,&[Some(1),Some(2),Some(3),Some(4),Some(5),Some(6),Some(7),Some(10),Some(14),Some(15),Some(16),Some(17),Some(18),Some(20),Some(21),Some(24),Some(28),Some(29),Some(36),Some(44),Some(45),Some(60),Some(76),Some(77),Some(108),Some(140),Some(141),Some(1073741894),Some(2147483647)]),
        (&[0x95,0xb8],13,&[Some(-2),Some(-1),Some(0),Some(1),Some(2)]),
        (&[0xf8,0x1f,0x1f,0xe7,0xf8,0x78,0xf9,0xf1,0xce,0x45,0xde,0xbb,0xf4,0xf5,0xf7,0xfa,0x1f,0x5f,0xef,0xfe,0x00,0x00,0x00,0x00,0xfc,0x7f,0xff,0xff,0xe7,0xf9,0xff,0xff,0xff,0x9f,0xf8,0x00,0x00,0x00,0x07,0xf3,0xff,0xff,0xff,0x3f,0xef,0xff,0xff,0xfc,0xc0],387,&[Some(-24),Some(-17),Some(-9),Some(-8),Some(-7),Some(-5),Some(-4),Some(-3),Some(-2),Some(-1),Some(0),Some(1),Some(2),Some(3),Some(4),Some(5),Some(6),Some(8),Some(9),Some(16),Some(24),Some(-25),Some(-1073741836),Some(-2147483648),Some(25),Some(1073741836),Some(2147483647)]),
    ];
    #[test]
    fn standard_all_published_prefixes(){
        let mut count=0;
        for number in 1..=15u8{
            let t=standard::table(number).unwrap();
            let p=PREFIXES[usize::from(number-1)];
            assert_eq!(t.codes().len(),p.len());
            for (actual,expected) in t.codes().iter().zip(p){
                let expected=if expected.is_empty(){None}else{
                    count+=1;
                    Some(Code{bits:u32::from_str_radix(expected,2).unwrap(),len:expected.len() as u8})
                };
                assert_eq!(*actual,expected,"table {}",number);
            }
        }
        assert_eq!(count,184);
    }
    fn decode_fixture(number:u8){
        let (bytes,len,values)=FIXTURES[usize::from(number-1)];
        let t=standard::table(number).unwrap();
        let mut reader=BitReader::new(bytes,len).unwrap();
        for expected in values{
            let v=expected.map(Integer::Value).unwrap_or(Integer::Oob);
            assert_eq!(t.decode(&mut reader).unwrap(),v,"table {}",number);
        }
        assert_eq!(reader.remaining(),0);
        // Entire final symbol truncated by one bit must fail, even for OOB.
        let mut reader=BitReader::new(bytes,len-1).unwrap();
        for _ in 1..values.len(){t.decode(&mut reader).unwrap();}
        assert!(t.decode(&mut reader).is_err());
    }
    #[test]fn standard_vectors_a_e(){for n in 1..=5{decode_fixture(n);}}
    #[test]fn standard_vectors_f_j(){for n in 6..=10{decode_fixture(n);}}
    #[test]fn standard_vectors_k_o(){for n in 11..=15{decode_fixture(n);}}
    #[test]
    fn standard_oob_and_disabled_rows(){
        let mut disabled=0;
        for n in 1..=15{
            let rows=standard::rows(n).unwrap();
            let count=rows.iter().filter(|r|r.range==Range::Oob).count();
            assert_eq!(count,usize::from(matches!(n,2|3|8|9|10)));
            disabled+=rows.iter().filter(|r|r.prefix_len==0).count();
        }
        assert_eq!(disabled,8);
        for n in [0,16,255]{assert!(standard::table(n).is_err());}
    }
    #[test]
    fn standard_extension_outside_i32_rejected(){
        for n in 1..=15u8{
            let t=standard::table(n).unwrap();
            for (row,code) in standard::rows(n).unwrap().iter().zip(t.codes()){
                if !matches!(row.range,Range::Lower{..}|Range::Upper{..}){continue;}
                let Some(code)=code else{continue;};
                // A full 32-bit all-one offset is outside signed i32
                // for every enabled standard extension.
                let width=usize::from(code.len)+32;
                let word=((u64::from(code.bits)<<32)|0xffff_ffff)<<(64-width);
                let bytes=word.to_be_bytes();
                assert!(t.decode(&mut BitReader::new(&bytes,width).unwrap()).is_err());
            }
        }
    }
    #[test]
    fn standard_annex_h_first_dictionary_fields(){
        // T.88 H.1 first dictionary: width 5, OOB, bitmap size 8.
        let b=standard::table(2).unwrap();
        let a=standard::table(1).unwrap();
        // 1110010 111111 01000
        let mut r=BitReader::new(&[229,250,0],18).unwrap();
        assert_eq!(b.decode(&mut r).unwrap(),Integer::Value(5));
        assert_eq!(b.decode(&mut r).unwrap(),Integer::Oob);
        assert_eq!(a.decode(&mut r).unwrap(),Integer::Value(8));
        assert_eq!(r.remaining(),0);
    }
}
