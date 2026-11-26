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
