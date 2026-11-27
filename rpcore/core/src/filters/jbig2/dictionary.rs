//! T.88 7.4.2 symbol dictionary data header and ordered dependency plan.
//! Direct and single-symbol refinement dictionaries with scoped bitmap context reuse.
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
        if d.huffman!=header.huffman||d.refine_aggregate!=header.refine_aggregate||d.template!=header.template||
            d.refinement_template!=header.refinement_template||d.adaptive!=header.adaptive||
            d.refinement_adaptive!=header.refinement_adaptive{
            return Err(Error::Invalid("JBIG2 dictionary context parameters mismatch"));
        }
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


/// Scoped direct Huffman dictionary decoding (T.88 6.5.5–6.5.10).
/// Refinement/aggregate and arithmetic pixel reconstruction are separate.
use super::bitmap::{self,Bitmap};
use super::integer::Integer;
use crate::filters::ccitt::{self,BitReader};

pub const MAX_DICTIONARY_WORK:u64=200_000_000;
#[derive(Debug,Clone,Copy)]
pub struct HuffmanLimits{
    pub pixels:u64,
    pub packed_bytes:usize,
    pub work:u64,
    pub export_runs:u32,
}
impl Default for HuffmanLimits{
    fn default()->Self{Self{pixels:bitmap::MAX_PIXELS,packed_bytes:bitmap::MAX_PACKED_BYTES,
        work:MAX_DICTIONARY_WORK,export_runs:MAX_EXPORT_RUNS}}
}
#[derive(Debug,PartialEq,Eq)]
pub struct HuffmanDictionary{
    /// Only newly decoded bitmaps, in coding order.
    pub new_symbols:Vec<Bitmap>,
    /// Indices into the input-then-new symbol namespace. Actual imported
    /// bitmap ownership/resolution is deferred to the dictionary executor.
    pub exported_indices:Vec<u32>,
    pub classes:u32,
    pub raw_classes:u32,
    pub mmr_classes:u32,
    pub packed_bytes:usize,
    pub work_used:u64,
}
fn charge_work(left:&mut u64,n:u64)->Result<()>{
    *left=left.checked_sub(n).ok_or(Error::Limit("JBIG2 dictionary work"))?;
    Ok(())
}
fn packed_len(width:u32,height:u32)->Result<usize>{
    if width==0||height==0{return Err(Error::Invalid("JBIG2 dictionary bitmap dimensions"));}
    if width>bitmap::MAX_SIDE||height>bitmap::MAX_SIDE||u64::from(width)*u64::from(height)>bitmap::MAX_PIXELS{
        return Err(Error::Limit("JBIG2 dictionary bitmap dimensions"));
    }
    let n=(width as usize).div_ceil(8).checked_mul(height as usize).ok_or(Error::Limit("JBIG2 dictionary packed length"))?;
    if n>bitmap::MAX_PACKED_BYTES{return Err(Error::Limit("JBIG2 dictionary packed length"));}
    Ok(n)
}
fn cut_collective(image:&Bitmap,sizes:&[SymbolSize],out:&mut Vec<Bitmap>)->Result<()>{
    let mut x=0u32;
    for s in sizes{
        if s.x!=x||s.height!=image.height()||s.index as usize!=out.len(){
            return Err(Error::Invalid("JBIG2 collective symbol ordering"));
        }
        x=x.checked_add(s.width).ok_or(Error::Limit("JBIG2 collective symbol width"))?;
        if x>image.width(){return Err(Error::Invalid("JBIG2 collective symbol span"));}
        let mut symbol=Bitmap::new(s.width,s.height,false)?;
        for y in 0..s.height{
            for dx in 0..s.width{
                if image.get(s.x+dx,y)==Some(true){symbol.set(dx,y,true)?;}
            }
        }
        out.push(symbol);
    }
    if x!=image.width(){return Err(Error::Invalid("JBIG2 collective symbol coverage"));}
    Ok(())
}

/// Decode the direct (non-refinement) Huffman payload described by a prior
/// plan. Class entropy fields and bitmap payloads alternate in one stream.
/// Each class is byte-aligned after BMSIZE. Optional MMR EOFB is handled by
/// the existing native CCITT core. All packed symbol bytes and transient
/// collective bytes are bounded before allocating; no third-party decoder.
pub fn decode_huffman_direct(plan:&Plan,stream:&[u8],limits:HuffmanLimits)->Result<HuffmanDictionary>{
    if !plan.header.huffman||plan.header.refine_aggregate||plan.header.context_used||plan.header.context_retained{
        return Err(Error::Unsupported("JBIG2 non-direct Huffman dictionary"));
    }
    if limits.pixels>bitmap::MAX_PIXELS||limits.packed_bytes>bitmap::MAX_PACKED_BYTES||
        limits.work>MAX_DICTIONARY_WORK||limits.export_runs>MAX_EXPORT_RUNS{
        return Err(Error::Limit("JBIG2 dictionary decode limits"));
    }
    let data=stream.get(plan.encoded.clone()).ok_or(Error::Invalid("JBIG2 dictionary encoded span"))?;
    if data.len()>MAX_BYTES||data.len()>ccitt::MAX_BITS/8{return Err(Error::Limit("JBIG2 dictionary entropy bytes"));}
    let tables=plan.tables.as_ref().ok_or(Error::Invalid("JBIG2 dictionary tables missing"))?;
    let total=plan.imported.checked_add(plan.header.new_symbols).ok_or(Error::Limit("JBIG2 dictionary total"))?;
    if total>MAX_SYMBOLS{return Err(Error::Limit("JBIG2 dictionary total"));}
    if plan.header.exported>total{return Err(Error::Invalid("JBIG2 dictionary export count"));}
    let mut state=DimensionState::new(plan.header.new_symbols,limits.pixels)?;
    let mut out=Vec::new();
    out.try_reserve_exact(plan.header.new_symbols as usize).map_err(|_|Error::Limit("JBIG2 dictionary symbols allocation"))?;
    let mut pos=0usize;let mut left=limits.work;
    let mut packed=0usize;let mut raw_classes=0u32;let mut mmr_classes=0u32;
    let mut sizes=Vec::new();
    while !state.complete(){
        charge_work(&mut left,1)?;
        let rest=&data[pos..];
        let mut reader=BitReader::new(rest,rest.len()*8)?;
        state.begin(tables[0].table.decode(&mut reader)?)?;
        sizes.clear();
        let mut class_bytes=0usize;
        let class=loop{
            charge_work(&mut left,1)?;
            match state.width(tables[1].table.decode(&mut reader)?)?{
                WidthEvent::Symbol(s)=>{
                    class_bytes=class_bytes.checked_add(packed_len(s.width,s.height)?)
                        .ok_or(Error::Limit("JBIG2 dictionary symbol bytes"))?;
                    if packed.checked_add(class_bytes).ok_or(Error::Limit("JBIG2 dictionary symbol bytes"))?>limits.packed_bytes{
                        return Err(Error::Limit("JBIG2 dictionary packed budget"));
                    }
                    sizes.push(s);
                }
                WidthEvent::End(c)=>break c,
            }
        };
        let Integer::Value(n)=tables[2].table.decode(&mut reader)? else{return Err(Error::Invalid("JBIG2 bitmap size OOB"));};
        let n=usize::try_from(n).map_err(|_|Error::Invalid("JBIG2 negative bitmap size"))?;
        let header_bytes=reader.position().div_ceil(8);
        charge_work(&mut left,(header_bytes as u64)*8)?;
        pos=pos.checked_add(header_bytes).ok_or(Error::Limit("JBIG2 dictionary offset"))?;
        if class.count==0{
            if n!=0{return Err(Error::Invalid("JBIG2 empty class bitmap size"));}
            raw_classes+=1;
            continue;
        }
        let collective_bytes=packed_len(class.width,class.height)?;
        // MMR temporarily holds its own packed output during conversion.
        let transient=collective_bytes.checked_mul(if n==0{1}else{2}).ok_or(Error::Limit("JBIG2 dictionary transient bytes"))?;
        let persistent=packed.checked_add(class_bytes).ok_or(Error::Limit("JBIG2 dictionary packed bytes"))?;
        if persistent.checked_add(transient).ok_or(Error::Limit("JBIG2 dictionary packed bytes"))?>limits.packed_bytes{
            return Err(Error::Limit("JBIG2 dictionary packed budget"));
        }
        let size=if n==0{collective_bytes}else{n};
        let end=pos.checked_add(size).ok_or(Error::Limit("JBIG2 dictionary bitmap span"))?;
        let encoded=data.get(pos..end).ok_or(Error::Invalid("JBIG2 dictionary bitmap truncated"))?;
        let area=u64::from(class.width)*u64::from(class.height);
        // Charge copying/splitting separately from the native MMR work.
        charge_work(&mut left,area+collective_bytes as u64)?;
        let image=if n==0{
            charge_work(&mut left,(size as u64)*8)?;
            if class.width%8!=0{
                let mask=(1u8<<(8-class.width%8))-1;
                let stride=(class.width as usize).div_ceil(8);
                if encoded.chunks_exact(stride).any(|r|r[stride-1]&mask!=0){
                    return Err(Error::Invalid("JBIG2 raw collective row padding"));
                }
            }
            raw_classes+=1;
            Bitmap::from_packed(class.width,class.height,encoded)?
        }else{
            let mmr=ccitt::decode_mmr_known_length(encoded,class.width,class.height,left)?;
            // Exactly the work charged by the existing CCITT decoder.
            let work=area*2+(size as u64)*8+32*mmr.mode_counts.iter().sum::<u64>();
            charge_work(&mut left,work)?;
            mmr_classes+=1;
            Bitmap::from_packed(class.width,class.height,&mmr.packed)?
        };
        cut_collective(&image,&sizes,&mut out)?;
        packed=persistent;
        pos=end;
    }
    let rest=&data[pos..];
    let mut reader=BitReader::new(rest,rest.len()*8)?;
    let export_table=huffman::standard::table(1)?;
    let indices=export_indices(plan.imported,plan.header.new_symbols,plan.header.exported,limits.export_runs,||{
        charge_work(&mut left,1)?;
        let start=reader.position();
        let v=export_table.decode(&mut reader)?;
        charge_work(&mut left,(reader.position()-start)as u64)?;
        Ok(v)
    })?;
    if reader.remaining()>7{return Err(Error::Invalid("JBIG2 dictionary trailing bytes"));}
    charge_work(&mut left,reader.remaining() as u64)?;
    Ok(HuffmanDictionary{new_symbols:out,exported_indices:indices,classes:state.classes,
        raw_classes,mmr_classes,packed_bytes:packed,work_used:limits.work-left})
}

#[cfg(test)]
mod collective_tests{
    use super::*;
    // Official T.88 Annex H.1 first and third segments, data parts.
    // Expected pixels: independent libtiff decoding (first), published raw
    // rows and independent Python bit slicing (second).
    const FIRST:&[u8]=&[0,1,0,0,0,1,0,0,0,1,0xe9,0xcb,0xf4,0,0x26,0xaf,4,0xbf,0xf0,0x78,0x2f,0xe0,0,0x40];
    const SECOND:&[u8]=&[0,1,0,0,0,2,0,0,0,2,0xe5,0xcd,0xf8,0,0x79,0xe0,0x84,0x10,0x81,0xf0,0x82,0x10,0x86,0x10,0x79,0xf0,0,0x80];
    fn make_plan(data:&[u8],imported:u32)->Plan{
        let (header,_)=parse(data).unwrap();
        let tables=Some([4,2,1,1].map(|n|SelectedTable{source:TableSource::Standard(n),table:huffman::standard::table(n).unwrap()}));
        let start=header.data_offset;
        Plan{header,imported,dictionaries:vec![],tables,context_source:None,encoded:start..data.len()}
    }
    fn decode(data:&[u8])->Result<HuffmanDictionary>{
        decode_huffman_direct(&make_plan(data,0),data,HuffmanLimits::default())
    }
    #[test]
    fn collective_annex_h_mmr(){
        let d=decode(FIRST).unwrap();
        assert_eq!((d.classes,d.raw_classes,d.mmr_classes),(1,0,1));
        assert_eq!(d.exported_indices,[0]);
        let s=&d.new_symbols[0];assert_eq!((s.width(),s.height()),(5,8));
        assert_eq!(s.packed(),[240,136,136,136,240,128,128,128]);
    }
    #[test]
    fn collective_annex_h_raw(){
        let d=decode(SECOND).unwrap();
        assert_eq!((d.classes,d.raw_classes,d.mmr_classes),(1,1,0));
        assert_eq!(d.exported_indices,[0,1]);
        for s in &d.new_symbols{assert_eq!((s.width(),s.height()),(6,6));}
        assert_eq!(d.new_symbols[0].packed(),[120,132,128,128,132,120]);
        assert_eq!(d.new_symbols[1].packed(),[120,4,124,132,132,124]);
    }
    #[test]
    fn collective_truncation_and_trailing_bytes(){
        for data in [FIRST,SECOND]{
            for n in 10..data.len(){assert!(decode(&data[..n]).is_err(),"truncated {n}");}
            let mut v=data.to_vec();v.push(0);assert!(decode(&v).is_err());
        }
        let mut bad=SECOND.to_vec();bad[15]|=1;
        assert!(decode(&bad).is_err());
    }
    #[test]
    fn collective_limits_and_invalid_mode(){
        let p=make_plan(FIRST,0);
        for limits in [
            HuffmanLimits{pixels:39,..HuffmanLimits::default()},
            HuffmanLimits{packed_bytes:23,..HuffmanLimits::default()},
            HuffmanLimits{work:1,..HuffmanLimits::default()},
            HuffmanLimits{export_runs:1,..HuffmanLimits::default()},
            HuffmanLimits{work:MAX_DICTIONARY_WORK+1,..HuffmanLimits::default()},
        ]{assert!(decode_huffman_direct(&p,FIRST,limits).is_err());}
        let mut p=make_plan(FIRST,0);p.header.refine_aggregate=true;
        assert!(decode_huffman_direct(&p,FIRST,HuffmanLimits::default()).is_err());
        p.header.refine_aggregate=false;p.header.huffman=false;
        assert!(decode_huffman_direct(&p,FIRST,HuffmanLimits::default()).is_err());
    }
    #[test]
    fn collective_exact_budget(){
        let d=decode(FIRST).unwrap();let p=make_plan(FIRST,0);
        let limits=HuffmanLimits{pixels:40,packed_bytes:24,work:d.work_used,export_runs:2};
        assert_eq!(decode_huffman_direct(&p,FIRST,limits).unwrap(),d);
        assert!(decode_huffman_direct(&p,FIRST,HuffmanLimits{work:d.work_used-1,..limits}).is_err());
    }
    #[test]
    fn collective_multi_class_import_namespace(){
        // Two height classes: DH=1/DW=3/OOB/BMSIZE=0, then
        // DH=1/DW=2/OOB/BMSIZE=0. Raw symbols 101, then 10/01.
        // Export runs 0,1,1,1 over one imported and two new symbols.
        let data=[0,1,0,0,0,2,0,0,0,2,112,252,0,160,111,192,128,64,0,66,16];
        let p=make_plan(&data,1);
        let d=decode_huffman_direct(&p,&data,HuffmanLimits::default()).unwrap();
        assert_eq!((d.classes,d.raw_classes),(2,2));
        assert_eq!(d.new_symbols[0].packed(),[0xa0]);
        assert_eq!(d.new_symbols[1].packed(),[0x80,0x40]);
        assert_eq!(d.exported_indices,[0,2]);
    }
    #[test]
    fn collective_empty_dictionary_imports_only(){
        // No new symbols; B.1 export runs 0,2.
        let data=[0,1,0,0,0,2,0,0,0,0,0,0x80];
        let p=make_plan(&data,2);
        let d=decode_huffman_direct(&p,&data,HuffmanLimits::default()).unwrap();
        assert!(d.new_symbols.is_empty());assert_eq!(d.exported_indices,[0,1]);
        assert_eq!(d.classes,0);
    }
}


/// Direct arithmetic dictionary: shared QM stream, separate IAx banks and
/// a persistent generic pixel bank across all new symbols (T.88 6.5.8.1).
/// Direct arithmetic GB context retention is scoped below; GR refinement pending.
pub const MAX_ARITHMETIC_DECISIONS:usize=16_777_216;
#[derive(Debug,Clone,Copy)]
pub struct ArithmeticLimits{
    pub pixels:u64,
    pub packed_bytes:usize,
    pub work:u64,
    pub export_runs:u32,
    pub decisions:usize,
}
impl Default for ArithmeticLimits{
    fn default()->Self{Self{pixels:bitmap::MAX_PIXELS,packed_bytes:bitmap::MAX_PACKED_BYTES,
        work:MAX_DICTIONARY_WORK,export_runs:MAX_EXPORT_RUNS,decisions:MAX_ARITHMETIC_DECISIONS}}
}
#[derive(Debug,PartialEq,Eq)]
pub struct ArithmeticDictionary{
    pub new_symbols:Vec<Bitmap>,
    pub exported_indices:Vec<u32>,
    pub classes:u32,
    pub pixel_decisions:u64,
    pub packed_bytes:usize,
    pub work_used:u64,
    pub marker_offset:usize,
}

/// Decode a positive-size direct symbol using the caller's existing QM
/// stream and generic context bank, without resetting either between symbols.
/// No TPGD or skip mask (Table 16). Errors can advance stream and contexts.
pub fn decode_direct_symbol(width:u32,height:u32,neighborhood:&Neighborhood,
    contexts:&mut [super::qm::Context],decoder:&mut super::qm::Decoder<'_>,
    work_limit:u64)->Result<Bitmap>{
    let template=neighborhood.template();
    if contexts.len()!=template.context_count(){return Err(Error::Invalid("JBIG2 symbol context bank"));}
    packed_len(width,height)?;
    let area=u64::from(width)*u64::from(height);
    let work=area.checked_mul(template.bits() as u64+1).ok_or(Error::Limit("JBIG2 symbol work"))?;
    if work>work_limit.min(MAX_DICTIONARY_WORK){return Err(Error::Limit("JBIG2 symbol work"));}
    let mut image=Bitmap::new(width,height,false)?;
    for y in 0..height{
        for x in 0..width{
            let cx=neighborhood.context(&image,x,y)?;
            let bit=decoder.decode(&mut contexts[cx])?!=0;
            image.set(x,y,bit)?;
        }
    }
    Ok(image)
}

/// Immutable snapshot of GB statistics only. A direct dictionary never
/// modifies GR statistics; refinement/aggregate modes remain unsupported.
#[derive(Debug,PartialEq,Eq)]
pub struct BitmapContextSnapshot{
    neighborhood:Neighborhood,
    contexts:Vec<super::qm::Context>,
}
impl BitmapContextSnapshot{
    pub fn template(&self)->Template{self.neighborhood.template()}
    pub fn entries(&self)->usize{self.contexts.len()}
    pub fn nonzero_entries(&self)->usize{
        self.contexts.iter().filter(|&&v|v!=super::qm::Context::default()).count()
    }
}
pub fn decode_arithmetic_direct(plan:&Plan,stream:&[u8],limits:ArithmeticLimits)->Result<ArithmeticDictionary>{
    if plan.header.context_used||plan.header.context_retained{
        return Err(Error::Unsupported("JBIG2 direct dictionary requires context-aware entry"));
    }
    Ok(decode_arithmetic_with_context(plan,stream,limits,None)?.0)
}
/// Caller supplies the last referred dictionary's snapshot if and only if
/// the context-used flag is set. Executor enforces provenance by segment ID.
/// A new QM stream and all IAx banks start fresh even when GB is inherited.
pub fn decode_arithmetic_with_context(plan:&Plan,stream:&[u8],limits:ArithmeticLimits,
    initial:Option<&BitmapContextSnapshot>)->Result<(ArithmeticDictionary,Option<BitmapContextSnapshot>)>{
    if plan.header.huffman||plan.header.refine_aggregate{
        return Err(Error::Unsupported("JBIG2 non-direct arithmetic dictionary"));
    }
    if plan.header.context_used!=initial.is_some(){
        return Err(Error::Invalid("JBIG2 dictionary context presence"));
    }
    if plan.tables.is_some()||plan.header.selections.is_some(){
        return Err(Error::Invalid("JBIG2 arithmetic dictionary Huffman tables"));
    }
    if limits.pixels>bitmap::MAX_PIXELS||limits.packed_bytes>bitmap::MAX_PACKED_BYTES||
        limits.work>MAX_DICTIONARY_WORK||limits.export_runs>MAX_EXPORT_RUNS||
        limits.decisions>MAX_ARITHMETIC_DECISIONS{
        return Err(Error::Limit("JBIG2 arithmetic dictionary limits"));
    }
    let data=stream.get(plan.encoded.clone()).ok_or(Error::Invalid("JBIG2 arithmetic dictionary span"))?;
    let total=plan.imported.checked_add(plan.header.new_symbols).ok_or(Error::Limit("JBIG2 dictionary total"))?;
    if total>MAX_SYMBOLS{return Err(Error::Limit("JBIG2 dictionary total"));}
    if plan.header.exported>total{return Err(Error::Invalid("JBIG2 dictionary export count"));}
    let neighborhood=Neighborhood::new(plan.header.template,&plan.header.adaptive)?;
    let mut state=DimensionState::new(plan.header.new_symbols,limits.pixels)?;
    let mut left=limits.work;
    // Charge full input scan and context initialization before decoder creation.
    charge_work(&mut left,data.len() as u64)?;
    let count=plan.header.template.context_count();
    if let Some(snapshot)=initial{
        if snapshot.neighborhood!=neighborhood||snapshot.contexts.len()!=count{
            return Err(Error::Invalid("JBIG2 bitmap snapshot parameters"));
        }
    }
    charge_work(&mut left,count as u64)?;
    let mut decoder=super::qm::Decoder::new(data,limits.decisions)?;
    let mut contexts=Vec::new();
    contexts.try_reserve_exact(count).map_err(|_|Error::Limit("JBIG2 symbol context allocation"))?;
    if let Some(snapshot)=initial{
        contexts.extend_from_slice(&snapshot.contexts);
    }else{
        contexts.resize(count,super::qm::Context::default());
    }
    let mut banks=ArithmeticIntegers::default();
    let mut out=Vec::new();
    out.try_reserve_exact(plan.header.new_symbols as usize).map_err(|_|Error::Limit("JBIG2 dictionary symbols allocation"))?;
    let mut packed=0usize;
    // Conservative upper bound on one Annex A integer (sign, class, 32
    // magnitude bits); actual binary decisions also have a separate QM cap.
    const INTEGER_WORK:u64=39;
    while !state.complete(){
        charge_work(&mut left,INTEGER_WORK)?;
        state.begin(banks.height(&mut decoder)?)?;
        loop{
            charge_work(&mut left,INTEGER_WORK)?;
            match state.width(banks.width(&mut decoder)?)?{
                WidthEvent::End(_)=>break,
                WidthEvent::Symbol(s)=>{
                    let bytes=packed_len(s.width,s.height)?;
                    let next=packed.checked_add(bytes).ok_or(Error::Limit("JBIG2 dictionary packed bytes"))?;
                    if next>limits.packed_bytes{return Err(Error::Limit("JBIG2 dictionary packed budget"));}
                    let work=u64::from(s.width)*u64::from(s.height)*(plan.header.template.bits()as u64+1);
                    charge_work(&mut left,work)?;
                    let image=decode_direct_symbol(s.width,s.height,&neighborhood,&mut contexts,&mut decoder,work)?;
                    out.push(image);
                    packed=next;
                }
            }
        }
    }
    let indices=export_indices(plan.imported,plan.header.new_symbols,plan.header.exported,limits.export_runs,||{
        charge_work(&mut left,INTEGER_WORK)?;
        banks.ex.decode(&mut decoder)
    })?;
    // Explicit FF AC was required on initialization; marker fill is not an
    // authentication mechanism or proof of all intended encoded decisions.
    let snapshot=if plan.header.context_retained{Some(BitmapContextSnapshot{neighborhood,contexts})}else{None};
    Ok((ArithmeticDictionary{new_symbols:out,exported_indices:indices,classes:state.classes,
        pixel_decisions:state.pixels(),packed_bytes:packed,work_used:limits.work-left,marker_offset:decoder.marker_offset()},snapshot))
}

#[cfg(test)]
mod arithmetic_direct_tests{
    use super::*;
    // Official T.88 (2000) H.1 segment 9 and segment 16. Figure H.4 and
    // Figure H.12(a) independently specify the expected six-by-six pixels.
    const TWO:&[u8]=&[8,0,2,255,0,0,0,2,0,0,0,2,0x4f,0xe7,0x8c,0x20,0x0e,0x1d,0xc7,0xcf,0x01,0x11,0xc4,0xb2,0x6f,0xff,0xac];
    const ONE:&[u8]=&[8,0,2,255,0,0,0,1,0,0,0,1,0x4f,0xe7,0x8d,0x68,0x1b,0x14,0x2f,0x3f,0xff,0xac];
    fn make_plan(data:&[u8])->Plan{
        let (header,_)=parse(data).unwrap();
        let start=header.data_offset;
        Plan{header,imported:0,dictionaries:vec![],tables:None,context_source:None,encoded:start..data.len()}
    }
    fn decode(data:&[u8])->Result<ArithmeticDictionary>{
        decode_arithmetic_direct(&make_plan(data),data,ArithmeticLimits::default())
    }
    #[test]
    fn arithmetic_annex_h_two_symbols(){
        let d=decode(TWO).unwrap();
        assert_eq!((d.classes,d.pixel_decisions,d.packed_bytes,d.marker_offset),(1,72,12,13));
        assert_eq!(d.exported_indices,[0,1]);
        for s in &d.new_symbols{assert_eq!((s.width(),s.height()),(6,6));}
        assert_eq!(d.new_symbols[0].packed(),[120,132,128,128,132,120]);
        assert_eq!(d.new_symbols[1].packed(),[120,4,124,132,132,124]);
    }
    #[test]
    fn arithmetic_annex_h_single_symbol(){
        let d=decode(ONE).unwrap();
        assert_eq!((d.classes,d.pixel_decisions,d.packed_bytes,d.marker_offset),(1,36,6,8));
        assert_eq!(d.exported_indices,[0]);
        assert_eq!(d.new_symbols[0].packed(),[120,4,124,132,132,124]);
    }
    #[test]
    fn arithmetic_persistent_bank_and_interleaving(){
        let n=Neighborhood::nominal(Template::T2);
        let mut c=vec![super::super::qm::Context::default();1024];
        let mut qm=super::super::qm::Decoder::new(&TWO[12..],1000).unwrap();
        let mut banks=ArithmeticIntegers::default();
        assert_eq!(banks.height(&mut qm).unwrap(),Integer::Value(6));
        assert_eq!(banks.width(&mut qm).unwrap(),Integer::Value(6));
        let a=decode_direct_symbol(6,6,&n,&mut c,&mut qm,396).unwrap();
        assert!(c.iter().any(|v|*v!=super::super::qm::Context::default()));
        assert_eq!(banks.width(&mut qm).unwrap(),Integer::Value(0));
        let b=decode_direct_symbol(6,6,&n,&mut c,&mut qm,396).unwrap();
        assert_eq!(banks.width(&mut qm).unwrap(),Integer::Oob);
        assert_eq!(banks.exports(&mut qm,0,2,2,2).unwrap(),[0,1]);
        assert_eq!(a.packed(),[120,132,128,128,132,120]);
        assert_eq!(b.packed(),[120,4,124,132,132,124]);
    }
    #[test]
    fn arithmetic_budget_exact_and_rejected(){
        let d=decode(TWO).unwrap();
        let p=make_plan(TWO);
        let l=ArithmeticLimits{pixels:72,packed_bytes:12,work:d.work_used,export_runs:2,decisions:1000};
        assert_eq!(decode_arithmetic_direct(&p,TWO,l).unwrap(),d);
        for l in [
            ArithmeticLimits{pixels:71,..l},ArithmeticLimits{packed_bytes:11,..l},
            ArithmeticLimits{work:d.work_used-1,..l},ArithmeticLimits{export_runs:1,..l},
            ArithmeticLimits{decisions:71,..l},ArithmeticLimits{decisions:MAX_ARITHMETIC_DECISIONS+1,..l},
        ]{assert!(decode_arithmetic_direct(&p,TWO,l).is_err());}
    }
    #[test]
    fn arithmetic_markers_and_invalid_modes(){
        for n in 12..TWO.len(){assert!(decode(&TWO[..n]).is_err());}
        let mut extra=TWO.to_vec();extra.push(0);assert!(decode(&extra).is_err());
        let mut bad=TWO.to_vec();*bad.last_mut().unwrap()=0xad;assert!(decode(&bad).is_err());
        let mut p=make_plan(TWO);p.header.context_used=true;
        assert!(decode_arithmetic_direct(&p,TWO,ArithmeticLimits::default()).is_err());
        p.header.context_used=false;p.header.context_retained=true;
        assert!(decode_arithmetic_direct(&p,TWO,ArithmeticLimits::default()).is_err());
        p.header.context_retained=false;p.header.refine_aggregate=true;
        assert!(decode_arithmetic_direct(&p,TWO,ArithmeticLimits::default()).is_err());
        p.header.refine_aggregate=false;p.header.huffman=true;
        assert!(decode_arithmetic_direct(&p,TWO,ArithmeticLimits::default()).is_err());
    }
    #[test]
    fn arithmetic_symbol_template_and_size_validation(){
        for t in [Template::T0,Template::T1,Template::T2,Template::T3]{
            let n=Neighborhood::nominal(t);
            let mut c=vec![super::super::qm::Context::default();t.context_count()];
            let mut d=super::super::qm::Decoder::new(&[255,172],0).unwrap();
            assert!(decode_direct_symbol(1,1,&n,&mut c[..1],&mut d,100).is_err());
            assert!(decode_direct_symbol(0,1,&n,&mut c,&mut d,100).is_err());
            assert!(decode_direct_symbol(1,1,&n,&mut c,&mut d,0).is_err());
            assert!(decode_direct_symbol(1,1,&n,&mut c,&mut d,100).is_err());
        }
    }
    #[test]
    fn arithmetic_namespace_and_span_bounds(){
        let mut p=make_plan(TWO);p.imported=MAX_SYMBOLS;
        assert!(decode_arithmetic_direct(&p,TWO,ArithmeticLimits::default()).is_err());
        p.imported=0;p.header.exported=3;
        assert!(decode_arithmetic_direct(&p,TWO,ArithmeticLimits::default()).is_err());
        p.header.exported=2;p.encoded.end=usize::MAX;
        assert!(decode_arithmetic_direct(&p,TWO,ArithmeticLimits::default()).is_err());
    }
}


/// Direct dictionary executor for one inspected PDF JBIG2 globals/page pair.
/// Retains each decoded bitmap once in an immutable arena. Export lists
/// contain identities, not bitmap copies; equal pixels from separate coding
/// operations are deliberately not coalesced. No page/region execution.
use std::collections::BTreeMap;

pub const MAX_EXEC_DICTIONARIES:u32=4096;
pub const MAX_EXEC_SLOTS:usize=2*MAX_SYMBOLS as usize;
pub const MAX_EXEC_CONTEXT_ENTRIES:usize=1_048_576;
#[derive(Debug,Clone,Copy)]
pub struct ExecutionLimits{
    pub dictionaries:u32,
    pub new_symbols:u32,
    /// Cumulative namespace plus export-list entries, including temporary
    /// entries, not just the number of unique bitmaps.
    pub symbol_slots:usize,
    pub pixels:u64,
    pub packed_bytes:usize,
    pub work:u64,
    pub export_runs_per_dictionary:u32,
    pub decisions_per_dictionary:usize,
    /// Retained GB/GR entries plus active GB for direct arithmetic, or
    /// active GR/IAID/IAx for arithmetic refinement, or active GR for
    /// Huffman refinement (not RSS).
    pub context_entries:usize,
}
impl Default for ExecutionLimits{
    fn default()->Self{Self{dictionaries:MAX_EXEC_DICTIONARIES,new_symbols:MAX_SYMBOLS,
        symbol_slots:MAX_EXEC_SLOTS,pixels:bitmap::MAX_PIXELS,packed_bytes:bitmap::MAX_PACKED_BYTES,
        work:MAX_DICTIONARY_WORK,export_runs_per_dictionary:MAX_EXPORT_RUNS,
        decisions_per_dictionary:MAX_ARITHMETIC_DECISIONS,context_entries:MAX_EXEC_CONTEXT_ENTRIES}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct SymbolId(usize);
impl SymbolId{
    /// Identity within the returned DictionarySet only.
    pub fn index(self)->usize{self.0}
}
#[derive(Debug,Clone,Copy,Default,PartialEq,Eq)]
pub struct ExecutionStats{
    pub dictionaries:u32,
    pub huffman:u32,
    pub arithmetic:u32,
    /// All executed single-refinement dictionaries, either coding mode.
    pub single_refinement:u32,
    pub huffman_single_refinement:u32,
    /// Arithmetic dictionaries dispatched in aggregation mode (not instance count).
    pub arithmetic_aggregation:u32,
    /// Huffman dictionaries that actually decoded at least one multi-instance symbol.
    pub huffman_aggregation:u32,
    pub new_symbols:u32,
    pub symbol_slots:usize,
    pub pixels:u64,
    pub packed_bytes:usize,
    pub work_used:u64,
    pub retained_contexts:u32,
    pub reused_contexts:u32,
    pub retained_gr_contexts:u32,
    pub reused_gr_contexts:u32,
    pub peak_context_entries:usize,
    /// Physical snapshot releases in the opt-in context reclaim path.
    pub released_contexts:u32,
    pub released_context_entries:usize,
}
#[derive(Debug,PartialEq,Eq)]
pub struct DictionarySet{
    arena:Vec<Option<Bitmap>>,
    exports:BTreeMap<u32,Vec<SymbolId>>,
    stats:ExecutionStats,
}
impl DictionarySet{
    pub fn stats(&self)->ExecutionStats{self.stats}
    pub fn dictionary(&self,segment_number:u32)->Option<&[SymbolId]>{
        self.exports.get(&segment_number).map(Vec::as_slice)
    }
    pub fn bitmap(&self,id:SymbolId)->Option<&Bitmap>{self.arena.get(id.0)?.as_ref()}
    pub fn symbol(&self,segment_number:u32,index:usize)->Option<&Bitmap>{
        self.bitmap(*self.dictionary(segment_number)?.get(index)?)
    }
}

/// Page-only reclamation after all symbol dictionaries have been decoded.
/// Stable SymbolIds are never reused. A bitmap remains alive until the
/// last export slot in every still-needed dictionary is removed.
/// Counters report actual dropped packed Vec payloads, not allocator RSS.
#[derive(Debug,Default,Clone,Copy,PartialEq,Eq)]
pub(super) struct Reclaimed{
    pub dictionaries:usize,pub bitmaps:usize,pub bytes:usize,
}
pub(super) struct PageReclaimer{
    references:Vec<usize>,
}
impl DictionarySet{
    /// Retain only exports required by subsequent text regions. All
    /// dictionary imports and context reuse have already completed.
    pub(super) fn prepare_page_reclaim(&mut self,needed:&BTreeMap<u32,usize>)
        ->Result<(PageReclaimer,Reclaimed)>{
        let mut references=Vec::new();
        references.try_reserve_exact(self.arena.len()).map_err(|_|Error::Limit("JBIG2 reclaim references"))?;
        references.resize(self.arena.len(),0usize);
        let before=self.exports.len();
        self.exports.retain(|id,_|needed.contains_key(id));
        let mut freed=Reclaimed{dictionaries:before-self.exports.len(),..Reclaimed::default()};
        for ids in self.exports.values(){
            for &id in ids{
                let count=references.get_mut(id.0).ok_or(Error::Invalid("JBIG2 reclaim identity"))?;
                *count=count.checked_add(1).ok_or(Error::Limit("JBIG2 reclaim references"))?;
            }
        }
        for (slot,&count) in self.arena.iter_mut().zip(&references){
            if count==0{
                if let Some(image)=slot.take(){freed.bitmaps+=1;freed.bytes+=image.packed().len();}
            }
        }
        Ok((PageReclaimer{references},freed))
    }
    pub(super) fn release_page_dictionary(&mut self,number:u32,state:&mut PageReclaimer)->Result<Reclaimed>{
        let ids=self.exports.remove(&number).ok_or(Error::Invalid("JBIG2 reclaim dictionary"))?;
        let mut freed=Reclaimed{dictionaries:1,..Reclaimed::default()};
        for id in ids{
            let count=state.references.get_mut(id.0).ok_or(Error::Invalid("JBIG2 reclaim identity"))?;
            *count=count.checked_sub(1).ok_or(Error::Invalid("JBIG2 reclaim underflow"))?;
            if *count==0{
                let image=self.arena.get_mut(id.0).and_then(Option::take)
                    .ok_or(Error::Invalid("JBIG2 reclaim bitmap"))?;
                freed.bitmaps+=1;freed.bytes+=image.packed().len();
            }
        }
        Ok(freed)
    }
}

/// Parse and resolve the original streams internally: callers cannot supply
/// a stale/forged dependency graph. Decode every Type 0 in graph order,
/// including unused dictionaries; other segment payloads remain opaque
/// except referenced Type 53 tables. Retention flags are not executed:
/// even unexported new symbols remain in the bounded arena for this call.
/// Any error discards the entire result. The work budget is a conservative
/// algorithmic accounting unit, not a memory/RSS or wall-clock guarantee.
pub fn execute_pdf_dictionaries(globals:&[u8],page:&[u8],limits:ExecutionLimits)->Result<DictionarySet>{
    execute_pdf_dictionaries_impl(globals,page,limits,true,true)
}
/// Explicit legacy single-reference execution. Use this entry for callers
/// that require the previous exact work, symbol-slot and context budgets.
/// No retry or implicit fallback is performed by either entry.
pub fn execute_pdf_dictionaries_single_reference(globals:&[u8],page:&[u8],limits:ExecutionLimits)->Result<DictionarySet>{
    execute_pdf_dictionaries_impl(globals,page,limits,false,false)
}
/// Explicit arithmetic-only aggregation execution (historical 9f3 default),
/// including retained/reused GR; rejects multi-instance Huffman dictionaries.
pub fn execute_pdf_dictionaries_with_aggregation(globals:&[u8],page:&[u8],limits:ExecutionLimits)->Result<DictionarySet>{
    execute_pdf_dictionaries_impl(globals,page,limits,true,false)
}
/// Explicit arithmetic and Huffman aggregation, equivalent to the default.
/// Huffman temporary namespace slots are charged only when actually used.
pub fn execute_pdf_dictionaries_with_huffman_aggregation(globals:&[u8],page:&[u8],limits:ExecutionLimits)->Result<DictionarySet>{
    execute_pdf_dictionaries_impl(globals,page,limits,true,true)
}
fn execute_pdf_dictionaries_impl(globals:&[u8],page:&[u8],limits:ExecutionLimits,enable_aggregation:bool,enable_huffman_aggregation:bool)->Result<DictionarySet>{
    let inspected=super::inspect_pdf_streams(globals,page)?;
    execute_pdf_dictionaries_inspected(globals,page,limits,enable_aggregation,enable_huffman_aggregation,&inspected)
}
/// Internal graph must have been parsed from these exact input streams.
/// Opt-in snapshot reclamation during the dictionary pass. Symbol identities
/// and cumulative bitmap budgets are unchanged; only actual dead probability
/// snapshots are dropped. Each call owns its own global/page state.
pub fn execute_pdf_dictionaries_reclaimed_contexts(globals:&[u8],page:&[u8],limits:ExecutionLimits)->Result<DictionarySet>{
    let inspected=super::inspect_pdf_streams(globals,page)?;
    execute_pdf_dictionaries_inspected_reclaimed_contexts(globals,page,limits,true,true,&inspected)
}
pub(super) fn execute_pdf_dictionaries_inspected(globals:&[u8],page:&[u8],limits:ExecutionLimits,
    enable_aggregation:bool,enable_huffman_aggregation:bool,inspected:&super::PdfInspection)->Result<DictionarySet>{
    execute_pdf_dictionaries_inspected_impl(globals,page,limits,enable_aggregation,enable_huffman_aggregation,inspected,false)
}
pub(super) fn execute_pdf_dictionaries_inspected_reclaimed_contexts(globals:&[u8],page:&[u8],limits:ExecutionLimits,
    enable_aggregation:bool,enable_huffman_aggregation:bool,inspected:&super::PdfInspection)->Result<DictionarySet>{
    execute_pdf_dictionaries_inspected_impl(globals,page,limits,enable_aggregation,enable_huffman_aggregation,inspected,true)
}
/// Compute the last actual context consumer, not the last bitmap import.
/// The normal plan still validates the full dependency and context parameters.
/// A header-only pass avoids decoding custom tables twice.
fn context_consumers(globals:&[u8],page:&[u8],inspected:&super::PdfInspection,left:&mut u64)->Result<BTreeMap<u32,usize>>{
    let stream=|source|match source{SegmentSource::Globals=>globals,SegmentSource::Page=>page};
    let mut consumers=BTreeMap::<u32,usize>::new();
    for node in &inspected.graph{
        charge_work(left,1)?;
        let s=inspected.segment(node.location).ok_or(Error::Invalid("JBIG2 context graph node"))?;
        if s.kind!=0{continue;}
        charge_work(left,32+node.dependencies.len() as u64)?;
        let data=stream(node.location.source).get(s.payload.clone()).ok_or(Error::Invalid("JBIG2 context payload"))?;
        let (h,_)=parse(data)?;
        if !h.context_used{continue;}
        let mut source=None;
        for &loc in &node.dependencies{
            let dep=inspected.segment(loc).ok_or(Error::Invalid("JBIG2 context dependency"))?;
            if dep.kind==0{source=Some(dep.number);}
        }
        let source=source.ok_or(Error::Invalid("JBIG2 missing context consumer source"))?;
        let count=consumers.entry(source).or_default();
        *count=count.checked_add(1).ok_or(Error::Limit("JBIG2 context consumers"))?;
    }
    Ok(consumers)
}
fn release_context(number:u32,snapshots:&mut BTreeMap<u32,BitmapContextSnapshot>,
    gr:&mut BTreeMap<u32,RefinementContextSnapshot>,huff:&mut BTreeMap<u32,HuffmanRefinementContextSnapshot>,
    retained:&mut usize,stats:&mut ExecutionStats)->Result<()>{
    let mut count=0u32;
    let mut entries=0usize;
    if let Some(s)=snapshots.remove(&number){count+=1;entries+=s.entries();}
    if let Some(s)=gr.remove(&number){count+=1;entries+=s.entries();}
    if let Some(s)=huff.remove(&number){count+=1;entries+=s.entries();}
    if count>1{return Err(Error::Invalid("JBIG2 duplicate context kind"));}
    *retained=retained.checked_sub(entries).ok_or(Error::Invalid("JBIG2 context release underflow"))?;
    stats.released_contexts+=count;
    stats.released_context_entries+=entries;
    Ok(())
}
fn execute_pdf_dictionaries_inspected_impl(globals:&[u8],page:&[u8],limits:ExecutionLimits,
    enable_aggregation:bool,enable_huffman_aggregation:bool,inspected:&super::PdfInspection,reclaim_contexts:bool)->Result<DictionarySet>{
    if limits.dictionaries>MAX_EXEC_DICTIONARIES||limits.new_symbols>MAX_SYMBOLS||
        limits.symbol_slots>MAX_EXEC_SLOTS||limits.pixels>bitmap::MAX_PIXELS||
        limits.packed_bytes>bitmap::MAX_PACKED_BYTES||limits.work>MAX_DICTIONARY_WORK||
        limits.export_runs_per_dictionary>MAX_EXPORT_RUNS||
        limits.decisions_per_dictionary>MAX_ARITHMETIC_DECISIONS||limits.context_entries>MAX_EXEC_CONTEXT_ENTRIES{
        return Err(Error::Limit("JBIG2 dictionary execution limits"));
    }
    let mut left=limits.work;
    let input=globals.len().checked_add(page.len()).ok_or(Error::Limit("JBIG2 dictionary input"))?;
    charge_work(&mut left,input as u64)?;
    let stream=|source|match source{SegmentSource::Globals=>globals,SegmentSource::Page=>page};
    let mut set=DictionarySet{arena:Vec::new(),exports:BTreeMap::new(),stats:ExecutionStats::default()};
    let mut snapshots:BTreeMap<u32,BitmapContextSnapshot>=BTreeMap::new();
    let mut gr_snapshots:BTreeMap<u32,RefinementContextSnapshot>=BTreeMap::new();
    let mut huff_gr_snapshots:BTreeMap<u32,HuffmanRefinementContextSnapshot>=BTreeMap::new();
    let mut retained_entries=0usize;
    let mut consumers=if reclaim_contexts{
        context_consumers(globals,page,inspected,&mut left)?
    }else{BTreeMap::new()};
    for node in &inspected.graph{
        charge_work(&mut left,1)?;
        let segment=inspected.segment(node.location).ok_or(Error::Invalid("JBIG2 dictionary graph node"))?;
        if segment.kind!=0{continue;}
        if set.stats.dictionaries>=limits.dictionaries{return Err(Error::Limit("JBIG2 executed dictionaries"));}
        // Use the graph node directly: do not re-scan the graph per dictionary.
        let mut deps=Vec::new();
        deps.try_reserve_exact(node.dependencies.len()).map_err(|_|Error::Limit("JBIG2 dictionary dependency allocation"))?;
        for &loc in &node.dependencies{
            charge_work(&mut left,1)?;
            let dep=inspected.segment(loc).ok_or(Error::Invalid("JBIG2 dictionary dependency"))?;
            let payload=stream(loc.source).get(dep.payload.clone()).ok_or(Error::Invalid("JBIG2 dictionary dependency span"))?;
            // Charge repeated custom table construction, even for reuses.
            if dep.kind==53{charge_work(&mut left,(payload.len() as u64).saturating_mul(64))?;}
            deps.push((dep,payload));
        }
        let original=stream(node.location.source);
        let plan=plan(segment,original,&deps)?;
        let new_count=set.stats.new_symbols.checked_add(plan.header.new_symbols)
            .ok_or(Error::Limit("JBIG2 executed symbols"))?;
        if new_count>limits.new_symbols{return Err(Error::Limit("JBIG2 executed symbols"));}
        let total=plan.imported.checked_add(plan.header.new_symbols).ok_or(Error::Limit("JBIG2 dictionary namespace"))? as usize;
        // Single refinement needs an additional temporary borrowed-bitmap
        // slice. Account for those pointer slots before allocating.
        let borrow_slots=if plan.header.refine_aggregate{plan.imported as usize}else{0};
        let aggregate=enable_aggregation&&plan.header.refine_aggregate&&!plan.header.huffman;
        // At most all imports and all but the last new symbol are
        // simultaneously borrowed by one embedded text region. Reserve
        // this extra array in the cumulative slot cap before decoding.
        let huff_aggregate=enable_huffman_aggregation&&plan.header.refine_aggregate&&plan.header.huffman;
        let aggregate_slots=if aggregate&&plan.header.new_symbols!=0{total-1}else{0};
        let mut slots=set.stats.symbol_slots.checked_add(total).and_then(|n|n.checked_add(plan.header.exported as usize))
            .and_then(|n|n.checked_add(borrow_slots)).and_then(|n|n.checked_add(aggregate_slots))
            .ok_or(Error::Limit("JBIG2 dictionary symbol slots"))?;
        if slots>limits.symbol_slots{return Err(Error::Limit("JBIG2 dictionary symbol slots"));}
        charge_work(&mut left,total as u64+u64::from(plan.header.exported)+borrow_slots as u64+aggregate_slots as u64)?;
        let mut namespace=Vec::new();
        namespace.try_reserve_exact(total).map_err(|_|Error::Limit("JBIG2 dictionary namespace allocation"))?;
        for &(id,declared) in &plan.dictionaries{
            let actual=set.dictionary(id).ok_or(Error::Invalid("JBIG2 dictionary missing decoded import"))?;
            if actual.len()!=declared as usize{return Err(Error::Invalid("JBIG2 dictionary import count mismatch"));}
            namespace.extend_from_slice(actual);
        }
        if namespace.len()!=plan.imported as usize{return Err(Error::Invalid("JBIG2 dictionary namespace count"));}
        let pixels=limits.pixels.checked_sub(set.stats.pixels).ok_or(Error::Limit("JBIG2 dictionary pixels"))?;
        let packed_bytes=limits.packed_bytes.checked_sub(set.stats.packed_bytes).ok_or(Error::Limit("JBIG2 dictionary packed bytes"))?;
        let (images,indices,bytes,work)=if plan.header.huffman&&plan.header.refine_aggregate{
            let initial=match plan.context_source{
                Some(id)=>Some(huff_gr_snapshots.get(&id).ok_or(Error::Invalid("JBIG2 missing retained Huffman GR context"))?),
                None=>None,
            };
            let template=super::refinement::Template::from_code(plan.header.refinement_template)?;
            let active=retained_entries.checked_add(template.context_count())
                .ok_or(Error::Limit("JBIG2 context entries"))?;
            if active>limits.context_entries{return Err(Error::Limit("JBIG2 context entries"));}
            let mut imports=Vec::new();
            imports.try_reserve_exact(borrow_slots).map_err(|_|Error::Limit("JBIG2 refinement import allocation"))?;
            for &id in &namespace{
                imports.push(set.bitmap(id).ok_or(Error::Invalid("JBIG2 refinement missing bitmap identity"))?);
            }
            let decode_limits=ArithmeticLimits{pixels,packed_bytes,work:left,export_runs:limits.export_runs_per_dictionary,
                decisions:limits.decisions_per_dictionary};
            let (d,snapshot)=if huff_aggregate{
                decode_huffman_refinement_impl(&plan,original,&imports,decode_limits,initial,true,limits.symbol_slots-slots)?
            }else{
                decode_huffman_single_refinement_with_context(&plan,original,&imports,decode_limits,initial)?
            };
            if initial.is_some(){
                set.stats.reused_contexts+=1;set.stats.reused_gr_contexts+=1;
            }
            if let Some(snapshot)=snapshot{
                retained_entries=retained_entries.checked_add(snapshot.entries())
                    .ok_or(Error::Limit("JBIG2 retained Huffman GR entries"))?;
                if huff_gr_snapshots.insert(segment.number,snapshot).is_some(){
                    return Err(Error::Invalid("JBIG2 duplicate Huffman GR snapshot"));
                }
                set.stats.retained_contexts+=1;set.stats.retained_gr_contexts+=1;
            }
            set.stats.peak_context_entries=set.stats.peak_context_entries.max(active);
            slots=slots.checked_add(d.aggregate_reference_slots).ok_or(Error::Limit("JBIG2 dictionary symbol slots"))?;
            if slots>limits.symbol_slots{return Err(Error::Limit("JBIG2 dictionary symbol slots"));}
            if d.aggregate_reference_slots!=0{set.stats.huffman_aggregation+=1;}else{
                set.stats.single_refinement+=1;set.stats.huffman_single_refinement+=1;
            }
            (d.new_symbols,d.exported_indices,d.packed_bytes,d.work_used)
        }else if plan.header.huffman{
            let d=decode_huffman_direct(&plan,original,HuffmanLimits{pixels,packed_bytes,work:left,
                export_runs:limits.export_runs_per_dictionary})?;
            (d.new_symbols,d.exported_indices,d.packed_bytes,d.work_used)
        }else if plan.header.refine_aggregate{
            let initial=match plan.context_source{
                Some(id)=>Some(gr_snapshots.get(&id).ok_or(Error::Invalid("JBIG2 missing retained refinement context"))?),
                None=>None,
            };
            let active=retained_entries.checked_add(refinement_context_entries(&plan.header,plan.imported)?)
                .and_then(|n|n.checked_add(if aggregate{9*512}else{0}))
                .ok_or(Error::Limit("JBIG2 context entries"))?;
            if active>limits.context_entries{return Err(Error::Limit("JBIG2 context entries"));}
            let mut imports=Vec::new();
            imports.try_reserve_exact(borrow_slots).map_err(|_|Error::Limit("JBIG2 refinement import allocation"))?;
            for &id in &namespace{
                imports.push(set.bitmap(id).ok_or(Error::Invalid("JBIG2 refinement missing bitmap identity"))?);
            }
            let decode_limits=ArithmeticLimits{pixels,packed_bytes,work:left,export_runs:limits.export_runs_per_dictionary,
                decisions:limits.decisions_per_dictionary};
            let (d,snapshot)=if aggregate{
                decode_arithmetic_aggregation_with_context(&plan,original,&imports,decode_limits,initial)?
            }else{
                decode_arithmetic_single_refinement_with_context(&plan,original,&imports,decode_limits,initial)?
            };
            if initial.is_some(){
                set.stats.reused_contexts+=1;set.stats.reused_gr_contexts+=1;
            }
            if let Some(snapshot)=snapshot{
                // Keep only the GR bank. IAID and integer contexts are
                // transient; GB is implicitly zero in this restricted mode.
                retained_entries=retained_entries.checked_add(snapshot.entries())
                    .ok_or(Error::Limit("JBIG2 retained GR entries"))?;
                if gr_snapshots.insert(segment.number,snapshot).is_some(){
                    return Err(Error::Invalid("JBIG2 duplicate GR snapshot"));
                }
                set.stats.retained_contexts+=1;set.stats.retained_gr_contexts+=1;
            }
            // Borrow ends before the arena grows. Only newly decoded
            // bitmaps are moved into it by the common commit path below.
            set.stats.peak_context_entries=set.stats.peak_context_entries.max(active);
            if aggregate{set.stats.arithmetic_aggregation+=1;}else{set.stats.single_refinement+=1;}
            (d.new_symbols,d.exported_indices,d.packed_bytes,d.work_used)
        }else{
            let active=retained_entries.checked_add(plan.header.template.context_count())
                .ok_or(Error::Limit("JBIG2 context entries"))?;
            if active>limits.context_entries{return Err(Error::Limit("JBIG2 context entries"));}
            let initial=match plan.context_source{
                Some(id)=>Some(snapshots.get(&id).ok_or(Error::Invalid("JBIG2 missing retained context"))?),
                None=>None,
            };
            let (d,snapshot)=decode_arithmetic_with_context(&plan,original,ArithmeticLimits{pixels,packed_bytes,work:left,
                export_runs:limits.export_runs_per_dictionary,decisions:limits.decisions_per_dictionary},initial)?;
            set.stats.peak_context_entries=set.stats.peak_context_entries.max(active);
            if initial.is_some(){set.stats.reused_contexts+=1;}
            if let Some(snapshot)=snapshot{
                retained_entries=active;
                if snapshots.insert(segment.number,snapshot).is_some(){
                    return Err(Error::Invalid("JBIG2 duplicate context snapshot"));
                }
                set.stats.retained_contexts+=1;
            }
            (d.new_symbols,d.exported_indices,d.packed_bytes,d.work_used)
        };
        charge_work(&mut left,work)?;
        if images.len()!=plan.header.new_symbols as usize||indices.len()!=plan.header.exported as usize{
            return Err(Error::Invalid("JBIG2 decoded dictionary count"));
        }
        let actual_pixels=images.iter().try_fold(0u64,|n,image|
            n.checked_add(u64::from(image.width())*u64::from(image.height()))
                .ok_or(Error::Limit("JBIG2 dictionary pixels")))?;
        let actual_bytes=images.iter().try_fold(0usize,|n,image|
            n.checked_add(image.packed().len()).ok_or(Error::Limit("JBIG2 dictionary packed bytes")))?;
        if actual_pixels>pixels||actual_bytes>packed_bytes||actual_bytes!=bytes{
            return Err(Error::Limit("JBIG2 dictionary decoded bitmap budget"));
        }
        let start=set.arena.len();
        set.arena.try_reserve_exact(images.len()).map_err(|_|Error::Limit("JBIG2 dictionary arena allocation"))?;
        namespace.extend((start..start+images.len()).map(SymbolId));
        let mut exported=Vec::new();
        exported.try_reserve_exact(indices.len()).map_err(|_|Error::Limit("JBIG2 dictionary export allocation"))?;
        for index in indices{
            exported.push(*namespace.get(index as usize).ok_or(Error::Invalid("JBIG2 dictionary export index"))?);
        }
        set.arena.extend(images.into_iter().map(Some));
        if set.exports.insert(segment.number,exported).is_some(){
            return Err(Error::Invalid("JBIG2 duplicate executed dictionary"));
        }
        set.stats.dictionaries+=1;
        if plan.header.huffman{set.stats.huffman+=1;}else{set.stats.arithmetic+=1;}
        set.stats.new_symbols=new_count;
        set.stats.symbol_slots=slots;
        set.stats.pixels+=actual_pixels;
        set.stats.packed_bytes+=actual_bytes;
        if reclaim_contexts{
            // Old and active banks coexist during decoding. Drop only
            // after successful completion, never mutate a shared source.
            if let Some(source)=plan.context_source{
                charge_work(&mut left,1)?;
                let count=consumers.get_mut(&source).ok_or(Error::Invalid("JBIG2 context missing consumer"))?;
                *count=count.checked_sub(1).ok_or(Error::Invalid("JBIG2 context consumer underflow"))?;
                if *count==0{
                    consumers.remove(&source);
                    release_context(source,&mut snapshots,&mut gr_snapshots,&mut huff_gr_snapshots,
                        &mut retained_entries,&mut set.stats)?;
                }
            }
            if plan.header.context_retained&&!consumers.contains_key(&segment.number){
                charge_work(&mut left,1)?;
                release_context(segment.number,&mut snapshots,&mut gr_snapshots,&mut huff_gr_snapshots,
                    &mut retained_entries,&mut set.stats)?;
            }
        }
    }
    if reclaim_contexts&&(!consumers.is_empty()||retained_entries!=0||!snapshots.is_empty()||
        !gr_snapshots.is_empty()||!huff_gr_snapshots.is_empty()||
        set.stats.released_contexts!=set.stats.retained_contexts){
        return Err(Error::Invalid("JBIG2 context reclamation incomplete"));
    }
    set.stats.work_used=limits.work-left;
    Ok(set)
}

#[cfg(test)]
mod executor_tests{
    use super::*;
    // Official Annex H dictionary payloads, independently verified in
    // collective_tests and arithmetic_direct_tests. Synthetic segment graph.
    const FIRST:&[u8]=&[0,1,0,0,0,1,0,0,0,1,0xe9,0xcb,0xf4,0,0x26,0xaf,4,0xbf,0xf0,0x78,0x2f,0xe0,0,0x40];
    const SECOND:&[u8]=&[0,1,0,0,0,2,0,0,0,2,0xe5,0xcd,0xf8,0,0x79,0xe0,0x84,0x10,0x81,0xf0,0x82,0x10,0x86,0x10,0x79,0xf0,0,0x80];
    const TWO:&[u8]=&[8,0,2,255,0,0,0,2,0,0,0,2,0x4f,0xe7,0x8c,0x20,0x0e,0x1d,0xc7,0xcf,1,0x11,0xc4,0xb2,0x6f,0xff,0xac];
    const MIXED:&[u8]=&[0,1,0,0,0,2,0,0,0,2,112,252,0,160,111,192,128,64,0,66,16];
    const A:&[u8]=&[0x42,0,0,0,0,0,1,1,0x10,0x49,0x23,0x81,0x80];
    fn seg(id:u8,kind:u8,page:u8,refs:&[u8],data:&[u8])->Vec<u8>{
        assert!(refs.len()<=4);
        let mut v=vec![0,0,0,id,kind,(refs.len()as u8)<<5];
        v.extend(refs);v.push(page);v.extend((data.len()as u32).to_be_bytes());v.extend(data);v
    }
    fn info(id:u8)->Vec<u8>{
        let mut data=Vec::new();
        for n in [20u32,20,300,300]{data.extend(n.to_be_bytes());}
        data.extend([0,0,0]);seg(id,48,1,&[],&data)
    }
    fn reexport(n:u8)->Vec<u8>{
        assert!(n<16);
        // No new symbols: B.1 run zero then n (each five bits).
        vec![0,1,0,0,0,n,0,0,0,0,n>>2,n<<6]
    }
    fn fixture()->(Vec<u8>,Vec<u8>){
        let mut g=seg(1,0,0,&[],FIRST);g.extend(seg(2,0,0,&[],SECOND));
        let mut p=info(3);p.extend(seg(4,0,1,&[],TWO));
        p.extend(seg(5,0,1,&[2,1,4],&reexport(5)));
        p.extend(seg(6,0,1,&[5,1],&reexport(6)));
        (g,p)
    }
    fn ids(set:&DictionarySet,id:u32)->Vec<usize>{
        set.dictionary(id).unwrap().iter().map(|s|s.index()).collect()
    }
    #[test]
    fn aggregation_executor_legacy_direct_parity(){
        let (g,p)=fixture();let l=ExecutionLimits::default();
        let legacy=execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap();
        assert_eq!(execute_pdf_dictionaries_with_aggregation(&g,&p,l).unwrap(),legacy);
        let tight=ExecutionLimits{work:legacy.stats().work_used,symbol_slots:legacy.stats().symbol_slots,..l};
        assert_eq!(execute_pdf_dictionaries_with_aggregation(&g,&p,tight).unwrap(),legacy);
    }
    #[test]
    fn default_executor_official_direct_exact_parity(){
        let (g,p)=fixture();
        let l=ExecutionLimits::default();
        let old=execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap();
        let exact=ExecutionLimits{work:old.stats().work_used,symbol_slots:old.stats().symbol_slots,
            context_entries:old.stats().peak_context_entries,..l};
        assert_eq!(execute_pdf_dictionaries(&g,&p,exact).unwrap(),old);
        assert_eq!(execute_pdf_dictionaries_with_aggregation(&g,&p,exact).unwrap(),old);
    }

    #[test]
    fn page_reclaimer_shared_identity_last_export_and_unused(){
        let (g,p)=fixture();
        let mut set=execute_pdf_dictionaries(&g,&p,ExecutionLimits::default()).unwrap();
        let id=set.dictionary(1).unwrap()[0];
        let shared=set.dictionary(2).unwrap()[0];
        let mut needed=BTreeMap::new();needed.insert(5,1);needed.insert(6,1);
        let (mut state,freed)=set.prepare_page_reclaim(&needed).unwrap();
        assert_eq!(freed,Reclaimed{dictionaries:3,bitmaps:0,bytes:0});
        assert!(set.dictionary(1).is_none());
        assert_eq!(set.bitmap(id).unwrap().packed(),[240,136,136,136,240,128,128,128]);
        assert!(std::ptr::eq(set.symbol(5,2).unwrap(),set.symbol(6,5).unwrap()));
        assert_eq!(set.release_page_dictionary(5,&mut state).unwrap(),Reclaimed{dictionaries:1,bitmaps:0,bytes:0});
        assert!(set.bitmap(shared).is_some());
        assert_eq!(set.release_page_dictionary(6,&mut state).unwrap(),Reclaimed{dictionaries:1,bitmaps:5,bytes:32});
        assert!(set.bitmap(id).is_none());assert!(set.bitmap(shared).is_none());
        assert!(set.release_page_dictionary(6,&mut state).is_err());
        assert_eq!(set.stats().packed_bytes,32); // cumulative accounting unchanged
    }
    #[test]
    fn executor_mixed_official_pixels(){
        let (g,p)=fixture();let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
        assert_eq!((d.stats().dictionaries,d.stats().huffman,d.stats().arithmetic),(5,4,1));
        assert_eq!((d.stats().new_symbols,d.stats().pixels,d.stats().packed_bytes),(5,184,32));
        assert_eq!(d.stats().symbol_slots,32);
        assert_eq!(d.symbol(1,0).unwrap().packed(),[240,136,136,136,240,128,128,128]);
        for id in [2,4]{
            assert_eq!(d.symbol(id,0).unwrap().packed(),[120,132,128,128,132,120]);
            assert_eq!(d.symbol(id,1).unwrap().packed(),[120,4,124,132,132,124]);
        }
    }
    #[test]
    fn executor_order_and_shared_identity(){
        let (g,p)=fixture();let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
        assert_eq!(ids(&d,1),[0]);assert_eq!(ids(&d,2),[1,2]);assert_eq!(ids(&d,4),[3,4]);
        assert_eq!(ids(&d,5),[1,2,0,3,4]);assert_eq!(ids(&d,6),[1,2,0,3,4,0]);
        assert!(std::ptr::eq(d.symbol(1,0).unwrap(),d.symbol(6,5).unwrap()));
        assert!(std::ptr::eq(d.symbol(2,0).unwrap(),d.symbol(6,0).unwrap()));
        // Equal pixels from different decodes do not imply identical IDs.
        assert_eq!(d.symbol(2,0),d.symbol(4,0));
        assert!(!std::ptr::eq(d.symbol(2,0).unwrap(),d.symbol(4,0).unwrap()));
        assert!(d.dictionary(3).is_none());assert!(d.symbol(6,6).is_none());
        assert!(d.bitmap(SymbolId(usize::MAX)).is_none());
    }
    #[test]
    fn executor_new_and_imported_namespace(){
        let g=seg(1,0,0,&[],FIRST);
        let mut p=info(2);p.extend(seg(3,0,1,&[1],MIXED));
        let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
        assert_eq!(ids(&d,3),[0,2]);
        assert_eq!(d.symbol(3,1).unwrap().packed(),[0x80,0x40]);
        assert_eq!(d.bitmap(SymbolId(1)).unwrap().packed(),[0xa0]);
        assert_eq!(d.stats().new_symbols,3);
    }
    #[test]
    fn executor_cumulative_limits(){
        let (g,p)=fixture();let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
        let l=ExecutionLimits{dictionaries:5,new_symbols:5,symbol_slots:32,pixels:184,
            packed_bytes:32,work:d.stats().work_used,..ExecutionLimits::default()};
        assert_eq!(execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap(),d);
        for l in [
            ExecutionLimits{dictionaries:4,..l},ExecutionLimits{new_symbols:4,..l},
            ExecutionLimits{symbol_slots:31,..l},ExecutionLimits{pixels:183,..l},
            ExecutionLimits{packed_bytes:31,..l},ExecutionLimits{work:l.work-1,..l},
            ExecutionLimits{export_runs_per_dictionary:1,..l},
            ExecutionLimits{decisions_per_dictionary:71,..l},
            ExecutionLimits{symbol_slots:MAX_EXEC_SLOTS+1,..l},
        ]{assert!(execute_pdf_dictionaries_single_reference(&g,&p,l).is_err());}
    }
    #[test]
    fn executor_custom_table_dispatch(){
        let g=seg(1,53,0,&[],A);
        let mut body=FIRST.to_vec();body[1]=0x41; // custom BMSIZE, other standard
        let mut p=info(2);p.extend(seg(3,0,1,&[1],&body));
        let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
        assert_eq!(d.symbol(3,0).unwrap().packed(),[240,136,136,136,240,128,128,128]);
        assert!(d.dictionary(1).is_none());
        let mut p=info(2);p.extend(seg(3,0,1,&[],&body));
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
    }
    #[test]
    fn executor_bad_graph_or_payload(){
        let g=seg(1,0,0,&[],FIRST);
        let mut p=info(2);p.extend(seg(3,0,1,&[0],&reexport(1)));
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
        let mut p=info(2);p.extend(seg(3,0,1,&[2],&reexport(1)));
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
        let mut p=info(2);p.extend(seg(3,0,1,&[1],&reexport(2)));
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
        let mut bad=TWO.to_vec();bad.pop();
        let mut p=info(2);p.extend(seg(3,0,1,&[],&bad));
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
        let mut bad=TWO.to_vec();bad[0]|=1; // reuse without a referred dictionary
        let mut p=info(2);p.extend(seg(3,0,1,&[],&bad));
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
    }
    #[test]
    fn executor_empty_and_non_dictionary_scope(){
        let p=info(1);
        let d=execute_pdf_dictionaries_single_reference(&[],&p,ExecutionLimits{dictionaries:0,new_symbols:0,
            symbol_slots:0,pixels:0,packed_bytes:0,..ExecutionLimits::default()}).unwrap();
        assert_eq!(d.stats().new_symbols,0);assert_eq!(d.stats().dictionaries,0);
        let mut p=info(1);p.extend(seg(2,0,1,&[],&[0,1,0,0,0,0,0,0,0,0]));
        p.extend(seg(3,38,1,&[],&[0xff])); // intentionally opaque region
        let d=execute_pdf_dictionaries_single_reference(&[],&p,ExecutionLimits::default()).unwrap();
        assert_eq!(d.dictionary(2),Some(&[][..]));assert!(d.dictionary(3).is_none());
        assert_eq!(d.stats().packed_bytes,0);
    }
}


#[cfg(test)]
fn assert_reclaimed_context_parity(old:&DictionarySet,new:&DictionarySet){
    assert_eq!(old.arena,new.arena);assert_eq!(old.exports,new.exports);
    let mut expected=old.stats();let actual=new.stats();
    assert!(actual.work_used>expected.work_used);
    assert!(actual.peak_context_entries<=expected.peak_context_entries);
    assert_eq!(actual.released_contexts,expected.retained_contexts);
    expected.work_used=actual.work_used;expected.peak_context_entries=actual.peak_context_entries;
    expected.released_contexts=actual.released_contexts;
    expected.released_context_entries=actual.released_context_entries;
    assert_eq!(expected,actual);
}
#[cfg(test)]
mod retained_context_tests{
    use super::*;
    // Official H.1 segment 9, with retained flag set. The entropy is
    // unchanged: retaining statistics cannot change decoding.
    const SOURCE:&[u8]=&[10,0,2,255,0,0,0,2,0,0,0,2,0x4f,0xe7,0x8c,0x20,0x0e,0x1d,0xc7,0xcf,1,0x11,0xc4,0xb2,0x6f,255,172];
    // Offline JS Annex E decoder/readback searched deterministic short
    // entropy vectors against source statistics; this is a synthetic
    // decode vector, not an independently encoded conformance file.
    // DH=1, DW=3, pixels 110, OOB, export runs 0,3.
    const CHILD:&[u8]=&[9,0,2,255,0,0,0,3,0,0,0,1,145,79,6,25,255,172];
    fn make_plan(data:&[u8],imported:u32)->Plan{
        let (header,_)=parse(data).unwrap();
        let start=header.data_offset;
        Plan{header,imported,dictionaries:vec![],tables:None,context_source:None,encoded:start..data.len()}
    }
    fn seg(id:u8,kind:u8,page:u8,refs:&[u8],data:&[u8])->Vec<u8>{
        let mut v=vec![0,0,0,id,kind,(refs.len()as u8)<<5];
        v.extend(refs);v.push(page);v.extend((data.len()as u32).to_be_bytes());v.extend(data);v
    }
    fn info(id:u8)->Vec<u8>{
        let mut data=Vec::new();for n in [20u32,20,300,300]{data.extend(n.to_be_bytes());}
        data.extend([0,0,0]);seg(id,48,1,&[],&data)
    }
    fn fixture()->(Vec<u8>,Vec<u8>){
        let g=seg(1,0,0,&[],SOURCE);
        let mut p=info(2);p.extend(seg(3,0,1,&[1],CHILD));(g,p)
    }
    #[test]
    fn default_executor_retained_gb_exact_parity(){
        let (g,p)=fixture();
        let l=ExecutionLimits::default();
        let old=execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap();
        let exact=ExecutionLimits{work:old.stats().work_used,context_entries:old.stats().peak_context_entries,..l};
        assert_eq!(execute_pdf_dictionaries(&g,&p,exact).unwrap(),old);
        assert!(execute_pdf_dictionaries(&g,&p,ExecutionLimits{context_entries:exact.context_entries-1,..exact}).is_err());
    }
    #[test]
    fn retained_official_source_snapshot(){
        let (d,s)=decode_arithmetic_with_context(&make_plan(SOURCE,0),SOURCE,ArithmeticLimits::default(),None).unwrap();
        let s=s.unwrap();assert_eq!(s.template(),Template::T2);
        assert_eq!((s.entries(),s.nonzero_entries()),(1024,38));
        assert_eq!(d.new_symbols[0].packed(),[120,132,128,128,132,120]);
        assert_eq!(d.new_symbols[1].packed(),[120,4,124,132,132,124]);
        let mut fresh=SOURCE.to_vec();fresh[0]&=!2;
        let (other,s)=decode_arithmetic_with_context(&make_plan(&fresh,0),&fresh,ArithmeticLimits::default(),None).unwrap();
        assert_eq!(other,d);assert!(s.is_none());
    }
    #[test]
    fn retained_nonzero_context_reference(){
        let (_,s)=decode_arithmetic_with_context(&make_plan(SOURCE,0),SOURCE,ArithmeticLimits::default(),None).unwrap();
        let s=s.unwrap();
        let p=make_plan(CHILD,2);
        let (d,none)=decode_arithmetic_with_context(&p,CHILD,ArithmeticLimits::default(),Some(&s)).unwrap();
        assert!(none.is_none());assert_eq!(d.exported_indices,[0,1,2]);
        let b=&d.new_symbols[0];assert_eq!((b.width(),b.height()),(3,1));
        assert_eq!(b.packed(),[192]);
        assert_eq!(s.nonzero_entries(),38);
        let mut fresh=CHILD.to_vec();fresh[0]&=!1;
        assert!(decode_arithmetic_direct(&make_plan(&fresh,2),&fresh,ArithmeticLimits::default()).is_err());
    }
    #[test]
    fn retained_reuse_and_retain_chain(){
        let (g,_)=fixture();
        let mut child=CHILD.to_vec();child[0]|=2;
        let mut p=info(2);p.extend(seg(3,0,1,&[1],&child));
        // Branch from the original source rather than mutating its snapshot.
        p.extend(seg(4,0,1,&[1],CHILD));
        let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
        assert_eq!(d.stats().retained_contexts,2);assert_eq!(d.stats().reused_contexts,2);
        assert_eq!(d.stats().peak_context_entries,3072);
        assert_eq!(d.symbol(3,2).unwrap().packed(),[192]);
        assert_eq!(d.symbol(4,2).unwrap().packed(),[192]);
        assert!(std::ptr::eq(d.symbol(3,0).unwrap(),d.symbol(4,0).unwrap()));
    }

    #[test]
    fn reclaim_gb_branch_and_unused_snapshot(){
        let (g,_)=fixture();let mut child=CHILD.to_vec();child[0]|=2;
        let mut p=info(2);p.extend(seg(3,0,1,&[1],&child));p.extend(seg(4,0,1,&[1],CHILD));
        let l=ExecutionLimits::default();let old=execute_pdf_dictionaries(&g,&p,l).unwrap();
        let d=execute_pdf_dictionaries_reclaimed_contexts(&g,&p,l).unwrap();
        assert_reclaimed_context_parity(&old,&d);
        assert_eq!((old.stats().peak_context_entries,d.stats().peak_context_entries),(3072,2048));
        assert_eq!((d.stats().released_contexts,d.stats().released_context_entries,d.stats().reused_contexts),(2,2048,2));
        let exact=ExecutionLimits{context_entries:2048,work:d.stats().work_used,..l};
        assert_eq!(execute_pdf_dictionaries_reclaimed_contexts(&g,&p,exact).unwrap(),d);
        assert!(execute_pdf_dictionaries(&g,&p,exact).is_err());
        assert!(execute_pdf_dictionaries_reclaimed_contexts(&g,&p,ExecutionLimits{context_entries:2047,..exact}).is_err());
        assert!(execute_pdf_dictionaries_reclaimed_contexts(&g,&p,ExecutionLimits{work:exact.work-1,..exact}).is_err());
        let mut p=info(2);p.extend(seg(3,0,1,&[],SOURCE)); // no context consumer
        let d=execute_pdf_dictionaries_reclaimed_contexts(&g,&p,ExecutionLimits{context_entries:1024,..l}).unwrap();
        assert_eq!((d.stats().released_contexts,d.stats().peak_context_entries),(2,1024));
        let mut source=SOURCE.to_vec();source[0]&=!2;
        let (_,p)=fixture();
        assert!(execute_pdf_dictionaries_reclaimed_contexts(&seg(1,0,0,&[],&source),&p,l).is_err());
    }
    #[test]
    fn retained_executor_budget(){
        let (g,p)=fixture();
        let l=ExecutionLimits{context_entries:2048,..ExecutionLimits::default()};
        let d=execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap();
        assert_eq!((d.stats().retained_contexts,d.stats().reused_contexts,d.stats().peak_context_entries),(1,1,2048));
        assert_eq!(d.stats().new_symbols,3);assert_eq!(d.stats().pixels,75);
        assert_eq!(d.symbol(3,2).unwrap().packed(),[192]);
        for n in [0,1023,2047,MAX_EXEC_CONTEXT_ENTRIES+1]{
            assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits{context_entries:n,..l}).is_err());
        }
    }
    #[test]
    fn retained_missing_and_mismatched_parameters(){
        let (g,p)=fixture();
        let mut source=SOURCE.to_vec();source[0]&=!2;
        assert!(execute_pdf_dictionaries_single_reference(&seg(1,0,0,&[],&source),&p,ExecutionLimits::default()).is_err());
        for (index,byte) in [(0,13),(2,1)]{
            let mut child=CHILD.to_vec();child[index]=byte;
            let mut p=info(2);p.extend(seg(3,0,1,&[1],&child));
            assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
        }
        let (_,s)=decode_arithmetic_with_context(&make_plan(SOURCE,0),SOURCE,ArithmeticLimits::default(),None).unwrap();
        let s=s.unwrap();
        let mut p=make_plan(CHILD,2);p.header.adaptive[0].x=1;
        assert!(decode_arithmetic_with_context(&p,CHILD,ArithmeticLimits::default(),Some(&s)).is_err());
    }
    #[test]
    fn retained_last_reference_not_any_reference(){
        let mut g=seg(1,0,0,&[],SOURCE);
        let mut no=SOURCE.to_vec();no[0]&=!2;
        g.extend(seg(2,0,0,&[],&no));
        let mut p=info(3);p.extend(seg(4,0,1,&[1,2],CHILD));
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
    }
    #[test]
    fn retained_context_flag_and_snapshot_presence(){
        let (_,s)=decode_arithmetic_with_context(&make_plan(SOURCE,0),SOURCE,ArithmeticLimits::default(),None).unwrap();
        let s=s.unwrap();
        assert!(decode_arithmetic_with_context(&make_plan(CHILD,2),CHILD,ArithmeticLimits::default(),None).is_err());
        assert!(decode_arithmetic_with_context(&make_plan(SOURCE,0),SOURCE,ArithmeticLimits::default(),Some(&s)).is_err());
        assert!(decode_arithmetic_direct(&make_plan(SOURCE,0),SOURCE,ArithmeticLimits::default()).is_err());
    }
}


/// T.88 6.5.8.2.3: fixed IAID width over the entire dictionary namespace.
pub fn refinement_id_bits(imported:u32,new:u32)->Result<u8>{
    let total=imported.checked_add(new).ok_or(Error::Limit("JBIG2 refinement namespace"))?;
    if total>MAX_SYMBOLS{return Err(Error::Limit("JBIG2 refinement namespace"));}
    Ok(if total<=1{0}else{(u32::BITS-(total-1).leading_zeros())as u8})
}
fn refinement_context_entries(header:&Header,imported:u32)->Result<usize>{
    let t=super::refinement::Template::from_code(header.refinement_template)?;
    let bits=refinement_id_bits(imported,header.new_symbols)?;
    // Count active GR, IAID and six independent 512-entry integer banks.
    Ok(t.context_count()+(1usize<<bits)+6*512)
}
/// Immutable bitmap probability state for the single-refinement subset.
/// GB is always zero here: neither this mode nor a compatible ancestor
/// executes a generic region. Retain its geometry, not a redundant bank.
/// GR is copied into a fresh active bank; no QM or IAx state is retained.
#[derive(Debug,PartialEq,Eq)]
pub struct RefinementContextSnapshot{
    generic:Neighborhood,
    neighborhood:super::refinement::Neighborhood,
    bank:super::refinement::ContextBank,
}
impl RefinementContextSnapshot{
    pub fn entries(&self)->usize{self.bank.entries().len()}
    pub fn nonzero_entries(&self)->usize{
        self.bank.entries().iter().filter(|&&v|v!=super::qm::Context::default()).count()
    }
    pub fn template(&self)->super::refinement::Template{self.bank.template()}
}
/// Compatibility wrapper for callers that do not manage snapshots.
pub fn decode_arithmetic_single_refinement(plan:&Plan,stream:&[u8],imports:&[&Bitmap],
    limits:ArithmeticLimits)->Result<ArithmeticDictionary>{
    if plan.header.context_used||plan.header.context_retained||plan.context_source.is_some(){
        return Err(Error::Unsupported("JBIG2 single refinement requires context-aware entry"));
    }
    Ok(decode_arithmetic_single_refinement_with_context(plan,stream,imports,limits,None)?.0)
}
/// Exactly one reference per new symbol. No Huffman refinement or
/// multi-instance aggregation. Executor verifies last-dictionary provenance;
/// direct callers must provide that dictionary's immutable snapshot.
pub fn decode_arithmetic_single_refinement_with_context(plan:&Plan,stream:&[u8],imports:&[&Bitmap],
    limits:ArithmeticLimits,initial:Option<&RefinementContextSnapshot>)->Result<(ArithmeticDictionary,Option<RefinementContextSnapshot>)>{
    decode_arithmetic_refinement_impl(plan,stream,imports,limits,initial,false)
}
/// Candidate aggregation entry: permits more than one instance per symbol.
pub fn decode_arithmetic_aggregation(plan:&Plan,stream:&[u8],imports:&[&Bitmap],
    limits:ArithmeticLimits)->Result<ArithmeticDictionary>{
    if plan.header.context_used||plan.header.context_retained||plan.context_source.is_some(){
        return Err(Error::Unsupported("JBIG2 aggregation retained contexts"));
    }
    Ok(decode_arithmetic_aggregation_with_context(plan,stream,imports,limits,None)?.0)
}
/// Opt-in aggregation with immutable GR probability snapshots. Each dictionary
/// starts a fresh QM stream, IAID and integer banks. Direct callers supply
/// the correct ancestor; executor provenance validation is a separate gate.
pub fn decode_arithmetic_aggregation_with_context(plan:&Plan,stream:&[u8],imports:&[&Bitmap],
    limits:ArithmeticLimits,initial:Option<&RefinementContextSnapshot>)
    ->Result<(ArithmeticDictionary,Option<RefinementContextSnapshot>)>{
    decode_arithmetic_refinement_impl(plan,stream,imports,limits,initial,true)
}
fn decode_arithmetic_refinement_impl(plan:&Plan,stream:&[u8],imports:&[&Bitmap],
    limits:ArithmeticLimits,initial:Option<&RefinementContextSnapshot>,aggregate:bool)
    ->Result<(ArithmeticDictionary,Option<RefinementContextSnapshot>)>{
    use super::{integer::{IntegerDecoder,IaidDecoder},refinement,text_region};
    let h=&plan.header;
    if h.huffman||!h.refine_aggregate{
        return Err(Error::Unsupported("JBIG2 single arithmetic refinement mode"));
    }
    if h.context_used!=initial.is_some()||(!h.context_used&&plan.context_source.is_some()){
        return Err(Error::Invalid("JBIG2 refinement context presence"));
    }
    if plan.tables.is_some()||h.selections.is_some()||imports.len()!=plan.imported as usize{
        return Err(Error::Invalid("JBIG2 refinement dictionary imports/tables"));
    }
    if limits.pixels>bitmap::MAX_PIXELS||limits.packed_bytes>bitmap::MAX_PACKED_BYTES||
        limits.work>MAX_DICTIONARY_WORK||limits.export_runs>MAX_EXPORT_RUNS||
        limits.decisions>MAX_ARITHMETIC_DECISIONS{return Err(Error::Limit("JBIG2 refinement dictionary limits"));}
    let bits=refinement_id_bits(plan.imported,h.new_symbols)?;
    let total=plan.imported+h.new_symbols;
    if h.exported>total{return Err(Error::Invalid("JBIG2 refinement export count"));}
    if h.new_symbols>0&&imports.is_empty(){return Err(Error::Invalid("JBIG2 refinement no initial reference"));}
    let template=refinement::Template::from_code(h.refinement_template)?;
    let neighborhood=refinement::Neighborhood::new(template,&h.refinement_adaptive)?;
    let generic=Neighborhood::new(h.template,&h.adaptive)?;
    if let Some(snapshot)=initial{
        if snapshot.generic!=generic||snapshot.neighborhood!=neighborhood||snapshot.template()!=template{
            return Err(Error::Invalid("JBIG2 refinement snapshot parameters"));
        }
    }
    let data=stream.get(plan.encoded.clone()).ok_or(Error::Invalid("JBIG2 refinement dictionary span"))?;
    let mut left=limits.work;
    charge_work(&mut left,data.len() as u64)?;
    charge_work(&mut left,refinement_context_entries(h,plan.imported)? as u64)?;
    if let Some(snapshot)=initial{charge_work(&mut left,snapshot.entries() as u64)?;}
    let mut decoder=super::qm::Decoder::new(data,limits.decisions)?;
    let mut id=IaidDecoder::new(bits)?;
    let mut gr=refinement::ContextBank::new(template)?;
    if let Some(snapshot)=initial{gr.restore_from(&snapshot.bank)?;}
    let mut banks=ArithmeticIntegers::default();
    let mut ai=IntegerDecoder::new();let mut dx_bank=IntegerDecoder::new();let mut dy_bank=IntegerDecoder::new();
    let mut text=if aggregate{
        charge_work(&mut left,9*512)?;
        Some(text_region::ArithmeticTextContexts::new())
    }else{None};
    let mut state=DimensionState::new(h.new_symbols,limits.pixels)?;
    let mut out:Vec<Bitmap>=Vec::new();
    out.try_reserve_exact(h.new_symbols as usize).map_err(|_|Error::Limit("JBIG2 refinement symbols allocation"))?;
    let mut packed=0usize;
    const INT_WORK:u64=39;
    while !state.complete(){
        charge_work(&mut left,INT_WORK)?;
        state.begin(banks.height(&mut decoder)?)?;
        loop{
            charge_work(&mut left,INT_WORK)?;
            match state.width(banks.width(&mut decoder)?)?{
                WidthEvent::End(_)=>break,
                WidthEvent::Symbol(s)=>{
                    let bytes=packed_len(s.width,s.height)?;
                    let next=packed.checked_add(bytes).ok_or(Error::Limit("JBIG2 refinement packed bytes"))?;
                    if next>limits.packed_bytes{return Err(Error::Limit("JBIG2 refinement packed budget"));}
                    charge_work(&mut left,INT_WORK)?;
                    match ai.decode(&mut decoder)?{
                        Integer::Value(1)=>{},
                        Integer::Value(n) if n>1=>{
                            let contexts=text.as_mut().ok_or(Error::Unsupported("JBIG2 multiple symbol aggregation"))?;
                            let instances=u32::try_from(n).map_err(|_|Error::Limit("JBIG2 aggregation instances"))?;
                            if instances>text_region::MAX_INSTANCES{return Err(Error::Limit("JBIG2 aggregation instances"));}
                            let available=plan.imported+s.index;
                            charge_work(&mut left,u64::from(available))?;
                            // Reserve worst-case arithmetic syntax work before decoding.
                            charge_work(&mut left,u64::from(instances)*(10*INT_WORK+u64::from(bits))+INT_WORK)?;
                            let mut refs=Vec::new();
                            refs.try_reserve_exact(available as usize).map_err(|_|Error::Limit("JBIG2 aggregation references"))?;
                            refs.extend(imports.iter().copied());
                            refs.extend(out.iter());
                            if refs.len()!=available as usize{return Err(Error::Invalid("JBIG2 aggregation namespace"));}
                            let options=text_region::Options{width:s.width,height:s.height,default_black:false,
                                op:bitmap::ComposeOp::Or,strips:1,corner:text_region::Corner::TopLeft,
                                transposed:false,ds_offset:0,instances};
                            let decoded=text_region::decode_arithmetic_shared_bounded(options,&refs,&mut decoder,&mut id,
                                contexts,Some(&neighborhood),Some(&mut gr),left.min(bitmap::MAX_COMPOSE_WORK),
                                limits.packed_bytes-next)?;
                            charge_work(&mut left,decoded.stats.work_used)?;
                            out.push(decoded.bitmap);packed=next;
                            continue;
                        },
                        _=>return Err(Error::Invalid("JBIG2 refinement instance count")),
                    }
                    charge_work(&mut left,u64::from(bits)+2*INT_WORK)?;
                    let available=plan.imported+s.index;
                    let index=id.decode_for_count(&mut decoder,available)? as usize;
                    let (dx,dy)=if let Some(contexts)=text.as_mut(){
                        contexts.decode_displacement(&mut decoder)?
                    }else{
                        let Integer::Value(dx)=dx_bank.decode(&mut decoder)? else{return Err(Error::Invalid("JBIG2 refinement X OOB"));};
                        let Integer::Value(dy)=dy_bank.decode(&mut decoder)? else{return Err(Error::Invalid("JBIG2 refinement Y OOB"));};
                        (dx,dy)
                    };
                    let reference=if index<imports.len(){imports[index]}else{
                        out.get(index-imports.len()).ok_or(Error::Invalid("JBIG2 refinement forward symbol"))?
                    };
                    // Table 18: known class dimensions, raw RDX/RDY,
                    // no extra RDW/RDH or half-size correction, TPGR off.
                    let opt=refinement::RefinementOptions{width:s.width,height:s.height,dx,dy,typical_prediction:false};
                    let work=refinement::work_bound(template,opt)?;
                    charge_work(&mut left,work)?;
                    let decoded=refinement::decode_bitmap(reference,&neighborhood,opt,&mut gr,&mut decoder,work)?;
                    out.push(decoded.bitmap);packed=next;
                }
            }
        }
    }
    let indices=export_indices(plan.imported,h.new_symbols,h.exported,limits.export_runs,||{
        charge_work(&mut left,INT_WORK)?;banks.ex.decode(&mut decoder)
    })?;
    let snapshot=if h.context_retained{Some(RefinementContextSnapshot{generic,neighborhood,bank:gr})}else{None};
    Ok((ArithmeticDictionary{new_symbols:out,exported_indices:indices,classes:state.classes,
        pixel_decisions:state.pixels(),packed_bytes:packed,work_used:limits.work-left,marker_offset:decoder.marker_offset()},snapshot))
}

#[cfg(test)]
mod single_refinement_tests{
    use super::*;
    // Independent JS Annex E encoder/reader, two new symbols (3x2,4x2),
    // first uses imported symbol with (+1,-1), second uses first new
    // symbol with (-1,+1). Fixed 2-bit IAID, shared GR/IAx banks.
    const T0:&[u8]=&[124,227,105,204,198,86,197,71,223,255,172];
    const T1:&[u8]=&[124,227,105,204,198,86,191,231,207,255,172];
    fn body(t:u8,entropy:&[u8])->Vec<u8>{
        let mut v=vec![8|(t<<4),2,2,255];
        if t==0{v.extend([255;4]);}
        v.extend([0,0,0,3,0,0,0,2]);v.extend(entropy);v
    }
    fn plan_for(data:&[u8])->Plan{
        let (header,_)=parse(data).unwrap();let start=header.data_offset;
        Plan{header,imported:1,dictionaries:vec![],tables:None,context_source:None,encoded:start..data.len()}
    }
    fn src()->Bitmap{Bitmap::from_packed(7,6,&[0,124,124,124,0,0]).unwrap()}
    fn check(d:&ArithmeticDictionary){
        assert_eq!((d.classes,d.pixel_decisions,d.packed_bytes),(1,14,4));
        assert_eq!(d.exported_indices,[0,1,2]);
        assert_eq!((d.new_symbols[0].width(),d.new_symbols[0].height()),(3,2));
        assert_eq!((d.new_symbols[1].width(),d.new_symbols[1].height()),(4,2));
        assert_eq!(d.new_symbols[0].packed(),[160,64]);
        assert_eq!(d.new_symbols[1].packed(),[192,48]);
    }
    #[test]
    fn single_refinement_t0_reference(){
        let b=body(0,T0);check(&decode_arithmetic_single_refinement(&plan_for(&b),&b,&[&src()],ArithmeticLimits::default()).unwrap());
    }
    #[test]
    fn single_refinement_t1_reference(){
        let b=body(1,T1);check(&decode_arithmetic_single_refinement(&plan_for(&b),&b,&[&src()],ArithmeticLimits::default()).unwrap());
    }
    #[test]
    fn single_refinement_fixed_iaid_width(){
        for (n,bits) in [(0,0),(1,0),(2,1),(3,2),(4,2),(5,3),(MAX_SYMBOLS,20)]{
            assert_eq!(refinement_id_bits(0,n).unwrap(),bits);
        }
        assert_eq!(refinement_id_bits(1,2).unwrap(),2);
        assert!(refinement_id_bits(MAX_SYMBOLS,1).is_err());
        assert!(refinement_id_bits(u32::MAX,1).is_err());
    }
    #[test]
    fn single_refinement_bad_instance_and_reference(){
        for (entropy,unsupported) in [(&[124,191,255,172][..],true),(&[124,239,255,172],false),
            (&[124,63,255,172],false),(&[124,219,255,172],false),(&[124,224,255,172],false)]{
            let b=body(1,entropy);let e=decode_arithmetic_single_refinement(&plan_for(&b),&b,&[&src()],ArithmeticLimits::default()).unwrap_err();
            assert_eq!(matches!(e,Error::Unsupported(_)),unsupported);
        }
    }
    #[test]
    fn single_refinement_budgets(){
        let b=body(1,T1);let p=plan_for(&b);let s=src();
        let d=decode_arithmetic_single_refinement(&p,&b,&[&s],ArithmeticLimits::default()).unwrap();
        let l=ArithmeticLimits{pixels:14,packed_bytes:4,work:d.work_used,decisions:66,export_runs:2};
        assert_eq!(decode_arithmetic_single_refinement(&p,&b,&[&s],l).unwrap(),d);
        for l in [ArithmeticLimits{pixels:13,..l},ArithmeticLimits{packed_bytes:3,..l},
            ArithmeticLimits{work:l.work-1,..l},ArithmeticLimits{decisions:65,..l},
            ArithmeticLimits{export_runs:1,..l}]{
            assert!(decode_arithmetic_single_refinement(&p,&b,&[&s],l).is_err());
        }
    }
    #[test]
    fn single_refinement_modes_and_spans(){
        let b=body(1,T1);let s=src();let l=ArithmeticLimits::default();
        for n in 12..b.len(){
            let p=plan_for(&b[..n]);assert!(decode_arithmetic_single_refinement(&p,&b[..n],&[&s],l).is_err());
        }
        let mut p=plan_for(&b);
        assert!(decode_arithmetic_single_refinement(&p,&b,&[],l).is_err());
        p.imported=0;assert!(decode_arithmetic_single_refinement(&p,&b,&[],l).is_err());
        p.imported=1;p.header.context_retained=true;
        assert!(decode_arithmetic_single_refinement(&p,&b,&[&s],l).is_err());
        p.header.context_retained=false;p.header.context_used=true;
        assert!(decode_arithmetic_single_refinement(&p,&b,&[&s],l).is_err());
        p.header.context_used=false;p.header.huffman=true;
        assert!(decode_arithmetic_single_refinement(&p,&b,&[&s],l).is_err());
        p.header.huffman=false;p.encoded.end=usize::MAX;
        assert!(decode_arithmetic_single_refinement(&p,&b,&[&s],l).is_err());
    }
    #[test]
    fn single_refinement_context_accounting(){
        let b=body(1,T1);let p=plan_for(&b);
        assert_eq!(refinement_context_entries(&p.header,1).unwrap(),1024+4+3072);
        let b=body(0,T0);let p=plan_for(&b);
        assert_eq!(refinement_context_entries(&p.header,1).unwrap(),8192+4+3072);
    }
}


#[cfg(test)]
mod refinement_executor_tests{
    use super::*;
    // T.88 B.4 DH=6, B.2 DW=7, OOB, B.1 BMSIZE=0, then raw
    // 7x6 rows and B.1 EX runs 0,1. Independently packed 25 header bits.
    const SOURCE:&[u8]=&[0,1,0,0,0,1,0,0,0,1,229,211,240,0,0,124,124,124,0,0,0,64];
    const T0:&[u8]=&[124,227,105,204,198,86,197,71,223,255,172];
    const T1:&[u8]=&[124,227,105,204,198,86,191,231,207,255,172];
    const RETAINED:&[u8]=&[10,0,2,255,0,0,0,2,0,0,0,2,0x4f,0xe7,0x8c,0x20,0x0e,0x1d,0xc7,0xcf,1,0x11,0xc4,0xb2,0x6f,255,172];
    fn body(t:u8,entropy:&[u8])->Vec<u8>{
        let mut v=vec![8|(t<<4),2,2,255];if t==0{v.extend([255;4]);}
        v.extend([0,0,0,3,0,0,0,2]);v.extend(entropy);v
    }
    fn seg(id:u8,kind:u8,page:u8,refs:&[u8],data:&[u8])->Vec<u8>{
        let mut v=vec![0,0,0,id,kind,(refs.len()as u8)<<5];v.extend(refs);
        v.push(page);v.extend((data.len()as u32).to_be_bytes());v.extend(data);v
    }
    fn info(id:u8)->Vec<u8>{
        let mut d=vec![];for n in [20u32,20,300,300]{d.extend(n.to_be_bytes());}
        d.extend([0,0,0]);seg(id,48,1,&[],&d)
    }
    fn reexport(n:u8)->Vec<u8>{vec![0,1,0,0,0,n,0,0,0,0,n>>2,n<<6]}
    fn fixture(t:u8)->(Vec<u8>,Vec<u8>){
        let g=seg(1,0,0,&[],SOURCE);let mut p=info(2);
        p.extend(seg(3,0,1,&[1],&body(t,if t==0{T0}else{T1})));
        p.extend(seg(4,0,1,&[3,1],&reexport(4)));(g,p)
    }
    fn ids(d:&DictionarySet,n:u32)->Vec<usize>{
        d.dictionary(n).unwrap().iter().map(|s|s.index()).collect()
    }
    #[test]
    fn default_executor_single_arithmetic_reexport_parity(){
        for t in [0,1]{
            let (g,p)=fixture(t);let l=ExecutionLimits::default();
            let old=execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap();
            let d=execute_pdf_dictionaries(&g,&p,l).unwrap();
            assert_eq!(d.arena,old.arena);assert_eq!(d.exports,old.exports);
            assert_eq!(d.stats().arithmetic_aggregation,1);
            assert_eq!(d.stats().work_used,old.stats().work_used+9*512+2);
            assert_eq!(d.stats().symbol_slots,old.stats().symbol_slots+2);
            assert_eq!(d.stats().peak_context_entries,old.stats().peak_context_entries+9*512);
            for i in 0..3{assert!(std::ptr::eq(d.symbol(3,i).unwrap(),d.symbol(4,i).unwrap()));}
            let exact=ExecutionLimits{work:d.stats().work_used,symbol_slots:d.stats().symbol_slots,
                context_entries:d.stats().peak_context_entries,..l};
            assert_eq!(execute_pdf_dictionaries(&g,&p,exact).unwrap(),d);
            assert!(execute_pdf_dictionaries(&g,&p,ExecutionLimits{work:exact.work-1,..exact}).is_err());
        }
    }
    #[test]
    fn refinement_executor_mixed_templates(){
        for t in [0,1]{
            let (g,p)=fixture(t);let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
            let s=d.stats();
            assert_eq!((s.dictionaries,s.huffman,s.arithmetic,s.single_refinement),(3,2,1,1));
            assert_eq!((s.new_symbols,s.pixels,s.packed_bytes,s.symbol_slots),(3,56,10,17));
            assert_eq!(s.peak_context_entries,if t==0{8192+4+3072}else{1024+4+3072});
            assert_eq!(d.symbol(1,0).unwrap().packed(),[0,124,124,124,0,0]);
            assert_eq!(d.symbol(3,1).unwrap().packed(),[160,64]);
            assert_eq!(d.symbol(3,2).unwrap().packed(),[192,48]);
        }
    }
    #[test]
    fn refinement_executor_reexport_identity(){
        let (g,p)=fixture(1);let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
        assert_eq!(ids(&d,1),[0]);assert_eq!(ids(&d,3),[0,1,2]);assert_eq!(ids(&d,4),[0,1,2,0]);
        for i in 0..3{assert!(std::ptr::eq(d.symbol(3,i).unwrap(),d.symbol(4,i).unwrap()));}
        assert!(std::ptr::eq(d.symbol(1,0).unwrap(),d.symbol(4,3).unwrap()));
    }
    #[test]
    fn refinement_executor_global_chain(){
        let mut g=seg(1,0,0,&[],SOURCE);g.extend(seg(2,0,0,&[1],&body(1,T1)));
        let mut p=info(3);p.extend(seg(4,0,1,&[2,1],&reexport(4)));
        let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
        assert_eq!(ids(&d,2),[0,1,2]);assert_eq!(ids(&d,4),[0,1,2,0]);
        assert_eq!(d.symbol(4,2).unwrap().packed(),[192,48]);
    }
    #[test]
    fn refinement_executor_exact_cumulative_limits(){
        let (g,p)=fixture(1);let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
        // 12-byte cap covers source raw collective plus split output peak.
        let l=ExecutionLimits{dictionaries:3,new_symbols:3,symbol_slots:17,pixels:56,
            packed_bytes:12,work:d.stats().work_used,context_entries:4100,
            export_runs_per_dictionary:2,decisions_per_dictionary:66};
        assert_eq!(execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap(),d);
        for l in [
            ExecutionLimits{dictionaries:2,..l},ExecutionLimits{new_symbols:2,..l},
            ExecutionLimits{symbol_slots:16,..l},ExecutionLimits{pixels:55,..l},
            ExecutionLimits{packed_bytes:11,..l},ExecutionLimits{work:l.work-1,..l},
            ExecutionLimits{context_entries:4099,..l},ExecutionLimits{decisions_per_dictionary:65,..l},
            ExecutionLimits{export_runs_per_dictionary:1,..l},
        ]{assert!(execute_pdf_dictionaries_single_reference(&g,&p,l).is_err());}
    }
    #[test]
    fn refinement_executor_retained_gb_coexistence(){
        let g=seg(1,0,0,&[],SOURCE);let mut p=info(2);
        p.extend(seg(3,0,1,&[],RETAINED));p.extend(seg(4,0,1,&[1],&body(1,T1)));
        let l=ExecutionLimits{context_entries:5124,..ExecutionLimits::default()};
        let d=execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap();
        assert_eq!((d.stats().retained_contexts,d.stats().reused_contexts,d.stats().peak_context_entries),(1,0,5124));
        assert_eq!(ids(&d,4),[0,3,4]);assert_eq!(d.symbol(4,2).unwrap().packed(),[192,48]);
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits{context_entries:5123,..l}).is_err());
    }
    #[test]
    fn refinement_executor_unsupported_modes_fail_closed(){
        let g=seg(1,0,0,&[],SOURCE);
        // Retaining GR is now supported. A direct Huffman dictionary is
        // still not a compatible retained refinement source.
        let mut used=body(1,T1);used[0]|=1;
        let mut p=info(2);p.extend(seg(3,0,1,&[1],&used));
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
        let mut p=info(2);p.extend(seg(3,0,1,&[1],&body(1,&[124,191,255,172])));
        assert!(matches!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()),Err(Error::Unsupported(_))));
        // A direct Huffman dictionary cannot supply retained GR.
        let mut p=info(2);p.extend(seg(3,0,1,&[1],&[17,3,0,0,0,1,0,0,0,0,0,64]));
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
    }
    #[test]
    fn refinement_executor_bad_dependency_or_entropy(){
        let g=seg(1,0,0,&[],SOURCE);
        for (refs,entropy) in [(&[9][..],T1),(&[][..],T1),(&[1][..],&[124,219,255,172][..]),(&[1][..],&T1[..T1.len()-1]) ]{
            let mut p=info(2);p.extend(seg(3,0,1,refs,&body(1,entropy)));
            assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
        }
    }
}


#[cfg(test)]
mod retained_gr_tests{
    use super::*;
    const SOURCE:&[u8]=&[0,1,0,0,0,1,0,0,0,1,229,211,240,0,0,124,124,124,0,0,0,64];
    const T0:&[u8]=&[124,227,105,204,198,86,197,71,223,255,172];
    const T1:&[u8]=&[124,227,105,204,198,86,191,231,207,255,172];
    // Independent Annex E JS encoder: new QM stream, all integer banks
    // reset; inherit only GR from the parent. Both templates happen to
    // produce the same bytes. 43 decisions, 6 pixels, four changed states.
    const CHILD:&[u8]=&[124,227,108,255,96,191,255,172];
    fn body(t:u8,flags:u8,child:bool,entropy:&[u8])->Vec<u8>{
        let mut v=vec![8|(t<<4)|flags,2,2,255];if t==0{v.extend([255;4]);}
        v.extend(if child{[0,0,0,4,0,0,0,1]}else{[0,0,0,3,0,0,0,2]});
        v.extend(entropy);v
    }
    fn parent(t:u8,flags:u8)->Vec<u8>{body(t,flags,false,if t==0{T0}else{T1})}
    fn plan_for(data:&[u8],imported:u32)->Plan{
        let (header,_)=parse(data).unwrap();let start=header.data_offset;
        Plan{header,imported,dictionaries:vec![],tables:None,context_source:None,encoded:start..data.len()}
    }
    fn src()->Bitmap{Bitmap::from_packed(7,6,&[0,124,124,124,0,0]).unwrap()}
    fn seg(id:u8,kind:u8,page:u8,refs:&[u8],data:&[u8])->Vec<u8>{
        let mut v=vec![0,0,0,id,kind,(refs.len()as u8)<<5];v.extend(refs);
        v.push(page);v.extend((data.len()as u32).to_be_bytes());v.extend(data);v
    }
    fn info(id:u8)->Vec<u8>{
        let mut d=vec![];for n in [20u32,20,300,300]{d.extend(n.to_be_bytes());}
        d.extend([0,0,0]);seg(id,48,1,&[],&d)
    }
    fn fixture(t:u8,child_flags:u8)->(Vec<u8>,Vec<u8>){
        let mut g=seg(1,0,0,&[],SOURCE);g.extend(seg(2,0,0,&[1],&parent(t,2)));
        let mut p=info(3);p.extend(seg(4,0,1,&[2],&body(t,child_flags,true,CHILD)));(g,p)
    }
    fn ids(d:&DictionarySet,n:u32)->Vec<usize>{
        d.dictionary(n).unwrap().iter().map(|s|s.index()).collect()
    }
    #[test]
    fn aggregation_context_existing_reference_vectors(){
        for t in [0,1]{
            let s=src();let p=parent(t,2);let l=ArithmeticLimits::default();
            let plan=plan_for(&p,1);
            let (legacy,expected)=decode_arithmetic_single_refinement_with_context(&plan,&p,&[&s],l,None).unwrap();
            let (d,snap)=decode_arithmetic_aggregation_with_context(&plan,&p,&[&s],l,None).unwrap();
            assert_eq!(d.new_symbols,legacy.new_symbols);
            assert_eq!(snap,expected);
            let snap=snap.unwrap();
            let b=body(t,3,true,CHILD);let mut plan=plan_for(&b,3);
            let imports=[&s,&d.new_symbols[0],&d.new_symbols[1]];
            let (child,after)=decode_arithmetic_aggregation_with_context(&plan,&b,&imports,l,Some(&snap)).unwrap();
            assert_eq!(child.new_symbols[0].packed(),[160,64]);
            assert_eq!(child.exported_indices,[0,1,2,3]);
            let (legacy,expected)=decode_arithmetic_single_refinement_with_context(&plan,&b,&imports,l,Some(&snap)).unwrap();
            assert_eq!(child.new_symbols,legacy.new_symbols);
            assert_eq!(after,expected);
            let exact=ArithmeticLimits{work:child.work_used,..l};
            assert!(decode_arithmetic_aggregation_with_context(&plan,&b,&imports,exact,Some(&snap)).is_ok());
            assert!(decode_arithmetic_aggregation_with_context(&plan,&b,&imports,ArithmeticLimits{work:child.work_used-1,..l},Some(&snap)).is_err());
            assert!(decode_arithmetic_aggregation_with_context(&plan,&b,&imports,l,None).is_err());
            assert!(decode_arithmetic_aggregation(&plan,&b,&imports,l).is_err());
            plan.header.context_used=false;
            assert!(decode_arithmetic_aggregation_with_context(&plan,&b,&imports,l,Some(&snap)).is_err());
        }
    }
    #[test]
    fn aggregation_executor_legacy_retained_parity(){
        for t in [0,1]{
            for flags in [1,3]{
                let (g,p)=fixture(t,flags);
                let l=ExecutionLimits::default();
                let legacy=execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap();
                let aggregate=execute_pdf_dictionaries_with_aggregation(&g,&p,l).unwrap();
                assert_eq!(legacy.arena,aggregate.arena);
                assert_eq!(legacy.exports,aggregate.exports);
                let a=legacy.stats();let z=aggregate.stats();
                assert_eq!((a.dictionaries,a.new_symbols,a.pixels,a.packed_bytes,a.retained_gr_contexts,a.reused_gr_contexts),
                    (z.dictionaries,z.new_symbols,z.pixels,z.packed_bytes,z.retained_gr_contexts,z.reused_gr_contexts));
                assert_eq!(z.arithmetic_aggregation,2);
                assert_eq!(z.single_refinement,0);
                assert_eq!(z.peak_context_entries,a.peak_context_entries+9*512);
                assert_eq!(z.symbol_slots,a.symbol_slots+5);
                assert_eq!(z.work_used,a.work_used+2*9*512+5);
                let tight=ExecutionLimits{context_entries:a.peak_context_entries,..l};
                assert!(execute_pdf_dictionaries_single_reference(&g,&p,tight).is_ok());
                assert!(execute_pdf_dictionaries_with_aggregation(&g,&p,tight).is_err());
            }
        }
    }
    #[test]
    fn retained_gr_both_templates_reference(){
        for t in [0,1]{
            let s=src();let p=parent(t,2);let l=ArithmeticLimits::default();
            let (d,snap)=decode_arithmetic_single_refinement_with_context(&plan_for(&p,1),&p,&[&s],l,None).unwrap();
            let snap=snap.unwrap();
            assert_eq!(snap.entries(),if t==0{8192}else{1024});
            assert_eq!(snap.nonzero_entries(),if t==0{12}else{11});
            let b=body(t,3,true,CHILD);
            let imports=[&s,&d.new_symbols[0],&d.new_symbols[1]];
            let (child,after)=decode_arithmetic_single_refinement_with_context(&plan_for(&b,3),&b,&imports,l,Some(&snap)).unwrap();
            let after=after.unwrap();
            assert_eq!(child.exported_indices,[0,1,2,3]);
            assert_eq!(child.new_symbols[0].packed(),[160,64]);
            assert_eq!(child.pixel_decisions,6);
            assert_eq!(snap.bank.entries().iter().zip(after.bank.entries()).filter(|(a,b)|a!=b).count(),4);
            assert_eq!(snap.nonzero_entries(),after.nonzero_entries());
            let b=body(t,0,true,CHILD);
            let reset=decode_arithmetic_single_refinement(&plan_for(&b,3),&b,&imports,l);
            if let Ok(reset)=reset{
                assert!(reset.new_symbols!=child.new_symbols||reset.exported_indices!=child.exported_indices);
            }
        }
    }
    #[test]
    fn retained_gr_executor_global_to_page(){
        for t in [0,1]{
            let (g,p)=fixture(t,1);
            let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
            assert_eq!(ids(&d,4),[0,1,2,3]);assert_eq!(d.symbol(4,3).unwrap().packed(),[160,64]);
            assert_eq!((d.stats().retained_gr_contexts,d.stats().reused_gr_contexts),(1,1));
            assert_eq!((d.stats().retained_contexts,d.stats().reused_contexts),(1,1));
            assert_eq!((d.stats().new_symbols,d.stats().pixels,d.stats().packed_bytes),(4,62,12));
            assert!(std::ptr::eq(d.symbol(2,1).unwrap(),d.symbol(4,1).unwrap()));
            assert!(!std::ptr::eq(d.symbol(2,1).unwrap(),d.symbol(4,3).unwrap()));
        }
    }
    #[test]
    fn retained_gr_immutable_branch(){
        let (g,mut p)=fixture(1,3);
        p.extend(seg(5,0,1,&[2],&body(1,1,true,CHILD)));
        let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
        assert_eq!(ids(&d,4),[0,1,2,3]);assert_eq!(ids(&d,5),[0,1,2,4]);
        assert_eq!(d.symbol(4,3),d.symbol(5,3));
        assert_eq!(d.stats().retained_gr_contexts,2);assert_eq!(d.stats().reused_gr_contexts,2);
        assert_eq!(d.stats().peak_context_entries,6148);
    }

    #[test]
    fn reclaim_arithmetic_gr_branch(){
        let (g,mut p)=fixture(1,3);
        p.extend(seg(5,0,1,&[2],&body(1,1,true,CHILD)));
        let l=ExecutionLimits::default();let old=execute_pdf_dictionaries(&g,&p,l).unwrap();
        let d=execute_pdf_dictionaries_reclaimed_contexts(&g,&p,l).unwrap();
        assert_reclaimed_context_parity(&old,&d);
        assert_eq!(old.stats().peak_context_entries-d.stats().peak_context_entries,1024);
        assert_eq!((d.stats().released_contexts,d.stats().released_context_entries,d.stats().reused_gr_contexts),(2,2048,2));
        let exact=ExecutionLimits{context_entries:d.stats().peak_context_entries,work:d.stats().work_used,..l};
        assert_eq!(execute_pdf_dictionaries_reclaimed_contexts(&g,&p,exact).unwrap(),d);
        assert!(execute_pdf_dictionaries(&g,&p,exact).is_err());
        assert!(execute_pdf_dictionaries_reclaimed_contexts(&g,&p,ExecutionLimits{context_entries:exact.context_entries-1,..exact}).is_err());
    }
    #[test]
    fn retained_gr_exact_context_and_work(){
        let (g,p)=fixture(1,1);
        let l=ExecutionLimits{context_entries:5124,..ExecutionLimits::default()};
        let d=execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap();
        assert_eq!(d.stats().peak_context_entries,5124);
        let l=ExecutionLimits{work:d.stats().work_used,..l};
        assert_eq!(execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap(),d);
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits{context_entries:5123,..l}).is_err());
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits{work:l.work-1,..l}).is_err());
    }
    #[test]
    fn retained_gr_presence_and_geometry(){
        let s=src();let p=parent(0,2);let l=ArithmeticLimits::default();
        let (d,snap)=decode_arithmetic_single_refinement_with_context(&plan_for(&p,1),&p,&[&s],l,None).unwrap();
        let snap=snap.unwrap();let imports=[&s,&d.new_symbols[0],&d.new_symbols[1]];
        let b=body(0,1,true,CHILD);let mut plan=plan_for(&b,3);
        assert!(decode_arithmetic_single_refinement_with_context(&plan,&b,&imports,l,None).is_err());
        plan.header.context_used=false;
        assert!(decode_arithmetic_single_refinement_with_context(&plan,&b,&imports,l,Some(&snap)).is_err());
        plan.header.context_used=true;plan.header.adaptive[0].x=1;
        assert!(decode_arithmetic_single_refinement_with_context(&plan,&b,&imports,l,Some(&snap)).is_err());
        plan.header.adaptive[0].x=2;plan.header.refinement_adaptive[1].x=0;
        assert!(decode_arithmetic_single_refinement_with_context(&plan,&b,&imports,l,Some(&snap)).is_err());
        let b=body(1,1,true,CHILD);
        assert!(decode_arithmetic_single_refinement_with_context(&plan_for(&b,3),&b,&imports,l,Some(&snap)).is_err());
    }
    #[test]
    fn retained_gr_last_dependency_validation(){
        let (_,p)=fixture(1,1);
        let mut g=seg(1,0,0,&[],SOURCE);g.extend(seg(2,0,0,&[1],&parent(1,0)));
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
        let (g,_)=fixture(1,1);
        // Last dependency is the direct dictionary, not the retained GR
        // dictionary. Never search backward for any compatible snapshot.
        let mut p=info(3);p.extend(seg(4,0,1,&[2,1],&body(1,1,true,CHILD)));
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
        let mut p=info(3);p.extend(seg(4,0,1,&[2],&body(0,1,true,CHILD)));
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
    }
    #[test]
    fn retained_gr_zero_bank_without_retain(){
        let s=src();let b=parent(1,0);let p=plan_for(&b,1);let l=ArithmeticLimits::default();
        let (d,snap)=decode_arithmetic_single_refinement_with_context(&p,&b,&[&s],l,None).unwrap();
        assert!(snap.is_none());
        assert_eq!(decode_arithmetic_single_refinement(&p,&b,&[&s],l).unwrap(),d);
        let b=parent(1,2);
        assert!(decode_arithmetic_single_refinement(&plan_for(&b,1),&b,&[&s],l).is_err());
    }
}


/// T.88 6.5.8.2.3: unlike arithmetic IAID, Huffman IDs use at least one bit.
pub fn huffman_refinement_id_bits(imported:u32,new:u32)->Result<u8>{
    Ok(refinement_id_bits(imported,new)?.max(1))
}

#[derive(Debug,PartialEq,Eq)]
pub struct HuffmanRefinementDictionary{
    pub new_symbols:Vec<Bitmap>,
    pub exported_indices:Vec<u32>,
    pub classes:u32,
    /// Decoded new symbols (legacy field name), not number of QM substreams.
    pub blocks:u32,
    pub entropy_bytes:usize,
    /// Actual QM pixel decisions, including temporary refined instances.
    pub pixel_decisions:u64,
    /// Peak simultaneous temporary borrowed references for multi-instance
    /// symbols, zero on the legacy single-instance path (not RSS).
    pub aggregate_reference_slots:usize,
    pub packed_bytes:usize,
    pub work_used:u64,
}
fn refinement_huff_value(table:&Table,reader:&mut BitReader<'_>,left:&mut u64)->Result<Integer>{
    let start=reader.position();
    let v=table.decode(reader)?;
    charge_work(left,1+(reader.position()-start)as u64)?;
    Ok(v)
}
fn refinement_huff_integer(table:&Table,reader:&mut BitReader<'_>,left:&mut u64)->Result<i64>{
    match refinement_huff_value(table,reader,left)?{
        Integer::Value(v)=>Ok(v),
        Integer::Oob=>Err(Error::Invalid("JBIG2 Huffman refinement integer OOB")),
    }
}

/// Huffman refinement snapshots are deliberately distinct from arithmetic
/// dictionary snapshots. Only GR probabilities exist in this subset.
/// No Huffman bit cursor, QM registers or table selection is retained.
#[derive(Debug,PartialEq,Eq)]
pub struct HuffmanRefinementContextSnapshot{
    neighborhood:super::refinement::Neighborhood,
    bank:super::refinement::ContextBank,
}
impl HuffmanRefinementContextSnapshot{
    pub fn entries(&self)->usize{self.bank.entries().len()}
    pub fn template(&self)->super::refinement::Template{self.bank.template()}
    pub fn nonzero_entries(&self)->usize{
        self.bank.entries().iter().filter(|&&v|v!=super::qm::Context::default()).count()
    }
}
/// Compatibility wrapper for callers that do not manage snapshots.
pub fn decode_huffman_single_refinement(plan:&Plan,stream:&[u8],imports:&[&Bitmap],
    limits:ArithmeticLimits)->Result<HuffmanRefinementDictionary>{
    if plan.header.context_used||plan.header.context_retained||plan.context_source.is_some(){
        return Err(Error::Unsupported("JBIG2 Huffman refinement requires context-aware entry"));
    }
    Ok(decode_huffman_single_refinement_with_context(plan,stream,imports,limits,None)?.0)
}
/// T.88 6.5.8.2.1–3/Table 18 and 7.4.2.2. Restricted to one instance per
/// new symbol, positive dimensions, known block size and explicit FF AC.
/// Each block restarts QM registers but preserves GR probabilities.
/// Executor verifies the last referred dictionary's provenance; standalone
/// callers must supply the matching immutable snapshot. No multi-instance
/// aggregation. Final marker is a boundary check, not entropy integrity.
pub fn decode_huffman_single_refinement_with_context(plan:&Plan,stream:&[u8],imports:&[&Bitmap],
    limits:ArithmeticLimits,initial:Option<&HuffmanRefinementContextSnapshot>)
    ->Result<(HuffmanRefinementDictionary,Option<HuffmanRefinementContextSnapshot>)>{
    decode_huffman_refinement_impl(plan,stream,imports,limits,initial,false,0)
}
/// Opt-in Table 17 aggregate and Table 18 single refinement using one GR bank.
/// Snapshot provenance remains the responsibility of the segment executor.
pub fn decode_huffman_aggregation_with_context(plan:&Plan,stream:&[u8],imports:&[&Bitmap],
    limits:ArithmeticLimits,initial:Option<&HuffmanRefinementContextSnapshot>)
    ->Result<(HuffmanRefinementDictionary,Option<HuffmanRefinementContextSnapshot>)>{
    decode_huffman_refinement_impl(plan,stream,imports,limits,initial,true,MAX_SYMBOLS as usize)
}
fn decode_huffman_refinement_impl(plan:&Plan,stream:&[u8],imports:&[&Bitmap],
    limits:ArithmeticLimits,initial:Option<&HuffmanRefinementContextSnapshot>,aggregate:bool,aggregate_slot_limit:usize)
    ->Result<(HuffmanRefinementDictionary,Option<HuffmanRefinementContextSnapshot>)>{
    use super::{refinement,text_region};
    let h=&plan.header;
    if !h.huffman||!h.refine_aggregate{
        return Err(Error::Unsupported("JBIG2 single Huffman refinement mode"));
    }
    if h.template!=Template::T0||!h.adaptive.is_empty(){
        return Err(Error::Invalid("JBIG2 Huffman generic parameters"));
    }
    if h.context_used!=initial.is_some()||(!h.context_used&&plan.context_source.is_some()){
        return Err(Error::Invalid("JBIG2 Huffman refinement context presence"));
    }
    if imports.len()!=plan.imported as usize||h.selections.is_none(){
        return Err(Error::Invalid("JBIG2 Huffman refinement imports/tables"));
    }
    if limits.pixels>bitmap::MAX_PIXELS||limits.packed_bytes>bitmap::MAX_PACKED_BYTES||
        limits.work>MAX_DICTIONARY_WORK||limits.export_runs>MAX_EXPORT_RUNS||
        limits.decisions>MAX_ARITHMETIC_DECISIONS{
        return Err(Error::Limit("JBIG2 Huffman refinement limits"));
    }
    let bits=huffman_refinement_id_bits(plan.imported,h.new_symbols)?;
    let total=plan.imported+h.new_symbols;
    if h.exported>total{return Err(Error::Invalid("JBIG2 Huffman refinement export count"));}
    if h.new_symbols>0&&imports.is_empty(){return Err(Error::Invalid("JBIG2 Huffman refinement no reference"));}
    let data=stream.get(plan.encoded.clone()).ok_or(Error::Invalid("JBIG2 Huffman refinement span"))?;
    if data.len()>MAX_BYTES||data.len()>ccitt::MAX_BITS/8{return Err(Error::Limit("JBIG2 Huffman refinement input"));}
    let tables=plan.tables.as_ref().ok_or(Error::Invalid("JBIG2 Huffman refinement tables"))?;
    let template=refinement::Template::from_code(h.refinement_template)?;
    let neighborhood=refinement::Neighborhood::new(template,&h.refinement_adaptive)?;
    if let Some(snapshot)=initial{
        if snapshot.neighborhood!=neighborhood||snapshot.template()!=template{
            return Err(Error::Invalid("JBIG2 Huffman GR snapshot parameters"));
        }
    }
    let mut left=limits.work;
    charge_work(&mut left,template.context_count()as u64)?;
    if let Some(snapshot)=initial{charge_work(&mut left,snapshot.entries() as u64)?;}
    let mut gr=refinement::ContextBank::new(template)?;
    if let Some(snapshot)=initial{gr.restore_from(&snapshot.bank)?;}
    let offset=huffman::standard::table(15)?;
    let size=huffman::standard::table(1)?;
    let mut reader=BitReader::new(data,data.len()*8)?;
    let mut base=0usize;
    let mut state=DimensionState::new(h.new_symbols,limits.pixels)?;
    let mut out:Vec<Bitmap>=Vec::new();
    out.try_reserve_exact(h.new_symbols as usize).map_err(|_|Error::Limit("JBIG2 Huffman refinement symbols allocation"))?;
    let mut packed=0usize;
    let mut entropy_bytes=0usize;
    let mut aggregate_reference_slots=0usize;
    let mut decisions=limits.decisions;
    while !state.complete(){
        state.begin(refinement_huff_value(&tables[0].table,&mut reader,&mut left)?)?;
        loop{
            match state.width(refinement_huff_value(&tables[1].table,&mut reader,&mut left)?)?{
                WidthEvent::End(_)=>break,
                WidthEvent::Symbol(s)=>{
                    let bytes=packed_len(s.width,s.height)?;
                    let next=packed.checked_add(bytes).ok_or(Error::Limit("JBIG2 Huffman refinement packed bytes"))?;
                    if next>limits.packed_bytes{return Err(Error::Limit("JBIG2 Huffman refinement packed budget"));}
                    match refinement_huff_integer(&tables[3].table,&mut reader,&mut left)?{
                        1=>{},
                        n if n>1=>{
                            if !aggregate{return Err(Error::Unsupported("JBIG2 multiple Huffman symbol aggregation"));}
                            let instances=u32::try_from(n).map_err(|_|Error::Limit("JBIG2 Huffman aggregate instances"))?;
                            if instances>text_region::MAX_INSTANCES{return Err(Error::Limit("JBIG2 Huffman aggregate instances"));}
                            let available=plan.imported+s.index;
                            let count=available as usize;
                            if count>aggregate_slot_limit{return Err(Error::Limit("JBIG2 dictionary symbol slots"));}
                            // Charge peak namespace growth once, and construction
                            // on each use. Reserve before allocating any references.
                            if count>aggregate_reference_slots{
                                charge_work(&mut left,(count-aggregate_reference_slots) as u64)?;
                                aggregate_reference_slots=count;
                            }
                            charge_work(&mut left,u64::from(available))?;
                            let mut refs=Vec::new();
                            refs.try_reserve_exact(count).map_err(|_|Error::Limit("JBIG2 Huffman aggregate references"))?;
                            refs.extend(imports.iter().copied());refs.extend(out.iter());
                            if refs.len()!=available as usize{return Err(Error::Invalid("JBIG2 Huffman aggregate namespace"));}
                            let decoded=text_region::decode_huffman_aggregate_bitmap(&mut reader,s.width,s.height,
                                instances,&refs,total,&neighborhood,&mut gr,decisions,left.min(bitmap::MAX_COMPOSE_WORK),
                                limits.packed_bytes-next)?;
                            charge_work(&mut left,decoded.decoded.stats.work_used)?;
                            decisions=decisions.checked_sub(decoded.pixel_decisions).ok_or(Error::Limit("JBIG2 Huffman aggregate decisions"))?;
                            entropy_bytes=entropy_bytes.checked_add(decoded.entropy_bytes).ok_or(Error::Limit("JBIG2 Huffman aggregate entropy bytes"))?;
                            out.push(decoded.decoded.bitmap);packed=next;
                            continue;
                        },
                        _=>return Err(Error::Invalid("JBIG2 Huffman refinement instance count")),
                    }
                    charge_work(&mut left,u64::from(bits))?;
                    let mut id=0u32;
                    for _ in 0..bits{id=(id<<1)|u32::from(reader.bit()?);}
                    if id>=plan.imported+s.index{return Err(Error::Invalid("JBIG2 Huffman refinement forward symbol"));}
                    let dx=refinement_huff_integer(&offset,&mut reader,&mut left)?;
                    let dy=refinement_huff_integer(&offset,&mut reader,&mut left)?;
                    // The fixed B.1 RSIZE is not the selected collective
                    // BMSIZE table. Skip arbitrary alignment bits.
                    let n=usize::try_from(refinement_huff_integer(&size,&mut reader,&mut left)?)
                        .map_err(|_|Error::Invalid("JBIG2 Huffman refinement negative block size"))?;
                    if n<2{return Err(Error::Invalid("JBIG2 Huffman refinement short block"));}
                    let rounded=reader.position().div_ceil(8);
                    charge_work(&mut left,(rounded*8-reader.position())as u64)?;
                    let start=base.checked_add(rounded).ok_or(Error::Limit("JBIG2 Huffman refinement block offset"))?;
                    let end=start.checked_add(n).ok_or(Error::Limit("JBIG2 Huffman refinement block end"))?;
                    let block=data.get(start..end).ok_or(Error::Invalid("JBIG2 Huffman refinement block truncated"))?;
                    let reference=if (id as usize)<imports.len(){imports[id as usize]}else{
                        out.get(id as usize-imports.len()).ok_or(Error::Invalid("JBIG2 Huffman refinement reference"))?
                    };
                    let opt=refinement::RefinementOptions{width:s.width,height:s.height,dx,dy,typical_prediction:false};
                    let work=refinement::work_bound(template,opt)?;
                    charge_work(&mut left,n as u64)?;
                    charge_work(&mut left,work)?;
                    let area=usize::try_from(u64::from(s.width)*u64::from(s.height))
                        .map_err(|_|Error::Limit("JBIG2 Huffman refinement decisions"))?;
                    decisions=decisions.checked_sub(area).ok_or(Error::Limit("JBIG2 Huffman refinement decisions"))?;
                    let mut qm=super::qm::Decoder::new(block,area)?;
                    let decoded=refinement::decode_bitmap(reference,&neighborhood,opt,&mut gr,&mut qm,work)?;
                    // Termination was checked on the exact declared slice.
                    // Resume Huffman at the declared end, not MQ read-ahead.
                    base=end;
                    let rest=&data[base..];
                    reader=BitReader::new(rest,rest.len()*8)?;
                    entropy_bytes=entropy_bytes.checked_add(n).ok_or(Error::Limit("JBIG2 Huffman refinement entropy bytes"))?;
                    out.push(decoded.bitmap);packed=next;
                }
            }
        }
    }
    let indices=export_indices(plan.imported,h.new_symbols,h.exported,limits.export_runs,||{
        refinement_huff_value(&size,&mut reader,&mut left)
    })?;
    if reader.remaining()>7{return Err(Error::Invalid("JBIG2 Huffman refinement trailing bytes"));}
    charge_work(&mut left,reader.remaining()as u64)?;
    let snapshot=if h.context_retained{Some(HuffmanRefinementContextSnapshot{neighborhood,bank:gr})}else{None};
    Ok((HuffmanRefinementDictionary{new_symbols:out,exported_indices:indices,classes:state.classes,
        blocks:state.decoded(),entropy_bytes,pixel_decisions:(limits.decisions-decisions) as u64,aggregate_reference_slots,packed_bytes:packed,work_used:limits.work-left},snapshot))
}

#[cfg(test)]
mod huffman_refinement_tests{
    use super::*;
    // Independent JS Annex E encoder with two fresh QM streams and a
    // shared GR bank. B.4/B.2/B.1/B.15 fields packed independently.
    // Both templates: 3x2 [160,64], then 4x2 [192,48], 14 decisions.
    const T0:&[u8]=&[184,4,176,96,187,255,172,130,202,64,221,127,255,172,252,3];
    const T1:&[u8]=&[184,4,176,96,187,255,172,130,202,64,220,127,255,172,252,3];
    const CUSTOM_A:&[u8]=&[0x42,0,0,0,0,0,1,1,0x10,0x49,0x23,0x81,0x80];
    fn body(t:u8,data:&[u8])->Vec<u8>{
        let mut v=vec![t<<4,3];if t==0{v.extend([255;4]);}
        v.extend([0,0,0,3,0,0,0,2]);v.extend(data);v
    }
    fn make_plan(data:&[u8])->Plan{
        let (header,_)=parse(data).unwrap();let start=header.data_offset;
        Plan{header,imported:1,dictionaries:vec![],tables:Some([4,2,1,1].map(|n|
            SelectedTable{source:TableSource::Standard(n),table:huffman::standard::table(n).unwrap()})),
            context_source:None,encoded:start..data.len()}
    }
    fn src()->Bitmap{Bitmap::from_packed(7,6,&[0,124,124,124,0,0]).unwrap()}
    fn check(d:&HuffmanRefinementDictionary){
        assert_eq!((d.classes,d.blocks,d.entropy_bytes,d.pixel_decisions,d.packed_bytes),(1,2,7,14,4));
        assert_eq!(d.exported_indices,[0,1,2]);
        assert_eq!(d.new_symbols[0].packed(),[160,64]);assert_eq!(d.new_symbols[1].packed(),[192,48]);
        assert_eq!((d.new_symbols[0].width(),d.new_symbols[0].height()),(3,2));
        assert_eq!((d.new_symbols[1].width(),d.new_symbols[1].height()),(4,2));
    }
    #[test]
    fn huffman_refinement_both_templates(){
        for (t,entropy) in [(0,T0),(1,T1)]{
            let b=body(t,entropy);
            check(&decode_huffman_single_refinement(&make_plan(&b),&b,&[&src()],ArithmeticLimits::default()).unwrap());
        }
    }
    #[test]
    fn huffman_refinement_custom_selected_agginst(){
        let mut b=body(1,T1);b[1]|=0x80;
        let mut p=make_plan(&b);
        p.tables.as_mut().unwrap()[3]=SelectedTable{source:TableSource::Custom(1),table:huffman::parse_custom(CUSTOM_A).unwrap().into_table()};
        check(&decode_huffman_single_refinement(&p,&b,&[&src()],ArithmeticLimits::default()).unwrap());
        // Slot 2 is not consulted for single-refinement RSIZE.
        p.tables.as_mut().unwrap()[2]=SelectedTable{source:TableSource::Standard(4),table:huffman::standard::table(4).unwrap()};
        check(&decode_huffman_single_refinement(&p,&b,&[&src()],ArithmeticLimits::default()).unwrap());
    }
    #[test]
    fn huffman_refinement_alignment_and_id_width(){
        for (n,k) in [(0,1),(1,1),(2,1),(3,2),(4,2),(5,3),(MAX_SYMBOLS,20)]{
            assert_eq!(huffman_refinement_id_bits(0,n).unwrap(),k);
        }
        assert!(huffman_refinement_id_bits(MAX_SYMBOLS,1).is_err());
        let mut e=T1.to_vec();
        // First header is 27 bits, second 20 bits, export suffix 16.
        e[3]|=0x1f;e[9]|=0x0f;
        let b=body(1,&e);
        check(&decode_huffman_single_refinement(&make_plan(&b),&b,&[&src()],ArithmeticLimits::default()).unwrap());
    }
    #[test]
    fn huffman_refinement_bad_count_id_and_size(){
        for (at,v,unsupported) in [(1,0,false),(1,8,true),(1,5,false),(3,0,false),(3,64,false),(3,128,false)]{
            let mut e=T1.to_vec();e[at]=v;let b=body(1,&e);
            let err=decode_huffman_single_refinement(&make_plan(&b),&b,&[&src()],ArithmeticLimits::default()).unwrap_err();
            assert_eq!(matches!(err,Error::Unsupported(_)),unsupported);
        }
    }
    #[test]
    fn huffman_refinement_exact_cumulative_budgets(){
        let b=body(1,T1);let p=make_plan(&b);let s=src();
        let d=decode_huffman_single_refinement(&p,&b,&[&s],ArithmeticLimits::default()).unwrap();
        let l=ArithmeticLimits{pixels:14,packed_bytes:4,work:d.work_used,decisions:14,export_runs:2};
        assert_eq!(decode_huffman_single_refinement(&p,&b,&[&s],l).unwrap(),d);
        for l in [ArithmeticLimits{pixels:13,..l},ArithmeticLimits{packed_bytes:3,..l},
            ArithmeticLimits{work:l.work-1,..l},ArithmeticLimits{decisions:13,..l},
            ArithmeticLimits{export_runs:1,..l}]{
            assert!(decode_huffman_single_refinement(&p,&b,&[&s],l).is_err());
        }
    }
    #[test]
    fn huffman_refinement_truncation_and_markers(){
        let s=src();let l=ArithmeticLimits::default();
        for n in 0..T1.len(){
            let b=body(1,&T1[..n]);assert!(decode_huffman_single_refinement(&make_plan(&b),&b,&[&s],l).is_err());
        }
        for (i,v) in [(6,0),(6,0x90),(13,0),(13,0x90)]{
            let mut e=T1.to_vec();e[i]=v;let b=body(1,&e);
            assert!(decode_huffman_single_refinement(&make_plan(&b),&b,&[&s],l).is_err());
        }
        let mut e=T1.to_vec();e.push(0);let b=body(1,&e);
        assert!(decode_huffman_single_refinement(&make_plan(&b),&b,&[&s],l).is_err());
    }
    #[test]
    fn huffman_refinement_modes_and_span(){
        let b=body(1,T1);let mut p=make_plan(&b);let s=src();let l=ArithmeticLimits::default();
        assert!(decode_huffman_single_refinement(&p,&b,&[],l).is_err());
        p.header.context_retained=true;
        assert!(decode_huffman_single_refinement(&p,&b,&[&s],l).is_err());
        p.header.context_retained=false;p.header.context_used=true;
        assert!(decode_huffman_single_refinement(&p,&b,&[&s],l).is_err());
        p.header.context_used=false;p.header.refine_aggregate=false;
        assert!(decode_huffman_single_refinement(&p,&b,&[&s],l).is_err());
        p.header.refine_aggregate=true;p.encoded.end=usize::MAX;
        assert!(decode_huffman_single_refinement(&p,&b,&[&s],l).is_err());
    }
}


#[cfg(test)]
mod huffman_refinement_executor_tests{
    use super::*;
    const SOURCE:&[u8]=&[0,1,0,0,0,1,0,0,0,1,229,211,240,0,0,124,124,124,0,0,0,64];
    const H0:&[u8]=&[184,4,176,96,187,255,172,130,202,64,221,127,255,172,252,3];
    const H1:&[u8]=&[184,4,176,96,187,255,172,130,202,64,220,127,255,172,252,3];
    const A1:&[u8]=&[124,227,105,204,198,86,191,231,207,255,172];
    const CUSTOM_A:&[u8]=&[0x42,0,0,0,0,0,1,1,0x10,0x49,0x23,0x81,0x80];
    const RETAINED_GB:&[u8]=&[10,0,2,255,0,0,0,2,0,0,0,2,0x4f,0xe7,0x8c,0x20,0x0e,0x1d,0xc7,0xcf,1,0x11,0xc4,0xb2,0x6f,255,172];
    // Independently encoded new symbol 4x2 [192,48], with fresh GR
    // probabilities, source ID 1 (parent's new 3x2 symbol), dx=-1,dy=1.
    // Huffman has an independent 3-byte QM block; arithmetic has 48 decisions.
    const HCHILD:&[u8]=&[184,133,148,96,233,255,172,252,4];
    const ACHILD:&[u8]=&[123,157,178,5,65,198,127,255,172];
    fn body(t:u8,huffman:bool,child:bool,entropy:&[u8])->Vec<u8>{
        let mut v=if huffman{vec![t<<4,3]}else{vec![8|(t<<4),2,2,255]};
        if t==0{v.extend([255;4]);}
        v.extend(if child{[0,0,0,4,0,0,0,1]}else{[0,0,0,3,0,0,0,2]});
        v.extend(entropy);v
    }
    fn seg(id:u8,kind:u8,page:u8,refs:&[u8],data:&[u8])->Vec<u8>{
        let mut v=vec![0,0,0,id,kind,(refs.len()as u8)<<5];v.extend(refs);
        v.push(page);v.extend((data.len()as u32).to_be_bytes());v.extend(data);v
    }
    fn info(id:u8)->Vec<u8>{
        let mut d=vec![];for n in [20u32,20,300,300]{d.extend(n.to_be_bytes());}
        d.extend([0,0,0]);seg(id,48,1,&[],&d)
    }
    fn reexport(n:u8)->Vec<u8>{vec![0,1,0,0,0,n,0,0,0,0,n>>2,n<<6]}
    fn fixture(t:u8)->(Vec<u8>,Vec<u8>){
        let g=seg(1,0,0,&[],SOURCE);let mut p=info(2);
        p.extend(seg(3,0,1,&[1],&body(t,true,false,if t==0{H0}else{H1})));
        p.extend(seg(4,0,1,&[3,1],&reexport(4)));(g,p)
    }
    fn ids(d:&DictionarySet,n:u32)->Vec<usize>{d.dictionary(n).unwrap().iter().map(|s|s.index()).collect()}
    #[test]
    fn default_executor_huffman_refinement_exact_parity(){
        for t in [0,1]{
            let (g,p)=fixture(t);let l=ExecutionLimits::default();
            let old=execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap();
            let exact=ExecutionLimits{work:old.stats().work_used,symbol_slots:old.stats().symbol_slots,
                context_entries:old.stats().peak_context_entries,..l};
            assert_eq!(execute_pdf_dictionaries(&g,&p,exact).unwrap(),old);
        }
    }
    #[test]
    fn huffman_executor_both_templates_and_identity(){
        for t in [0,1]{
            let (g,p)=fixture(t);let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
            let s=d.stats();
            assert_eq!((s.dictionaries,s.huffman,s.arithmetic,s.single_refinement,s.huffman_single_refinement),(3,3,0,1,1));
            assert_eq!((s.new_symbols,s.pixels,s.packed_bytes,s.symbol_slots),(3,56,10,17));
            assert_eq!(s.peak_context_entries,if t==0{8192}else{1024});
            assert_eq!(ids(&d,3),[0,1,2]);assert_eq!(ids(&d,4),[0,1,2,0]);
            assert_eq!(d.symbol(3,1).unwrap().packed(),[160,64]);assert_eq!(d.symbol(3,2).unwrap().packed(),[192,48]);
            for i in 0..3{assert!(std::ptr::eq(d.symbol(3,i).unwrap(),d.symbol(4,i).unwrap()));}
            assert!(std::ptr::eq(d.symbol(1,0).unwrap(),d.symbol(4,3).unwrap()));
        }
    }
    #[test]
    fn huffman_executor_cross_mode_reference_chains(){
        for parent_huff in [false,true]{
            let mut g=seg(1,0,0,&[],SOURCE);
            g.extend(seg(2,0,0,&[1],&body(1,parent_huff,false,if parent_huff{H1}else{A1})));
            let mut p=info(3);
            p.extend(seg(4,0,1,&[2],&body(1,!parent_huff,true,if parent_huff{ACHILD}else{HCHILD})));
            let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
            let s=d.stats();
            assert_eq!((s.dictionaries,s.huffman,s.arithmetic,s.single_refinement,s.huffman_single_refinement),(3,2,1,2,1));
            assert_eq!((s.new_symbols,s.pixels,s.packed_bytes,s.symbol_slots),(4,64,12,20));
            assert_eq!(s.peak_context_entries,4100);
            assert_eq!(ids(&d,2),[0,1,2]);assert_eq!(ids(&d,4),[0,1,2,3]);
            assert_eq!(d.symbol(4,3).unwrap().packed(),[192,48]);
            assert!(std::ptr::eq(d.symbol(2,1).unwrap(),d.symbol(4,1).unwrap()));
            assert!(!std::ptr::eq(d.symbol(2,2).unwrap(),d.symbol(4,3).unwrap()));
        }
    }
    #[test]
    fn huffman_executor_custom_table_graph(){
        let mut g=seg(1,0,0,&[],SOURCE);g.extend(seg(2,53,0,&[],CUSTOM_A));
        let mut p=info(3);let mut b=body(1,true,false,H1);b[1]|=0x80;
        p.extend(seg(4,0,1,&[2,1],&b));
        let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
        assert_eq!(d.stats().dictionaries,2);assert_eq!(d.symbol(4,2).unwrap().packed(),[192,48]);
        let mut p=info(3);p.extend(seg(4,0,1,&[1],&b));
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
    }
    #[test]
    fn huffman_executor_exact_cumulative_limits(){
        let (g,p)=fixture(1);let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
        let l=ExecutionLimits{dictionaries:3,new_symbols:3,symbol_slots:17,pixels:56,packed_bytes:12,
            work:d.stats().work_used,context_entries:1024,export_runs_per_dictionary:2,decisions_per_dictionary:14};
        assert_eq!(execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap(),d);
        for l in [ExecutionLimits{dictionaries:2,..l},ExecutionLimits{new_symbols:2,..l},
            ExecutionLimits{symbol_slots:16,..l},ExecutionLimits{pixels:55,..l},ExecutionLimits{packed_bytes:11,..l},
            ExecutionLimits{work:l.work-1,..l},ExecutionLimits{context_entries:1023,..l},
            ExecutionLimits{export_runs_per_dictionary:1,..l},ExecutionLimits{decisions_per_dictionary:13,..l}]{
            assert!(execute_pdf_dictionaries_single_reference(&g,&p,l).is_err());
        }
    }
    #[test]
    fn huffman_executor_retained_gb_coexistence(){
        let g=seg(1,0,0,&[],SOURCE);let mut p=info(2);
        p.extend(seg(3,0,1,&[],RETAINED_GB));p.extend(seg(4,0,1,&[1],&body(1,true,false,H1)));
        let l=ExecutionLimits{context_entries:2048,..ExecutionLimits::default()};
        let d=execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap();
        assert_eq!((d.stats().retained_contexts,d.stats().reused_contexts,d.stats().peak_context_entries),(1,0,2048));
        assert_eq!(ids(&d,4),[0,3,4]);assert_eq!(d.symbol(4,2).unwrap().packed(),[192,48]);
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits{context_entries:2047,..l}).is_err());
    }
    #[test]
    fn huffman_executor_unsupported_modes(){
        let g=seg(1,0,0,&[],SOURCE);
        let mut used=body(1,true,false,H1);used[0]|=1;
        let mut p=info(2);p.extend(seg(3,0,1,&[1],&used));
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
        let mut e=H1.to_vec();e[1]=8;
        let mut p=info(2);p.extend(seg(3,0,1,&[1],&body(1,true,false,&e)));
        assert!(matches!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()),Err(Error::Unsupported(_))));
    }
    #[test]
    fn huffman_executor_bad_dependencies_and_block_boundaries(){
        let g=seg(1,0,0,&[],SOURCE);
        for refs in [&[][..],&[9][..]]{
            let mut p=info(2);p.extend(seg(3,0,1,refs,&body(1,true,false,H1)));
            assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
        }
        for (i,v) in [(1,5),(3,64),(3,128),(6,0x90),(13,0)]{
            let mut e=H1.to_vec();e[i]=v;
            let mut p=info(2);p.extend(seg(3,0,1,&[1],&body(1,true,false,&e)));
            assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
        }
    }
}


#[cfg(test)]
mod retained_huffman_gr_tests{
    use super::*;
    const SOURCE:&[u8]=&[0,1,0,0,0,1,0,0,0,1,229,211,240,0,0,124,124,124,0,0,0,64];
    const H0:&[u8]=&[184,4,176,96,187,255,172,130,202,64,221,127,255,172,252,3];
    const H1:&[u8]=&[184,4,176,96,187,255,172,130,202,64,220,127,255,172,252,3];
    const A1:&[u8]=&[124,227,105,204,198,86,191,231,207,255,172];
    const CUSTOM_A:&[u8]=&[0x42,0,0,0,0,0,1,1,0x10,0x49,0x23,0x81,0x80];
    // Independently encoded fresh QM child with inherited GR: 3x2 [160,64],
    // source ID 0, offsets (+1,-1), 6 decisions. Both templates have the
    // same 3-byte block [159,255,172]; resetting GR changes the decisions.
    const CHILD:&[u8]=&[184,4,176,96,159,255,172,252,4];
    fn body(t:u8,flags:u8,child:bool,entropy:&[u8])->Vec<u8>{
        let mut v=vec![(t<<4)|flags,3];if t==0{v.extend([255;4]);}
        v.extend(if child{[0,0,0,4,0,0,0,1]}else{[0,0,0,3,0,0,0,2]});v.extend(entropy);v
    }
    fn parent(t:u8,flags:u8)->Vec<u8>{body(t,flags,false,if t==0{H0}else{H1})}
    fn plan_for(b:&[u8],imported:u32)->Plan{
        let (header,_)=parse(b).unwrap();let start=header.data_offset;
        Plan{header,imported,dictionaries:vec![],tables:Some([4,2,1,1].map(|n|
            SelectedTable{source:TableSource::Standard(n),table:huffman::standard::table(n).unwrap()})),
            context_source:None,encoded:start..b.len()}
    }
    fn src()->Bitmap{Bitmap::from_packed(7,6,&[0,124,124,124,0,0]).unwrap()}
    fn seg(id:u8,kind:u8,page:u8,refs:&[u8],data:&[u8])->Vec<u8>{
        let mut v=vec![0,0,0,id,kind,(refs.len()as u8)<<5];v.extend(refs);
        v.push(page);v.extend((data.len()as u32).to_be_bytes());v.extend(data);v
    }
    fn info(id:u8)->Vec<u8>{
        let mut d=vec![];for n in [20u32,20,300,300]{d.extend(n.to_be_bytes());}
        d.extend([0,0,0]);seg(id,48,1,&[],&d)
    }
    fn fixture(t:u8,flags:u8)->(Vec<u8>,Vec<u8>){
        let mut g=seg(1,0,0,&[],SOURCE);g.extend(seg(2,0,0,&[1],&parent(t,2)));
        let mut p=info(3);p.extend(seg(4,0,1,&[2],&body(t,flags,true,CHILD)));(g,p)
    }
    fn ids(d:&DictionarySet,n:u32)->Vec<usize>{d.dictionary(n).unwrap().iter().map(|s|s.index()).collect()}
    #[test]
    fn default_executor_retained_huffman_gr_exact_parity(){
        for t in [0,1]{for flags in [1,3]{
            let (g,p)=fixture(t,flags);let l=ExecutionLimits::default();
            let old=execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap();
            let exact=ExecutionLimits{work:old.stats().work_used,context_entries:old.stats().peak_context_entries,..l};
            assert_eq!(execute_pdf_dictionaries(&g,&p,exact).unwrap(),old);
            assert!(execute_pdf_dictionaries(&g,&p,ExecutionLimits{context_entries:exact.context_entries-1,..exact}).is_err());
        }}
    }
    // Same independent QM blocks as H0/H1, with the first symbol now
    // encoded by Table 17 (one refined instance, one fully clipped
    // direct instance). The second symbol uses Table 18 and references
    // the first output. Repack only the explicit Huffman syntax.
    fn mixed_parent(t:u8,flags:u8)->Vec<u8>{
        // DH=2, DW=3, AGGINST=2; initial DT=1, DT=1, FS=0,
        // ID=00 RI=1 DW=-4 DH=-4 DX=3 DY=1 RSIZE=3; byte align.
        let mut entropy=vec![184,8,0,30,56,234,140,187,255,172];
        // DS=2, ID=00 RI=0 (x=4 beyond 3px output), strip OOB;
        // DW=1 AGGINST=1 ID=01 DX=-1 DY=1 RSIZE=4; byte align.
        entropy.extend([208,96,178,144,if t==0{221}else{220},127,255,172,252,3]);
        body(t,flags,false,&entropy)
    }
    #[test]
    fn huffman_aggregation_dynamic_slot_gate_and_legacy_exact(){
        for t in [0,1]{
            let s=src();let l=ArithmeticLimits::default();
            let old=parent(t,2);let p=plan_for(&old,1);
            let reference=decode_huffman_single_refinement_with_context(&p,&old,&[&s],l,None).unwrap();
            let exact=ArithmeticLimits{work:reference.0.work_used,..l};
            assert_eq!(decode_huffman_refinement_impl(&p,&old,&[&s],exact,None,true,0).unwrap(),reference);
            let mixed=mixed_parent(t,2);let p=plan_for(&mixed,1);
            let d=decode_huffman_refinement_impl(&p,&mixed,&[&s],l,None,true,1).unwrap();
            assert_eq!(d.0.new_symbols,reference.0.new_symbols);
            assert_eq!(d.0.aggregate_reference_slots,1);
            assert!(matches!(decode_huffman_refinement_impl(&p,&mixed,&[&s],l,None,true,0),
                Err(Error::Limit("JBIG2 dictionary symbol slots"))));
            assert_eq!(decode_huffman_refinement_impl(&p,&mixed,&[&s],ArithmeticLimits{work:d.0.work_used,..l},
                None,true,1).unwrap(),d);
        }
    }
    #[test]
    fn huffman_mixed_aggregation_single_and_retained_context_parity(){
        for t in [0,1]{
            let s=src();let l=ArithmeticLimits::default();
            let old=parent(t,2);
            let (reference,reference_snapshot)=decode_huffman_single_refinement_with_context(
                &plan_for(&old,1),&old,&[&s],l,None).unwrap();
            let mixed=mixed_parent(t,2);
            let p=plan_for(&mixed,1);
            let (d,snapshot)=decode_huffman_aggregation_with_context(&p,&mixed,&[&s],l,None).unwrap();
            assert_eq!(d.new_symbols,reference.new_symbols);
            assert_eq!(d.exported_indices,[0,1,2]);
            assert_eq!((d.classes,d.blocks,d.entropy_bytes,d.pixel_decisions,d.packed_bytes),(1,2,7,14,4));
            assert_eq!(snapshot,reference_snapshot);
            assert_eq!(d.aggregate_reference_slots,1);
            assert_eq!(reference.aggregate_reference_slots,0);
            let exact=ArithmeticLimits{pixels:14,packed_bytes:4,work:d.work_used,decisions:14,export_runs:2};
            assert_eq!(decode_huffman_aggregation_with_context(&p,&mixed,&[&s],exact,None).unwrap().0,d);
            for cap in [ArithmeticLimits{pixels:13,..exact},ArithmeticLimits{packed_bytes:3,..exact},
                ArithmeticLimits{work:exact.work-1,..exact},ArithmeticLimits{decisions:13,..exact},
                ArithmeticLimits{export_runs:1,..exact}]{
                assert!(decode_huffman_aggregation_with_context(&p,&mixed,&[&s],cap,None).is_err());
            }
            assert!(matches!(decode_huffman_single_refinement_with_context(&p,&mixed,&[&s],l,None),Err(Error::Unsupported(_))));
            let imports=[&s,&d.new_symbols[0],&d.new_symbols[1]];
            let b=body(t,3,true,CHILD);
            let (child,after)=decode_huffman_aggregation_with_context(&plan_for(&b,3),&b,&imports,l,snapshot.as_ref()).unwrap();
            let (old_child,old_after)=decode_huffman_single_refinement_with_context(
                &plan_for(&b,3),&b,&imports,l,reference_snapshot.as_ref()).unwrap();
            assert_eq!(child,old_child);
            assert_eq!(after,old_after);
            assert_eq!(child.new_symbols[0].packed(),[160,64]);
            let b=body(t,0,true,CHILD);
            let reset=decode_huffman_aggregation_with_context(&plan_for(&b,3),&b,&imports,l,None).unwrap().0;
            assert_ne!(reset.new_symbols,child.new_symbols);
        }
    }
    #[test]
    fn huffman_mixed_aggregation_global_page_snapshot_and_legacy_parity(){
        for t in [0,1]{
            let l=ExecutionLimits::default();
            let (old_g,old_p)=fixture(t,1);
            let old=execute_pdf_dictionaries_single_reference(&old_g,&old_p,l).unwrap();
            assert_eq!(execute_pdf_dictionaries(&old_g,&old_p,l).unwrap(),old);
            let opt=execute_pdf_dictionaries_with_huffman_aggregation(&old_g,&old_p,l).unwrap();
            assert_eq!(old,opt);
            for (seg,count) in [(1,1),(2,3),(4,4)]{
                assert_eq!(ids(&old,seg),ids(&opt,seg));
                for i in 0..count{assert_eq!(old.symbol(seg,i),opt.symbol(seg,i));}
            }
            let mut g=seg(1,0,0,&[],SOURCE);
            g.extend(seg(2,0,0,&[1],&mixed_parent(t,2)));
            let mut p=info(3);
            p.extend(seg(4,0,1,&[2],&body(t,3,true,CHILD)));
            p.extend(seg(5,0,1,&[2],&body(t,1,true,CHILD)));
            let d=execute_pdf_dictionaries_with_huffman_aggregation(&g,&p,l).unwrap();
            assert_eq!(execute_pdf_dictionaries(&g,&p,l).unwrap(),d);
            assert_eq!(ids(&d,4),[0,1,2,3]);assert_eq!(ids(&d,5),[0,1,2,4]);
            assert_eq!(d.symbol(4,3).unwrap().packed(),[160,64]);
            assert_eq!(d.symbol(4,3),d.symbol(5,3));
            assert!(std::ptr::eq(d.symbol(2,1).unwrap(),d.symbol(4,1).unwrap()));
            assert!(!std::ptr::eq(d.symbol(4,3).unwrap(),d.symbol(5,3).unwrap()));
            let stats=d.stats();
            assert_eq!((stats.dictionaries,stats.huffman_aggregation,stats.retained_gr_contexts,stats.reused_gr_contexts),(4,1,2,2));
            assert_eq!((stats.single_refinement,stats.huffman_single_refinement),(2,2));
            assert_eq!(stats.peak_context_entries,3*if t==0{8192}else{1024});
            let exact=ExecutionLimits{work:stats.work_used,symbol_slots:stats.symbol_slots,
                pixels:stats.pixels,packed_bytes:stats.packed_bytes,context_entries:stats.peak_context_entries,
                decisions_per_dictionary:14,..l};
            assert_eq!(execute_pdf_dictionaries_with_huffman_aggregation(&g,&p,exact).unwrap(),d);
            for cap in [ExecutionLimits{work:exact.work-1,..exact},
                ExecutionLimits{symbol_slots:exact.symbol_slots-1,..exact},
                ExecutionLimits{context_entries:exact.context_entries-1,..exact},
                ExecutionLimits{decisions_per_dictionary:13,..exact}]{
                assert!(execute_pdf_dictionaries_with_huffman_aggregation(&g,&p,cap).is_err());
            }
            assert!(matches!(execute_pdf_dictionaries_with_aggregation(&g,&p,l),Err(Error::Unsupported(_))));
            assert!(matches!(execute_pdf_dictionaries_single_reference(&g,&p,l),Err(Error::Unsupported(_))));
            // The last referred dictionary is not the retained parent.
            let mut bad=info(3);bad.extend(seg(4,0,1,&[2,1],&body(t,1,true,CHILD)));
            assert!(execute_pdf_dictionaries_with_huffman_aggregation(&g,&bad,l).is_err());
        }
    }

    #[test]
    fn reclaim_huffman_gr_both_templates_and_branch(){
        for t in [0,1]{
            let l=ExecutionLimits::default();
            let mut g=seg(1,0,0,&[],SOURCE);g.extend(seg(2,0,0,&[1],&mixed_parent(t,2)));
            let mut p=info(3);
            p.extend(seg(4,0,1,&[2],&body(t,3,true,CHILD)));
            p.extend(seg(5,0,1,&[2],&body(t,1,true,CHILD)));
            let old=execute_pdf_dictionaries(&g,&p,l).unwrap();
            let d=execute_pdf_dictionaries_reclaimed_contexts(&g,&p,l).unwrap();
            assert_reclaimed_context_parity(&old,&d);
            let bank=if t==0{8192}else{1024};
            assert_eq!((old.stats().peak_context_entries,d.stats().peak_context_entries),(3*bank,2*bank));
            assert_eq!((d.stats().released_contexts,d.stats().released_context_entries),(2,2*bank));
            assert_eq!(d.symbol(4,3).unwrap().packed(),[160,64]);
            let exact=ExecutionLimits{context_entries:2*bank,work:d.stats().work_used,..l};
            assert_eq!(execute_pdf_dictionaries_reclaimed_contexts(&g,&p,exact).unwrap(),d);
            assert!(execute_pdf_dictionaries_reclaimed_contexts(&g,&p,ExecutionLimits{context_entries:2*bank-1,..exact}).is_err());
            assert!(execute_pdf_dictionaries_reclaimed_contexts(&g,&p,ExecutionLimits{work:exact.work-1,..exact}).is_err());
        }
    }
    #[test]
    fn retained_huffman_gr_both_templates(){
        for t in [0,1]{
            let s=src();let l=ArithmeticLimits::default();let p=parent(t,2);
            let (d,snap)=decode_huffman_single_refinement_with_context(&plan_for(&p,1),&p,&[&s],l,None).unwrap();
            let snap=snap.unwrap();assert_eq!(snap.entries(),if t==0{8192}else{1024});
            assert_eq!(snap.nonzero_entries(),if t==0{12}else{11});
            let imports=[&s,&d.new_symbols[0],&d.new_symbols[1]];
            let b=body(t,3,true,CHILD);
            let (child,after)=decode_huffman_single_refinement_with_context(&plan_for(&b,3),&b,&imports,l,Some(&snap)).unwrap();
            let after=after.unwrap();assert_eq!(child.new_symbols[0].packed(),[160,64]);
            assert_eq!(child.exported_indices,[0,1,2,3]);
            assert_eq!((child.blocks,child.entropy_bytes,child.pixel_decisions),(1,3,6));
            assert_eq!(snap.bank.entries().iter().zip(after.bank.entries()).filter(|(a,b)|a!=b).count(),4);
            assert_eq!(snap.nonzero_entries(),after.nonzero_entries());
            let b=body(t,0,true,CHILD);
            let reset=decode_huffman_single_refinement(&plan_for(&b,3),&b,&imports,l).unwrap();
            assert_ne!(reset.new_symbols,child.new_symbols);
        }
    }
    #[test]
    fn retained_huffman_gr_global_to_page(){
        for t in [0,1]{
            let (g,p)=fixture(t,1);let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
            let s=d.stats();assert_eq!((s.dictionaries,s.huffman_single_refinement,s.retained_gr_contexts,s.reused_gr_contexts),(3,2,1,1));
            assert_eq!((s.retained_contexts,s.reused_contexts),(1,1));
            assert_eq!((s.new_symbols,s.pixels,s.packed_bytes,s.symbol_slots),(4,62,12,20));
            assert_eq!(ids(&d,4),[0,1,2,3]);assert_eq!(d.symbol(4,3).unwrap().packed(),[160,64]);
            assert!(std::ptr::eq(d.symbol(2,1).unwrap(),d.symbol(4,1).unwrap()));
            assert!(!std::ptr::eq(d.symbol(2,1).unwrap(),d.symbol(4,3).unwrap()));
        }
    }
    #[test]
    fn retained_huffman_gr_immutable_branch(){
        let (g,mut p)=fixture(1,3);p.extend(seg(5,0,1,&[2],&body(1,1,true,CHILD)));
        let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
        assert_eq!(ids(&d,4),[0,1,2,3]);assert_eq!(ids(&d,5),[0,1,2,4]);
        assert_eq!(d.symbol(4,3),d.symbol(5,3));
        assert_eq!((d.stats().retained_gr_contexts,d.stats().reused_gr_contexts,d.stats().peak_context_entries),(2,2,3072));
    }
    #[test]
    fn retained_huffman_gr_exact_budgets(){
        let (g,p)=fixture(1,1);
        let l=ExecutionLimits{context_entries:2048,..ExecutionLimits::default()};
        let d=execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap();assert_eq!(d.stats().peak_context_entries,2048);
        let l=ExecutionLimits{work:d.stats().work_used,..l};
        assert_eq!(execute_pdf_dictionaries_single_reference(&g,&p,l).unwrap(),d);
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits{context_entries:2047,..l}).is_err());
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits{work:l.work-1,..l}).is_err());
    }
    #[test]
    fn retained_huffman_gr_last_dictionary_not_table(){
        let (mut g,_)=fixture(1,1);g.extend(seg(3,53,0,&[],CUSTOM_A));
        let mut p=info(4);let mut child=body(1,1,true,CHILD);child[1]|=0x80;
        p.extend(seg(5,0,1,&[2,3],&child));
        let d=execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).unwrap();
        assert_eq!(ids(&d,5),[0,1,2,3]);assert_eq!(d.symbol(5,3).unwrap().packed(),[160,64]);
        // The last *dictionary* is now the direct source, not the snapshot.
        let mut p=info(4);p.extend(seg(5,0,1,&[2,1,3],&child));
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
    }
    #[test]
    fn retained_huffman_gr_modes_and_geometry(){
        let s=src();let l=ArithmeticLimits::default();let p=parent(0,2);
        let (d,snap)=decode_huffman_single_refinement_with_context(&plan_for(&p,1),&p,&[&s],l,None).unwrap();
        let snap=snap.unwrap();let imports=[&s,&d.new_symbols[0],&d.new_symbols[1]];
        let b=body(0,1,true,CHILD);let mut plan=plan_for(&b,3);
        assert!(decode_huffman_single_refinement_with_context(&plan,&b,&imports,l,None).is_err());
        plan.header.context_used=false;
        assert!(decode_huffman_single_refinement_with_context(&plan,&b,&imports,l,Some(&snap)).is_err());
        plan.header.context_used=true;plan.header.refinement_adaptive[1].x=0;
        assert!(decode_huffman_single_refinement_with_context(&plan,&b,&imports,l,Some(&snap)).is_err());
        let b=body(1,1,true,CHILD);
        assert!(decode_huffman_single_refinement_with_context(&plan_for(&b,3),&b,&imports,l,Some(&snap)).is_err());
        let mut a=vec![26,2,2,255,0,0,0,3,0,0,0,2];a.extend(A1);
        let mut g=seg(1,0,0,&[],SOURCE);g.extend(seg(2,0,0,&[1],&a));
        let mut p=info(3);p.extend(seg(4,0,1,&[2],&b));
        assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
    }
    #[test]
    fn retained_huffman_gr_empty_and_missing_snapshot(){
        let s=src();let l=ArithmeticLimits::default();
        // No new symbols, export one imported symbol, retain zero GR bank.
        let b=[18,3,0,0,0,1,0,0,0,0,0,64];
        let (d,snap)=decode_huffman_single_refinement_with_context(&plan_for(&b,1),&b,&[&s],l,None).unwrap();
        assert!(d.new_symbols.is_empty());assert_eq!(d.exported_indices,[0]);
        assert_eq!(snap.unwrap().nonzero_entries(),0);
        let b=parent(1,0);let (d,snap)=decode_huffman_single_refinement_with_context(&plan_for(&b,1),&b,&[&s],l,None).unwrap();
        assert!(snap.is_none());assert_eq!(decode_huffman_single_refinement(&plan_for(&b,1),&b,&[&s],l).unwrap(),d);
        let mut g=seg(1,0,0,&[],SOURCE);g.extend(seg(2,0,0,&[1],&b));
        let (_,p)=fixture(1,1);assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
    }
}

#[cfg(test)]
mod aggregation_candidate_tests{
    use super::*;
    fn fixture()->(Vec<u8>,Plan,Bitmap){
        let mut b=vec![24,2,2,255,0,0,0,3,0,0,0,2];
        b.extend([124,227,105,204,198,86,191,231,207,255,172]);
        let (header,_)=parse(&b).unwrap();let start=header.data_offset;
        let p=Plan{header,imported:1,dictionaries:vec![],tables:None,context_source:None,encoded:start..b.len()};
        (b,p,Bitmap::from_packed(7,6,&[0,124,124,124,0,0]).unwrap())
    }
    #[test]
    fn aggregation_entry_preserves_single_reference(){
        let (b,p,s)=fixture();
        let expected=decode_arithmetic_single_refinement(&p,&b,&[&s],ArithmeticLimits::default()).unwrap();
        let actual=decode_arithmetic_aggregation(&p,&b,&[&s],ArithmeticLimits::default()).unwrap();
        assert_eq!(actual.new_symbols,expected.new_symbols);
        assert_eq!(actual.exported_indices,expected.exported_indices);
        assert_eq!(actual.work_used,expected.work_used+9*512);
    }
    #[test]
    fn aggregation_entry_context_work_preflight(){
        let (b,p,s)=fixture();
        let expected=decode_arithmetic_single_refinement(&p,&b,&[&s],ArithmeticLimits::default()).unwrap();
        let limits=ArithmeticLimits{work:expected.work_used,..ArithmeticLimits::default()};
        assert!(decode_arithmetic_aggregation(&p,&b,&[&s],limits).is_err());
    }
}

#[cfg(test)]
mod aggregation_multi_vectors{
    use super::*;
    // Independent Annex E encoder: imported black 1x1, two direct
    // instances at x=0 and x=1 in new 2x1 symbol. Fixed one-bit IAID.
    const GOOD:&[u8]=&[147,166,142,126,109,112,255,172];
    const FORWARD:&[u8]=&[147,166,137,255,172];
    fn run(entropy:&[u8],limits:ArithmeticLimits,t:u8)->Result<ArithmeticDictionary>{
        let mut b=vec![8|(t<<4),2,2,255];
        if t==0{b.extend([255;4]);}
        b.extend([0,0,0,2,0,0,0,1]);b.extend(entropy);
        let (header,_)=parse(&b)?;let start=header.data_offset;
        let p=Plan{header,imported:1,dictionaries:vec![],tables:None,context_source:None,encoded:start..b.len()};
        let src=Bitmap::from_packed(1,1,&[128])?;
        decode_arithmetic_aggregation(&p,&b,&[&src],limits)
    }
    #[test]
    fn aggregation_two_direct_instances_pixels(){
        for t in [0,1]{
            let d=run(GOOD,ArithmeticLimits::default(),t).unwrap();
            assert_eq!(d.exported_indices,[0,1]);
            assert_eq!((d.new_symbols[0].width(),d.new_symbols[0].height()),(2,1));
            assert_eq!(d.new_symbols[0].packed(),[192]);
        }
    }
    #[test]
    fn aggregation_forward_reference_rejected(){
        assert!(matches!(run(FORWARD,ArithmeticLimits::default(),1),
            Err(Error::Invalid("JBIG2 IAID symbol index"))));
    }
    #[test]
    fn aggregation_exact_decisions_and_work(){
        let d=run(GOOD,ArithmeticLimits::default(),1).unwrap();
        let l=ArithmeticLimits{decisions:54,work:d.work_used,..ArithmeticLimits::default()};
        assert_eq!(run(GOOD,l,1).unwrap(),d);
        assert!(run(GOOD,ArithmeticLimits{decisions:53,..l},1).is_err());
        assert!(run(GOOD,ArithmeticLimits{work:d.work_used-1,..l},1).is_err());
    }
}

#[cfg(test)]
mod aggregation_mixed_vectors{
    use super::*;
    // Offline Annex E vector: single black refinement followed by
    // direct prior symbol and refined white. Shared displacement/GR/IAID.
    const MIXED:&[u8]=&[149,20,56,203,95,170,125,186,152,236,191,255,172];
    fn run(t:u8,limits:ArithmeticLimits)->Result<ArithmeticDictionary>{
        let mut b=vec![8|(t<<4),2,2,255];
        if t==0{b.extend([255;4]);}
        b.extend([0,0,0,3,0,0,0,2]);b.extend(MIXED);
        let (header,_)=parse(&b)?;let start=header.data_offset;
        let p=Plan{header,imported:1,dictionaries:vec![],tables:None,context_source:None,encoded:start..b.len()};
        let src=Bitmap::from_packed(1,1,&[128])?;
        decode_arithmetic_aggregation(&p,&b,&[&src],limits)
    }
    #[test]
    fn mixed_single_and_multi_refined_pixels(){
        for t in [0,1]{
            let d=run(t,ArithmeticLimits::default()).unwrap();
            assert_eq!(d.exported_indices,[0,1,2]);
            assert_eq!(d.new_symbols.len(),2);
            assert_eq!(d.new_symbols[0].packed(),[128]);
            assert_eq!((d.new_symbols[1].width(),d.new_symbols[1].height()),(2,1));
            assert_eq!(d.new_symbols[1].packed(),[128]);
        }
    }
    #[test]
    fn mixed_aggregation_parent_child_entropy(){
        // Independently encoded child syntax: DH=DW=AI=1, ID=0 (2 bits),
        // RDX=RDY=0, GR=1, width OOB, export runs 0/4.
        // Only the centre GR context inherits (state 6, MPS 1).
        const CHILD:&[u8]=&[149,20,57,186,107,255,172];
        for t in [0,1]{
            let mut b=vec![8|(t<<4)|2,2,2,255];
            if t==0{b.extend([255;4]);}
            b.extend([0,0,0,3,0,0,0,2]);b.extend(MIXED);
            let (header,_)=parse(&b).unwrap();let start=header.data_offset;
            let p=Plan{header,imported:1,dictionaries:vec![],tables:None,context_source:None,encoded:start..b.len()};
            let src=Bitmap::from_packed(1,1,&[128]).unwrap();
            let l=ArithmeticLimits::default();
            let (parent,snap)=decode_arithmetic_aggregation_with_context(&p,&b,&[&src],l,None).unwrap();
            let snap=snap.unwrap();
            let centre=if t==0{16}else{8};
            assert_eq!(snap.bank.entries()[centre],super::super::qm::Context::new(6,1).unwrap());
            let mut child=vec![8|(t<<4)|3,2,2,255];
            if t==0{child.extend([255;4]);}
            child.extend([0,0,0,4,0,0,0,1]);child.extend(CHILD);
            let (header,_)=parse(&child).unwrap();let start=header.data_offset;
            let mut p=Plan{header,imported:3,dictionaries:vec![],tables:None,context_source:None,encoded:start..child.len()};
            let imports=[&src,&parent.new_symbols[0],&parent.new_symbols[1]];
            let exact=ArithmeticLimits{decisions:38,..l};
            let (d,after)=decode_arithmetic_aggregation_with_context(&p,&child,&imports,exact,Some(&snap)).unwrap();
            assert_eq!(d.exported_indices,[0,1,2,3]);
            assert_eq!(d.new_symbols[0].packed(),[128]);
            assert_ne!(after.unwrap(),snap);
            assert!(decode_arithmetic_aggregation_with_context(&p,&child,&imports,ArithmeticLimits{decisions:37,..l},Some(&snap)).is_err());
            p.header.context_used=false;
            if let Ok((reset,_))=decode_arithmetic_aggregation_with_context(&p,&child,&imports,exact,None){
                assert!(reset.new_symbols!=d.new_symbols||reset.exported_indices!=d.exported_indices);
            }
        }
    }
    #[test]
    fn mixed_aggregation_retains_gr(){
        for t in [0,1]{
            let mut b=vec![8|(t<<4)|2,2,2,255];
            if t==0{b.extend([255;4]);}
            b.extend([0,0,0,3,0,0,0,2]);b.extend(MIXED);
            let (header,_)=parse(&b).unwrap();let start=header.data_offset;
            let p=Plan{header,imported:1,dictionaries:vec![],tables:None,context_source:None,encoded:start..b.len()};
            let src=Bitmap::from_packed(1,1,&[128]).unwrap();
            let (d,snap)=decode_arithmetic_aggregation_with_context(&p,&b,&[&src],ArithmeticLimits::default(),None).unwrap();
            assert_eq!(d,run(t,ArithmeticLimits::default()).unwrap());
            let snap=snap.unwrap();
            assert_eq!(snap.entries(),if t==0{8192}else{1024});
            assert!(snap.nonzero_entries()>0);
            assert!(decode_arithmetic_aggregation(&p,&b,&[&src],ArithmeticLimits::default()).is_err());
        }
    }
    #[test]
    fn mixed_single_and_multi_exact_budget(){
        for t in [0,1]{
            let d=run(t,ArithmeticLimits::default()).unwrap();
            let l=ArithmeticLimits{decisions:92,work:d.work_used,..ArithmeticLimits::default()};
            assert_eq!(run(t,l).unwrap(),d);
            assert!(run(t,ArithmeticLimits{decisions:91,..l}).is_err());
            assert!(run(t,ArithmeticLimits{work:d.work_used-1,..l}).is_err());
        }
    }
    #[test]
    fn mixed_aggregation_peak_packed_budget(){
        for t in [0,1]{
            let l=ArithmeticLimits{packed_bytes:3,..ArithmeticLimits::default()};
            let d=run(t,l).unwrap();
            assert_eq!(d.packed_bytes,2);
            assert!(matches!(run(t,ArithmeticLimits{packed_bytes:2,..l}),
                Err(Error::Limit("JBIG2 text temporary packed budget"))));
            assert!(run(t,ArithmeticLimits{packed_bytes:1,..l}).is_err());
        }
    }

}

#[cfg(test)]
mod aggregation_displaced_vectors{
    use super::*;
    // Independent Annex E vectors with raw horizontal displacement +1/-1.
    // Both refinement calls sample the same noncentral reference context:
    // right neighbour for +1 and left neighbour for -1, on both templates.
    const POS:&[u8]=&[149,19,226,202,95,170,125,186,152,236,191,255,172];
    const NEG:&[u8]=&[149,17,50,194,95,170,125,186,152,236,191,255,172];
    fn run(t:u8,data:&[u8],limits:ArithmeticLimits)->Result<ArithmeticDictionary>{
        let mut b=vec![8|(t<<4),2,2,255];
        if t==0{b.extend([255;4]);}
        b.extend([0,0,0,3,0,0,0,2]);b.extend(data);
        let (header,_)=parse(&b)?;let start=header.data_offset;
        let p=Plan{header,imported:1,dictionaries:vec![],tables:None,context_source:None,encoded:start..b.len()};
        let src=Bitmap::from_packed(1,1,&[128])?;
        decode_arithmetic_aggregation(&p,&b,&[&src],limits)
    }
    #[test]
    fn mixed_positive_negative_displacement(){
        for t in [0,1]{
            for data in [POS,NEG]{
                let d=run(t,data,ArithmeticLimits::default()).unwrap();
                assert_eq!(d.exported_indices,[0,1,2]);
                assert_eq!(d.new_symbols.len(),2);
                assert_eq!(d.new_symbols[0].packed(),[128]);
                assert_eq!(d.new_symbols[1].packed(),[128]);
                assert_eq!((d.new_symbols[1].width(),d.new_symbols[1].height()),(2,1));
            }
        }
    }
    // Vertical and diagonal displacements, same GR context across each
    // pair of refinement calls. T1 omits upper diagonal neighbours.
    const AXES:[&[u8];4]=[&[149,20,51,107,79,170,125,186,152,236,191,255,172],&[149,20,8,106,207,170,125,186,152,236,191,255,172],&[149,19,221,106,79,170,125,186,152,236,191,255,172],&[149,17,2,97,207,170,125,186,152,236,191,255,172]];
    #[test]
    fn mixed_vertical_diagonal_displacement(){
        for t in [0,1]{
            for data in AXES{
                let d=run(t,data,ArithmeticLimits{decisions:92,..ArithmeticLimits::default()}).unwrap();
                assert_eq!(d.exported_indices,[0,1,2]);
                assert_eq!(d.new_symbols.len(),2);
                assert_eq!(d.new_symbols[0].packed(),[128]);
                assert_eq!(d.new_symbols[1].packed(),[128]);
                assert_eq!((d.new_symbols[1].width(),d.new_symbols[1].height()),(2,1));
                assert!(run(t,data,ArithmeticLimits{decisions:91,..ArithmeticLimits::default()}).is_err());
            }
        }
    }
    #[test]
    fn mixed_displaced_exact_decision_budget(){
        for t in [0,1]{
            for data in [POS,NEG]{
                let d=run(t,data,ArithmeticLimits{decisions:92,..ArithmeticLimits::default()}).unwrap();
                assert!(run(t,data,ArithmeticLimits{decisions:91,..ArithmeticLimits::default()}).is_err());
                assert!(run(t,data,ArithmeticLimits{work:d.work_used-1,..ArithmeticLimits::default()}).is_err());
            }
        }
    }
}

#[cfg(test)]
mod aggregation_executor_tests{
    use super::*;
    // Independent raw Huffman 1x1 source: B.4 DH=1, B.2 DW=1/OOB,
    // B.1 BMSIZE=0, black pixel, B.1 EX=0,1.
    const SOURCE:&[u8]=&[0,1,0,0,0,1,0,0,0,1,95,128,128,0,64];
    const MIXED:&[u8]=&[149,20,56,203,95,170,125,186,152,236,191,255,172];
    fn seg(id:u8,kind:u8,page:u8,refs:&[u8],data:&[u8])->Vec<u8>{
        let mut v=vec![0,0,0,id,kind,(refs.len()as u8)<<5];
        v.extend(refs);v.push(page);v.extend((data.len()as u32).to_be_bytes());v.extend(data);v
    }
    fn fixture(t:u8)->(Vec<u8>,Vec<u8>){
        let g=seg(1,0,0,&[],SOURCE);
        let mut info=Vec::new();
        for n in [20u32,20,300,300]{info.extend(n.to_be_bytes());}
        info.extend([0,0,0]);let mut p=seg(2,48,1,&[],&info);
        let mut body=vec![8|(t<<4),2,2,255];if t==0{body.extend([255;4]);}
        body.extend([0,0,0,3,0,0,0,2]);body.extend(MIXED);
        p.extend(seg(3,0,1,&[1],&body));(g,p)
    }
    #[test]
    fn aggregation_default_dispatch_and_legacy_opt_out(){
        for t in [0,1]{
            for (g,p) in [fixture(t),retained_fixture(t)]{
                let l=ExecutionLimits::default();
                let d=execute_pdf_dictionaries(&g,&p,l).unwrap();
                assert_eq!(d,execute_pdf_dictionaries_with_aggregation(&g,&p,l).unwrap());
                assert!(matches!(execute_pdf_dictionaries_single_reference(&g,&p,l),Err(Error::Unsupported(_))));
                let tight=ExecutionLimits{context_entries:d.stats().peak_context_entries-1,..l};
                assert!(execute_pdf_dictionaries(&g,&p,tight).is_err());
            }
        }
    }
    #[test]
    fn aggregation_executor_pixels_and_identity(){
        for t in [0,1]{
            let (g,p)=fixture(t);
            assert!(matches!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()),Err(Error::Unsupported(_))));
            let d=execute_pdf_dictionaries_with_aggregation(&g,&p,ExecutionLimits::default()).unwrap();
            let s=d.stats();
            assert_eq!((s.dictionaries,s.arithmetic_aggregation,s.single_refinement),(2,1,0));
            assert_eq!((s.new_symbols,s.pixels,s.packed_bytes,s.symbol_slots),(3,4,3,11));
            assert_eq!(s.peak_context_entries,if t==0{8192+4+3072+4608}else{1024+4+3072+4608});
            assert_eq!(d.symbol(1,0).unwrap().packed(),[128]);
            assert!(std::ptr::eq(d.symbol(1,0).unwrap(),d.symbol(3,0).unwrap()));
            assert_eq!(d.symbol(3,1).unwrap().packed(),[128]);
            let last=d.symbol(3,2).unwrap();
            assert_eq!((last.width(),last.height()),(2,1));assert_eq!(last.packed(),[128]);
        }
    }
    fn retained_fixture(t:u8)->(Vec<u8>,Vec<u8>){
        let mut g=seg(1,0,0,&[],SOURCE);
        let mut parent=vec![8|(t<<4)|2,2,2,255];if t==0{parent.extend([255;4]);}
        parent.extend([0,0,0,3,0,0,0,2]);parent.extend(MIXED);
        g.extend(seg(2,0,0,&[1],&parent));
        let mut info=Vec::new();for n in [20u32,20,300,300]{info.extend(n.to_be_bytes());}
        info.extend([0,0,0]);let mut p=seg(3,48,1,&[],&info);
        let mut child=vec![8|(t<<4)|3,2,2,255];if t==0{child.extend([255;4]);}
        child.extend([0,0,0,4,0,0,0,1]);child.extend([149,20,57,186,107,255,172]);
        p.extend(seg(4,0,1,&[2],&child));(g,p)
    }
    #[test]
    fn aggregation_executor_retained_global_to_page(){
        for t in [0,1]{
            let (g,p)=retained_fixture(t);
            let d=execute_pdf_dictionaries_with_aggregation(&g,&p,ExecutionLimits::default()).unwrap();
            let s=d.stats();
            assert_eq!((s.dictionaries,s.arithmetic_aggregation,s.single_refinement),(3,2,0));
            assert_eq!((s.retained_contexts,s.reused_contexts,s.retained_gr_contexts,s.reused_gr_contexts),(2,1,2,1));
            assert_eq!((s.new_symbols,s.pixels,s.packed_bytes,s.symbol_slots),(4,5,4,25));
            assert_eq!(s.peak_context_entries,if t==0{2*8192+4+3072+4608}else{2*1024+4+3072+4608});
            assert_eq!(d.symbol(4,3).unwrap().packed(),[128]);
            for i in 0..3{assert!(std::ptr::eq(d.symbol(2,i).unwrap(),d.symbol(4,i).unwrap()));}
            assert!(execute_pdf_dictionaries_single_reference(&g,&p,ExecutionLimits::default()).is_err());
        }
    }
    #[test]
    fn aggregation_executor_rejects_invalid_context_provenance(){
        let l=ExecutionLimits::default();
        let (g,p)=retained_fixture(1);
        // The parent is the second global segment; clear its retain flag.
        let mut no_retain=g.clone();
        let parent_start=seg(1,0,0,&[],SOURCE).len()+12;
        no_retain[parent_start]&=!2;
        assert!(matches!(execute_pdf_dictionaries_with_aggregation(&no_retain,&p,l),
            Err(Error::Invalid("JBIG2 dictionary context not retained"))));
        // Last dictionary, not any earlier retained dictionary, is the source.
        let info_len=seg(3,48,1,&[],&[0;19]).len();
        let child_body=&p[info_len+12..];
        let mut wrong_last=p[..info_len].to_vec();
        wrong_last.extend(seg(4,0,1,&[2,1],child_body));
        assert!(matches!(execute_pdf_dictionaries_with_aggregation(&g,&wrong_last,l),
            Err(Error::Invalid("JBIG2 dictionary context not retained"))));
        let (_,other_template)=retained_fixture(0);
        assert!(matches!(execute_pdf_dictionaries_with_aggregation(&g,&other_template,l),
            Err(Error::Invalid("JBIG2 dictionary context parameters mismatch"))));
        let mut changed_at=p.clone();
        changed_at[info_len+12+2]=1;
        assert!(matches!(execute_pdf_dictionaries_with_aggregation(&g,&changed_at,l),
            Err(Error::Invalid("JBIG2 dictionary context parameters mismatch"))));
    }
    #[test]
    fn aggregation_executor_retained_fanout(){
        for t in [0,1]{
            let (g,mut p)=retained_fixture(t);
            let info_len=seg(3,48,1,&[],&[0;19]).len();
            let body=p[info_len+12..].to_vec();
            p.extend(seg(5,0,1,&[2],&body));
            let d=execute_pdf_dictionaries_with_aggregation(&g,&p,ExecutionLimits::default()).unwrap();
            let s=d.stats();
            assert_eq!((s.dictionaries,s.arithmetic_aggregation,s.retained_gr_contexts,s.reused_gr_contexts),(4,3,3,2));
            assert_eq!(d.symbol(4,3).unwrap().packed(),[128]);
            assert_eq!(d.symbol(4,3),d.symbol(5,3));
            assert!(!std::ptr::eq(d.symbol(4,3).unwrap(),d.symbol(5,3).unwrap()));
            assert_eq!(s.peak_context_entries,if t==0{3*8192+4+3072+4608}else{3*1024+4+3072+4608});
            let exact=ExecutionLimits{context_entries:s.peak_context_entries,work:s.work_used,..ExecutionLimits::default()};
            assert_eq!(execute_pdf_dictionaries_with_aggregation(&g,&p,exact).unwrap(),d);
            assert!(execute_pdf_dictionaries_with_aggregation(&g,&p,ExecutionLimits{context_entries:exact.context_entries-1,..exact}).is_err());
        }
    }
    #[test]
    fn aggregation_executor_retained_exact_limits(){
        let (g,p)=retained_fixture(1);
        let d=execute_pdf_dictionaries_with_aggregation(&g,&p,ExecutionLimits::default()).unwrap();
        let l=ExecutionLimits{dictionaries:3,new_symbols:4,symbol_slots:25,pixels:5,packed_bytes:4,
            work:d.stats().work_used,export_runs_per_dictionary:2,decisions_per_dictionary:92,context_entries:9732};
        assert_eq!(execute_pdf_dictionaries_with_aggregation(&g,&p,l).unwrap(),d);
        for l in [ExecutionLimits{symbol_slots:24,..l},ExecutionLimits{context_entries:9731,..l},
            ExecutionLimits{work:l.work-1,..l},ExecutionLimits{packed_bytes:3,..l}]{
            assert!(execute_pdf_dictionaries_with_aggregation(&g,&p,l).is_err());
        }
    }
    #[test]
    fn aggregation_executor_exact_limits(){
        let (g,p)=fixture(1);
        let d=execute_pdf_dictionaries_with_aggregation(&g,&p,ExecutionLimits::default()).unwrap();
        let l=ExecutionLimits{dictionaries:2,new_symbols:3,symbol_slots:11,pixels:4,packed_bytes:4,
            work:d.stats().work_used,export_runs_per_dictionary:2,decisions_per_dictionary:92,context_entries:8708};
        assert_eq!(execute_pdf_dictionaries_with_aggregation(&g,&p,l).unwrap(),d);
        for l in [ExecutionLimits{symbol_slots:10,..l},ExecutionLimits{packed_bytes:3,..l},
            ExecutionLimits{context_entries:8707,..l},ExecutionLimits{work:l.work-1,..l},
            ExecutionLimits{decisions_per_dictionary:91,..l}]{
            assert!(execute_pdf_dictionaries_with_aggregation(&g,&p,l).is_err());
        }
    }
}

#[cfg(test)]
mod huffman_aggregation_dictionary_tests{
    use super::*;
    // Raw Huffman 1x1 black global symbol from prior independent fixture.
    const SOURCE:&[u8]=&[0,1,0,0,0,1,0,0,0,1,95,128,128,0,64];
    // B.4 DH=1, B.2 DW=3, B.1 aggregate count=2;
    // Table 17 initial DT=1, DT=1, FS=0, ID=0 RI=0,
    // DS=2 ID=0 RI=0 OOB; width OOB; export 0,2.
    const DIRECT:&[u8]=&[112,16,0,52,63,128,64];
    fn seg(id:u8,kind:u8,page:u8,refs:&[u8],data:&[u8])->Vec<u8>{
        let mut v=vec![0,0,0,id,kind,(refs.len()as u8)<<5];
        v.extend(refs);v.push(page);v.extend((data.len()as u32).to_be_bytes());v.extend(data);v
    }
    fn body(t:u8)->Vec<u8>{
        let mut v=vec![t<<4,3];if t==0{v.extend([255;4]);}
        v.extend([0,0,0,2,0,0,0,1]);v.extend(DIRECT);v
    }
    #[test]
    fn huffman_aggregation_opt_in_executor_pixels_and_limits(){
        for t in [0,1]{
            let g=seg(1,0,0,&[],SOURCE);
            let mut info=Vec::new();for n in [20u32,20,300,300]{info.extend(n.to_be_bytes());}
            info.extend([0,0,0]);let mut p=seg(2,48,1,&[],&info);
            p.extend(seg(3,0,1,&[1],&body(t)));
            let l=ExecutionLimits{decisions_per_dictionary:0,..ExecutionLimits::default()};
            assert!(matches!(execute_pdf_dictionaries_with_aggregation(&g,&p,l),Err(Error::Unsupported(_))));
            assert!(matches!(execute_pdf_dictionaries_single_reference(&g,&p,l),Err(Error::Unsupported(_))));
            let d=execute_pdf_dictionaries_with_huffman_aggregation(&g,&p,l).unwrap();
            assert_eq!(execute_pdf_dictionaries(&g,&p,l).unwrap(),d);
            let stats=d.stats();
            assert_eq!(d.symbol(3,1).unwrap().packed(),[160]);
            assert_eq!((d.symbol(3,1).unwrap().width(),d.symbol(3,1).unwrap().height()),(3,1));
            assert!(std::ptr::eq(d.symbol(1,0).unwrap(),d.symbol(3,0).unwrap()));
            assert_eq!((stats.dictionaries,stats.huffman_aggregation,stats.huffman_single_refinement),(2,1,0));
            assert_eq!((stats.new_symbols,stats.pixels,stats.packed_bytes,stats.symbol_slots),(2,4,2,8));
            let exact=ExecutionLimits{pixels:stats.pixels,packed_bytes:stats.packed_bytes,
                work:stats.work_used,symbol_slots:stats.symbol_slots,context_entries:stats.peak_context_entries,..l};
            assert_eq!(execute_pdf_dictionaries_with_huffman_aggregation(&g,&p,exact).unwrap(),d);
            for cap in [ExecutionLimits{pixels:3,..exact},ExecutionLimits{packed_bytes:1,..exact},
                ExecutionLimits{work:exact.work-1,..exact},ExecutionLimits{symbol_slots:7,..exact},
                ExecutionLimits{context_entries:exact.context_entries-1,..exact}]{
                assert!(execute_pdf_dictionaries_with_huffman_aggregation(&g,&p,cap).is_err());
            }
        }
    }
}
