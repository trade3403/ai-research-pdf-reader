//! T.88 7.4.2 symbol dictionary data header and ordered dependency plan.
//! Includes bounded integer state and export runs; no bitmap reconstruction or retained context reuse yet.
use crate::{Error,Result,MAX_BYTES};
use super::{Segment,SegmentLocation,SegmentSource,PdfInspection};
use super::generic::{Offset,Template,Neighborhood};
use super::huffman::{self,Table,Range as ValueRange};
use std::ops::Range;

pub const MAX_SYMBOLS:u32=1_048_576;
const MAX_REFS:usize=4096;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Selection{Standard(u8),Custom}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Header{
    pub huffman:bool,
    pub refine_aggregate:bool,
    pub context_used:bool,
    pub context_retained:bool,
    pub template:Template,
    pub refinement_template:u8,
    pub adaptive:Vec<Offset>,
    pub refinement_adaptive:Vec<Offset>,
    /// DH, DW, BMSIZE, AGGINST in normative order. None in arithmetic mode.
    pub selections:Option<[Selection;4]>,
    pub exported:u32,
    pub new_symbols:u32,
    pub data_offset:usize,
}

pub fn parse(data:&[u8])->Result<(Header,&[u8])>{
    if data.len()>MAX_BYTES{return Err(Error::Limit("JBIG2 dictionary bytes"));}
    let f=data.get(..2).ok_or(Error::Invalid("JBIG2 dictionary flags truncated"))?;
    let f=u16::from_be_bytes([f[0],f[1]]);
    let huffman=f&1!=0;
    let refine_aggregate=f&2!=0;
    let context_used=f&0x100!=0;
    let context_retained=f&0x200!=0;
    let template=Template::from_code(((f>>10)&3)as u8)?;
    let refinement_template=((f>>12)&1)as u8;
    if f&0xe000!=0 || (!huffman&&f&0xfc!=0)
        || ((!huffman||!refine_aggregate)&&f&0x80!=0)
        || (huffman&&!refine_aggregate&&f&0x300!=0)
        || (huffman&&template!=Template::T0)
        || (!refine_aggregate&&refinement_template!=0){
        return Err(Error::Invalid("JBIG2 dictionary flag combination"));
    }
    let select=|v:u16,base:u8|->Result<Selection>{
        match v{0=>Ok(Selection::Standard(base)),1=>Ok(Selection::Standard(base+1)),
            3=>Ok(Selection::Custom),_=>Err(Error::Invalid("JBIG2 dictionary table selector"))}
    };
    let selections=if huffman{Some([
        select((f>>2)&3,4)?,select((f>>4)&3,2)?,
        if f&0x40!=0{Selection::Custom}else{Selection::Standard(1)},
        if f&0x80!=0{Selection::Custom}else{Selection::Standard(1)},
    ])}else{None};
    let mut pos=2usize;
    let mut read_offsets=|n:usize|->Result<Vec<Offset>>{
        let bytes=data.get(pos..pos+n*2).ok_or(Error::Invalid("JBIG2 dictionary AT truncated"))?;
        pos+=n*2;
        Ok(bytes.chunks_exact(2).map(|p|Offset{x:p[0] as i8,y:p[1] as i8}).collect())
    };
    let adaptive=if huffman{Vec::new()}else{read_offsets(template.adaptive_count())?};
    if !huffman{Neighborhood::new(template,&adaptive)?;}
    let refinement_adaptive=if refine_aggregate&&refinement_template==0{read_offsets(2)?}else{Vec::new()};
    // The first refinement AT samples the target; the second samples an
    // already decoded reference. Only the target must be causal.
    if refinement_adaptive.first().is_some_and(|p|p.y>0||p.y==0&&p.x>=0){
        return Err(Error::Invalid("JBIG2 dictionary noncausal refinement AT"));
    }
    let counts=data.get(pos..pos+8).ok_or(Error::Invalid("JBIG2 dictionary counts truncated"))?;
    let exported=u32::from_be_bytes([counts[0],counts[1],counts[2],counts[3]]);
    let new_symbols=u32::from_be_bytes([counts[4],counts[5],counts[6],counts[7]]);
    pos+=8;
    if exported>MAX_SYMBOLS||new_symbols>MAX_SYMBOLS{return Err(Error::Limit("JBIG2 dictionary symbol count"));}
    Ok((Header{huffman,refine_aggregate,context_used,context_retained,template,refinement_template,
        adaptive,refinement_adaptive,selections,exported,new_symbols,data_offset:pos},&data[pos..]))
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum TableSource{Standard(u8),Custom(u32)}
pub struct SelectedTable{pub source:TableSource,pub table:Table}
pub struct Plan{
    pub header:Header,
    pub imported:u32,
    /// Symbol dictionaries in referred-to order, with declared export counts.
    pub dictionaries:Vec<(u32,u32)>,
    pub tables:Option<[SelectedTable;4]>,
    /// Last referred dictionary if context reuse requested. No context copied.
    pub context_source:Option<u32>,
    pub encoded:Range<usize>,
}

/// Plan from an already parsed segment and resolved dependencies. The caller
/// supplies original payload slices in the exact order of segment.referred.
/// For dictionaries, counts are structural declarations, not decoded symbols.
pub fn plan(segment:&Segment,stream:&[u8],dependencies:&[(&Segment,&[u8])])->Result<Plan>{
    if segment.kind!=0{return Err(Error::Invalid("JBIG2 expected dictionary segment"));}
    if dependencies.len()!=segment.referred.len(){return Err(Error::Invalid("JBIG2 dictionary dependencies"));}
    if dependencies.len()>MAX_REFS{return Err(Error::Limit("JBIG2 dictionary references"));}
    let payload=stream.get(segment.payload.clone()).ok_or(Error::Invalid("JBIG2 dictionary payload span"))?;
    let (header,_)=parse(payload)?;
    let mut dictionaries=Vec::new();
    let mut custom=Vec::new();
    let mut imported=0u32;
    let mut last_header=None;
    for (&id,&(dep,data)) in segment.referred.iter().zip(dependencies){
        if dep.number!=id||id>=segment.number||dep.page!=0&&dep.page!=segment.page{
            return Err(Error::Invalid("JBIG2 dictionary dependency identity/page"));
        }
        if dep.payload.len()!=data.len(){return Err(Error::Invalid("JBIG2 dictionary dependency payload size"));}
        match dep.kind{
            0=>{
                let (d,_)=parse(data)?;
                imported=imported.checked_add(d.exported).ok_or(Error::Limit("JBIG2 dictionary imports"))?;
                if imported>MAX_SYMBOLS{return Err(Error::Limit("JBIG2 dictionary imports"));}
                dictionaries.push((id,d.exported));
                last_header=Some(d);
            },
            53=>{
                if custom.len()>=4{return Err(Error::Invalid("JBIG2 extra dictionary tables"));}
                custom.push((id,data));
            },
            _=>return Err(Error::Invalid("JBIG2 dictionary dependency kind")),
        }
    }
    let total=imported.checked_add(header.new_symbols).ok_or(Error::Limit("JBIG2 dictionary total symbols"))?;
    if total>MAX_SYMBOLS{return Err(Error::Limit("JBIG2 dictionary total symbols"));}
    if header.exported>total{return Err(Error::Invalid("JBIG2 dictionary exports exceed symbols"));}
    let context_source=if header.context_used{
        let d=last_header.ok_or(Error::Invalid("JBIG2 dictionary missing context source"))?;
        if !d.context_retained{return Err(Error::Invalid("JBIG2 dictionary context not retained"));}
        // Context shape/availability is checked when actual reuse is implemented.
        Some(dictionaries.last().unwrap().0)
    }else{None};
    let mut next=0usize;
    let tables=if let Some(selections)=header.selections{
        let mut load=|slot:usize|->Result<SelectedTable>{
            match selections[slot]{
                Selection::Standard(n)=>Ok(SelectedTable{source:TableSource::Standard(n),table:huffman::standard::table(n)?}),
                Selection::Custom=>{
                    let &(id,data)=custom.get(next).ok_or(Error::Invalid("JBIG2 missing dictionary custom table"))?;
                    next+=1;
                    let t=huffman::parse_custom(data)?;
                    let has_oob=t.rows().iter().any(|r|r.prefix_len!=0&&r.range==ValueRange::Oob);
                    if has_oob!=(slot==1){return Err(Error::Invalid("JBIG2 dictionary custom table OOB capability"));}
                    Ok(SelectedTable{source:TableSource::Custom(id),table:t.into_table()})
                }
            }
        };
        Some([load(0)?,load(1)?,load(2)?,load(3)?])
    }else{None};
    if next!=custom.len(){return Err(Error::Invalid("JBIG2 unused dictionary custom table"));}
    let start=segment.payload.start+header.data_offset;
    Ok(Plan{header,imported,dictionaries,tables,context_source,encoded:start..segment.payload.end})
}

/// Bridge from the existing PDF globals/page graph; never concatenate bytes.
/// The graph must correspond to the supplied original streams.
pub fn plan_pdf(inspected:&PdfInspection,globals:&[u8],page:&[u8],location:SegmentLocation)->Result<Plan>{
    let stream=|source|match source{SegmentSource::Globals=>globals,SegmentSource::Page=>page};
    let segment=inspected.segment(location).ok_or(Error::Invalid("JBIG2 dictionary location"))?;
    let node=inspected.graph.iter().find(|n|n.location==location&&n.number==segment.number)
        .ok_or(Error::Invalid("JBIG2 dictionary graph node"))?;
    if node.dependencies.len()>MAX_REFS{return Err(Error::Limit("JBIG2 dictionary references"));}
    let mut deps=Vec::with_capacity(node.dependencies.len());
    for &loc in &node.dependencies{
        let dep=inspected.segment(loc).ok_or(Error::Invalid("JBIG2 dictionary graph dependency"))?;
        let data=stream(loc.source).get(dep.payload.clone()).ok_or(Error::Invalid("JBIG2 dictionary dependency span"))?;
        deps.push((dep,data));
    }
    plan(segment,stream(location.source),&deps)
}

#[cfg(test)]
mod tests{
    use super::*;
    use super::super::{inspect,inspect_pdf_streams};
    // T.88 Annex H first (Huffman) and second-page (arithmetic) dictionary
    // data header fields, with original opaque entropy prefix.
    const HUFF:&[u8]=&[0,1,0,0,0,1,0,0,0,1,0xe9,0xcb,0xf4,0,0x26,0xaf,4,0xbf,0xf0,0x78,0x2f,0xe0,0,0x40];
    const ARITH:&[u8]=&[8,0,2,255,0,0,0,2,0,0,0,2,0x4f,0xe7,0x8c,0x20,0x0e,0x1d,0xc7,0xcf,1,0x11,0xc4,0xb2,0x6f,0xff,0xac];
    const A:&[u8]=&[0x42,0,0,0,0,0,1,1,0x10,0x49,0x23,0x81,0x80];
    // Two singleton normal rows disabled? One enabled singleton and OOB,
    // disabled lower/upper, all fields 1 bit: 10 00 1 -> 10001000.
    const OOB:&[u8]=&[1,0,0,0,0,0,0,0,1,0x88];
    fn dict(f:u16,at:&[u8],exported:u32,new:u32)->Vec<u8>{
        let mut d=f.to_be_bytes().to_vec();d.extend(at);d.extend(exported.to_be_bytes());d.extend(new.to_be_bytes());d
    }
    fn seg(id:u8,kind:u8,page:u8,refs:&[u8],data:&[u8])->Vec<u8>{
        let mut v=vec![0,0,0,id,kind,(refs.len() as u8)<<5];
        v.extend(refs);v.push(page);v.extend((data.len() as u32).to_be_bytes());v.extend(data);v
    }
    fn info()->Vec<u8>{
        let mut v=vec![];for n in [10u32,10,300,300]{v.extend(n.to_be_bytes());}v.extend([0,0,0]);v
    }
    #[test]
    fn dictionary_annex_h_headers(){
        let (a,rest)=parse(HUFF).unwrap();
        assert!(a.huffman);assert_eq!((a.exported,a.new_symbols,a.data_offset),(1,1,10));
        assert_eq!(rest,&HUFF[10..]);
        assert_eq!(a.selections,Some([Selection::Standard(4),Selection::Standard(2),Selection::Standard(1),Selection::Standard(1)]));
        let (a,rest)=parse(ARITH).unwrap();
        assert!(!a.huffman);assert_eq!(a.template,Template::T2);
        assert_eq!(a.adaptive,vec![Offset{x:2,y:-1}]);
        assert_eq!((a.exported,a.new_symbols,a.data_offset),(2,2,12));
        assert_eq!(rest,&ARITH[12..]);
    }
    #[test]
    fn dictionary_templates_and_refinement(){
        for t in 0..4u16{
            let count=if t==0{4}else{1};
            let at=[3,255,253,255,2,254,254,254];
            let d=dict(t<<10,&at[..2*count],1,1);
            let (a,rest)=parse(&d).unwrap();
            assert_eq!(a.adaptive.len(),count);assert!(rest.is_empty());
        }
        let d=dict(3,&[255,255,1,1],1,1);
        let (a,_)=parse(&d).unwrap();
        assert_eq!(a.refinement_adaptive,vec![Offset{x:-1,y:-1},Offset{x:1,y:1}]);
        assert!(a.adaptive.is_empty());
        let d=dict(0x1003,&[],1,1);
        assert!(parse(&d).unwrap().0.refinement_adaptive.is_empty());
        assert!(parse(&dict(3,&[0,0,0,0],0,0)).is_err());
        assert!(parse(&dict(0x800,&[0,0],0,0)).is_err());
    }
    #[test]
    fn dictionary_invalid_flags_counts_truncation(){
        for f in [0x2001,9,33,4,16,64,129,0x101,0x201,0x401,0x1001]{
            assert!(parse(&dict(f,&[3,255,253,255,2,254,254,254],0,0)).is_err(),"flags {f:x}");
        }
        for (d,n) in [(HUFF,10),(ARITH,12)]{
            for k in 0..n{assert!(parse(&d[..k]).is_err());}
        }
        assert!(parse(&dict(1,&[],MAX_SYMBOLS+1,0)).is_err());
        assert!(parse(&dict(1,&[],0,MAX_SYMBOLS+1)).is_err());
    }
    #[test]
    fn dictionary_standard_and_ordered_custom(){
        let mut g=seg(1,53,0,&[],A);g.extend(seg(2,0,0,&[],HUFF));g.extend(seg(3,53,0,&[],OOB));g.extend(seg(4,53,0,&[],A));g.extend(seg(5,53,0,&[],A));
        let mut p=seg(6,48,1,&[],&info());
        // Select all four custom tables interspersed with a dictionary.
        // Five references require extended count, two? one retention byte.
        let d=dict(0xff,&[255,255,1,1],2,1);
        let mut s=vec![0,0,0,7,0,0xe0,0,0,5,0,1,2,3,4,5,1];
        s.extend((d.len() as u32).to_be_bytes());s.extend(d);p.extend(s);
        let graph=inspect_pdf_streams(&g,&p).unwrap();
        let plan=plan_pdf(&graph,&g,&p,SegmentLocation{source:SegmentSource::Page,index:1}).unwrap();
        assert_eq!(plan.imported,1);assert_eq!(plan.dictionaries,vec![(2,1)]);
        assert_eq!(plan.tables.unwrap().map(|t|t.source),[TableSource::Custom(1),TableSource::Custom(3),TableSource::Custom(4),TableSource::Custom(5)]);
    }
    #[test]
    fn dictionary_bad_dependencies_and_oob(){
        let mut g=seg(1,53,0,&[],A);g.extend(seg(2,53,0,&[],OOB));
        let a=inspect(&g).unwrap();
        let refs=[(&a.segments[0],A)];
        let s=seg(3,0,0,&[1],&dict(0x31,&[],0,0));
        let i=inspect(&s).unwrap();assert!(plan(&i.segments[0],&s,&refs).is_err()); // DW lacks OOB
        let s=seg(3,0,0,&[2],&dict(0x0d,&[],0,0));
        let i=inspect(&s).unwrap();assert!(plan(&i.segments[0],&s,&[(&a.segments[1],OOB)]).is_err()); // DH has OOB
        let s=seg(3,0,0,&[1],&dict(1,&[],0,0));
        let i=inspect(&s).unwrap();assert!(plan(&i.segments[0],&s,&refs).is_err()); // unused
        let s=seg(3,0,0,&[],&dict(0x31,&[],0,0));
        let i=inspect(&s).unwrap();assert!(plan(&i.segments[0],&s,&[]).is_err()); // missing
        let s=seg(3,0,0,&[2],&dict(0x31,&[],0,0));
        let i=inspect(&s).unwrap();assert!(plan(&i.segments[0],&s,&refs).is_err()); // wrong id
    }
    #[test]
    fn dictionary_import_export_budget_and_context(){
        let d=dict(0xa00,&[2,255],MAX_SYMBOLS,MAX_SYMBOLS);
        let g=seg(1,0,0,&[],&d);let a=inspect(&g).unwrap();
        let s=seg(2,0,0,&[1],&dict(0x900,&[2,255],MAX_SYMBOLS,0));
        let i=inspect(&s).unwrap();let p=plan(&i.segments[0],&s,&[(&a.segments[0],&d)]).unwrap();
        assert_eq!(p.context_source,Some(1));assert_eq!(p.imported,MAX_SYMBOLS);
        let s=seg(2,0,0,&[1],&dict(0x900,&[2,255],0,1));
        let i=inspect(&s).unwrap();assert!(plan(&i.segments[0],&s,&[(&a.segments[0],&d)]).is_err());
        let s=seg(2,0,0,&[],&dict(1,&[],2,1));
        let i=inspect(&s).unwrap();assert!(plan(&i.segments[0],&s,&[]).is_err());
        let s=seg(2,0,0,&[],&dict(0x900,&[2,255],0,0));
        let i=inspect(&s).unwrap();assert!(plan(&i.segments[0],&s,&[]).is_err());
    }
    #[test]
    fn dictionary_annex_h_standard_plan(){
        let s=seg(1,0,0,&[],HUFF);let i=inspect(&s).unwrap();
        let p=plan(&i.segments[0],&s,&[]).unwrap();
        assert_eq!(&s[p.encoded],&HUFF[10..]);
        let t=p.tables.unwrap();
        let bytes=&HUFF[10..];
        let mut reader=crate::filters::ccitt::BitReader::new(bytes,25).unwrap();
        use super::super::integer::Integer;
        assert_eq!(t[0].table.decode(&mut reader).unwrap(),Integer::Value(8));
        assert_eq!(t[1].table.decode(&mut reader).unwrap(),Integer::Value(5));
        assert_eq!(t[1].table.decode(&mut reader).unwrap(),Integer::Oob);
        assert_eq!(t[2].table.decode(&mut reader).unwrap(),Integer::Value(8));
        assert_eq!(reader.remaining(),0);
    }
}


/// Incremental height-class state (T.88 6.5.5–6.5.7).
/// A caller must decode the bitmap for each returned symbol, or the collective
/// bitmap after OOB, before advancing the entropy stream. No pixel execution.
pub const MAX_CLASSES:u32=MAX_SYMBOLS;
pub const MAX_EXPORT_RUNS:u32=2*MAX_SYMBOLS+2;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct SymbolSize{pub index:u32,pub width:u32,pub height:u32,pub x:u32}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct HeightClass{pub first:u32,pub count:u32,pub height:u32,pub width:u32}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum WidthEvent{Symbol(SymbolSize),End(HeightClass)}
pub struct DimensionState{
    expected:u32,decoded:u32,classes:u32,height:u32,width:u32,
    total_width:u32,first:u32,active:bool,pixels:u64,work_limit:u64,
}
impl DimensionState{
    pub fn new(expected:u32,work_limit:u64)->Result<Self>{
        if expected>MAX_SYMBOLS||work_limit>super::bitmap::MAX_PIXELS{
            return Err(Error::Limit("JBIG2 dictionary dimensions budget"));
        }
        Ok(Self{expected,decoded:0,classes:0,height:0,width:0,total_width:0,
            first:0,active:false,pixels:0,work_limit})
    }
    pub fn decoded(&self)->u32{self.decoded}
    pub fn pixels(&self)->u64{self.pixels}
    pub fn complete(&self)->bool{self.decoded==self.expected&&!self.active}
    pub fn begin(&mut self,delta:super::integer::Integer)->Result<u32>{
        if self.active||self.complete(){return Err(Error::Invalid("JBIG2 dictionary height state"));}
        if self.classes>=MAX_CLASSES{return Err(Error::Limit("JBIG2 dictionary height classes"));}
        let h=dimension(self.height,delta)?;
        self.height=h;self.width=0;self.total_width=0;self.first=self.decoded;
        self.active=true;self.classes+=1;Ok(h)
    }
    pub fn width(&mut self,delta:super::integer::Integer)->Result<WidthEvent>{
        use super::integer::Integer;
        if !self.active{return Err(Error::Invalid("JBIG2 dictionary width state"));}
        if delta==Integer::Oob{
            self.active=false;
            return Ok(WidthEvent::End(HeightClass{first:self.first,count:self.decoded-self.first,
                height:self.height,width:self.total_width}));
        }
        if self.decoded>=self.expected{return Err(Error::Invalid("JBIG2 excess dictionary symbols"));}
        let width=dimension(self.width,delta)?;
        let total_width=self.total_width.checked_add(width).ok_or(Error::Limit("JBIG2 collective width"))?;
        if total_width>super::bitmap::MAX_SIDE{return Err(Error::Limit("JBIG2 collective width"));}
        let pixels=self.pixels.checked_add(u64::from(width)*u64::from(self.height))
            .ok_or(Error::Limit("JBIG2 dictionary pixels"))?;
        if pixels>self.work_limit{return Err(Error::Limit("JBIG2 dictionary pixels"));}
        let result=SymbolSize{index:self.decoded,width,height:self.height,x:self.total_width};
        self.width=width;self.total_width=total_width;self.pixels=pixels;self.decoded+=1;
        Ok(WidthEvent::Symbol(result))
    }
}
fn dimension(previous:u32,delta:super::integer::Integer)->Result<u32>{
    use super::integer::Integer;
    let Integer::Value(delta)=delta else{return Err(Error::Invalid("JBIG2 unexpected dimension OOB"));};
    let v=i64::from(previous).checked_add(delta).ok_or(Error::Limit("JBIG2 dictionary dimension"))?;
    // Zero-size symbols are outside the current positive-dimension bitmap core.
    if v<=0{return Err(Error::Invalid("JBIG2 nonpositive symbol dimension"));}
    if v>i64::from(super::bitmap::MAX_SIDE){return Err(Error::Limit("JBIG2 dictionary dimension"));}
    Ok(v as u32)
}

/// Imported indices precede newly decoded indices (T.88 6.5.10).
/// The alternating run decoder permits zero-length runs, but charges each
/// run against a caller budget. The returned indices are monotonically ordered.
pub fn export_indices(imported:u32,new:u32,expected:u32,run_limit:u32,
    mut next:impl FnMut()->Result<super::integer::Integer>)->Result<Vec<u32>>{
    use super::integer::Integer;
    let total=imported.checked_add(new).ok_or(Error::Limit("JBIG2 export symbols"))?;
    if total>MAX_SYMBOLS||run_limit>MAX_EXPORT_RUNS{return Err(Error::Limit("JBIG2 export budget"));}
    if expected>total{return Err(Error::Invalid("JBIG2 export count"));}
    let mut out=Vec::new();
    out.try_reserve_exact(expected as usize).map_err(|_|Error::Limit("JBIG2 export allocation"))?;
    let mut pos=0u32;let mut flag=false;let mut runs=0;
    while pos<total{
        if runs>=run_limit{return Err(Error::Limit("JBIG2 export runs"));}
        runs+=1;
        let Integer::Value(v)=next()? else{return Err(Error::Invalid("JBIG2 export OOB"));};
        let n=u32::try_from(v).map_err(|_|Error::Invalid("JBIG2 export run length"))?;
        let end=pos.checked_add(n).ok_or(Error::Limit("JBIG2 export run end"))?;
        if end>total{return Err(Error::Invalid("JBIG2 export run exceeds symbols"));}
        if flag{
            if out.len()+n as usize>expected as usize{return Err(Error::Invalid("JBIG2 too many exports"));}
            out.extend(pos..end);
        }
        pos=end;flag=!flag;
    }
    if out.len()!=expected as usize{return Err(Error::Invalid("JBIG2 export count mismatch"));}
    Ok(out)
}

pub fn huffman_exports(reader:&mut crate::filters::ccitt::BitReader<'_>,imported:u32,new:u32,
    expected:u32,run_limit:u32)->Result<Vec<u32>>{
    let table=huffman::standard::table(1)?;
    export_indices(imported,new,expected,run_limit,||table.decode(reader))
}

/// Independent persistent IADH, IADW and IAEX banks, sharing one arithmetic
/// stream. Callers interleave symbol pixel decisions on that same stream.
#[derive(Default)]
pub struct ArithmeticIntegers{
    dh:super::integer::IntegerDecoder,dw:super::integer::IntegerDecoder,
    ex:super::integer::IntegerDecoder,
}
impl ArithmeticIntegers{
    pub fn height(&mut self,decoder:&mut super::qm::Decoder<'_>)->Result<super::integer::Integer>{self.dh.decode(decoder)}
    pub fn width(&mut self,decoder:&mut super::qm::Decoder<'_>)->Result<super::integer::Integer>{self.dw.decode(decoder)}
    pub fn exports(&mut self,decoder:&mut super::qm::Decoder<'_>,imported:u32,new:u32,
        expected:u32,run_limit:u32)->Result<Vec<u32>>{
        export_indices(imported,new,expected,run_limit,||self.ex.decode(decoder))
    }
}

#[cfg(test)]
mod dimension_tests{
    use super::*;
    use super::super::integer::Integer::{self,Value,Oob};
    use crate::filters::ccitt::BitReader;
    #[test]
    fn dimensions_signed_classes_reference(){
        // Explicit T.88 B.5/B.3 prefixes packed offline, no pixel fields.
        let bytes=[225,195,127,127,125,251,251,127,127,253,251,254,124];
        let mut reader=BitReader::new(&bytes,103).unwrap();
        let dh=huffman::standard::table(5).unwrap();
        let dw=huffman::standard::table(3).unwrap();
        let mut state=DimensionState::new(7,1000).unwrap();
        let mut sizes=Vec::new();let mut classes=Vec::new();
        while !state.complete(){
            state.begin(dh.decode(&mut reader).unwrap()).unwrap();
            loop{match state.width(dw.decode(&mut reader).unwrap()).unwrap(){
                WidthEvent::Symbol(s)=>sizes.push((s.width,s.height,s.x)),
                WidthEvent::End(c)=>{classes.push(c);break;}
            }}
        }
        assert_eq!(sizes,vec![(3,4,0),(5,4,3),(3,4,8),(2,3,0),(1,3,2),(1,3,0),(1,3,1)]);
        assert_eq!(classes.iter().map(|c|(c.first,c.count,c.height,c.width)).collect::<Vec<_>>(),
            vec![(0,3,4,11),(3,2,3,3),(5,2,3,2)]);
        assert_eq!(state.pixels(),59);assert_eq!(reader.remaining(),0);
    }
    #[test]
    fn dimensions_order_and_bounds(){
        let mut s=DimensionState::new(1,12).unwrap();
        assert!(s.width(Value(1)).is_err());
        assert!(s.begin(Oob).is_err());assert!(s.begin(Value(0)).is_err());
        assert!(s.begin(Value(i64::MAX)).is_err());
        s.begin(Value(3)).unwrap();assert!(s.begin(Value(1)).is_err());
        assert!(s.width(Value(-1)).is_err());assert!(s.width(Value(5)).is_err());
        assert_eq!(s.decoded(),0);assert_eq!(s.pixels(),0);
        assert!(matches!(s.width(Value(4)).unwrap(),WidthEvent::Symbol(_)));
        assert!(!s.complete());assert!(s.width(Value(0)).is_err());
        assert!(matches!(s.width(Oob).unwrap(),WidthEvent::End(_)));
        assert!(s.complete());assert!(s.begin(Value(1)).is_err());
        assert!(DimensionState::new(MAX_SYMBOLS+1,0).is_err());
    }
    #[test]
    fn dimensions_empty_class_and_collective_limit(){
        let mut s=DimensionState::new(2,super::super::bitmap::MAX_PIXELS).unwrap();
        s.begin(Value(1)).unwrap();
        assert!(matches!(s.width(Oob).unwrap(),WidthEvent::End(HeightClass{count:0,..})));
        s.begin(Value(0)).unwrap();
        s.width(Value(i64::from(super::super::bitmap::MAX_SIDE))).unwrap();
        assert!(s.width(Value(0)).is_err());
        assert_eq!(s.decoded(),1);
        s.classes=MAX_CLASSES;
        s.width(Oob).unwrap();assert!(s.begin(Value(0)).is_err());
        assert!(DimensionState::new(0,0).unwrap().complete());
    }
    #[test]
    fn exports_reference_huffman_runs(){
        let bytes=[0,130,0,140,65];
        let mut r=BitReader::new(&bytes,40).unwrap();
        assert_eq!(huffman_exports(&mut r,3,7,6,8).unwrap(),vec![0,1,4,5,6,9]);
        assert_eq!(r.remaining(),0);
        let mut r=BitReader::new(&bytes,40-1).unwrap();
        assert!(huffman_exports(&mut r,3,7,6,8).is_err());
    }
    #[test]
    fn exports_annex_h_first_dictionary(){
        // Official first dictionary: two B.1 runs, 0 then 1.
        let mut r=BitReader::new(&[0,0x40],10).unwrap();
        assert_eq!(huffman_exports(&mut r,0,1,1,2).unwrap(),vec![0]);
        assert_eq!(r.remaining(),0);
    }
    #[test]
    fn exports_invalid_and_zero_runs(){
        fn scripted(total:u32,expected:u32,limit:u32,values:&[Integer])->Result<Vec<u32>>{
            let mut it=values.iter().copied();
            export_indices(0,total,expected,limit,||it.next().ok_or(Error::Invalid("test short")))
        }
        assert_eq!(scripted(3,0,1,&[Value(3)]).unwrap(),Vec::<u32>::new());
        assert_eq!(scripted(0,0,0,&[]).unwrap(),Vec::<u32>::new());
        assert!(scripted(1,1,1,&[Value(0)]).is_err());
        assert!(scripted(1,0,1,&[Oob]).is_err());
        assert!(scripted(1,0,1,&[Value(-1)]).is_err());
        assert!(scripted(1,0,1,&[Value(2)]).is_err());
        assert!(scripted(1,1,1,&[Value(1)]).is_err());
        assert!(scripted(1,0,2,&[Value(0),Value(1)]).is_err());
        assert!(scripted(1,2,0,&[]).is_err());
        assert!(scripted(1,0,MAX_EXPORT_RUNS+1,&[]).is_err());
    }
    #[test]
    fn dictionary_arithmetic_integer_budget(){
        // Real QM adapter, with zero available decisions, must fail closed.
        let mut banks=ArithmeticIntegers::default();
        let mut d=super::super::qm::Decoder::new(&[255,172],0).unwrap();
        assert!(banks.height(&mut d).is_err());
        assert!(banks.width(&mut d).is_err());
        assert!(banks.exports(&mut d,0,1,1,2).is_err());
        assert_eq!(banks.exports(&mut d,0,0,0,0).unwrap(),Vec::<u32>::new());
    }
}
