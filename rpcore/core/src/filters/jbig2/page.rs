//! Bounded native page executor for embedded, known-height, unstriped
//! T.88 direct regions. All unsupported segment kinds fail closed.
//! No independent refinement regions, intermediate buffers or retention
//! lifecycle yet. The symbol dictionary arena is decoded once up front.
use crate::{Error,Result};
use super::{bitmap::{self,Bitmap,ComposeBudget,ComposeOp},dictionary,generic,pattern,halftone,text_region,PageInfo,SegmentSource};
use std::collections::BTreeMap;

#[derive(Debug,Clone,Copy)]
pub struct Limits{
    pub dictionaries:dictionary::ExecutionLimits,
    pub regions:usize,pub page_pixels:u64,pub region_pixels:u64,
    pub patterns:u32,pub pattern_pixels:u64,pub grid_cells:u64,
    pub text_slots:usize,pub text_instances:u64,pub huffman_bits:usize,
    /// Across pattern, generic, text and halftone regions; symbol
    /// dictionary entropy retains its independent per-dictionary cap.
    pub decisions:usize,pub packed_bytes:usize,pub work:u64,
}
impl Default for Limits{
    fn default()->Self{Self{dictionaries:dictionary::ExecutionLimits::default(),
        regions:super::MAX_SEGMENTS,page_pixels:bitmap::MAX_PIXELS,region_pixels:bitmap::MAX_PIXELS,
        patterns:pattern::MAX_PATTERNS,pattern_pixels:bitmap::MAX_PIXELS,grid_cells:halftone::MAX_GRID_CELLS,
        text_slots:text_region::MAX_INSTANCES as usize,text_instances:u64::from(text_region::MAX_INSTANCES),
        huffman_bits:text_region::MAX_TEXT_DECISIONS,decisions:halftone::MAX_DECISIONS,
        packed_bytes:bitmap::MAX_PACKED_BYTES,work:bitmap::MAX_COMPOSE_WORK}}
}
#[derive(Debug,Clone,Copy,Default,PartialEq,Eq)]
pub struct Stats{
    pub dictionaries:dictionary::ExecutionStats,pub pattern_dictionaries:usize,
    pub patterns:u32,pub pattern_pixels:u64,pub grid_cells:u64,
    pub generic_regions:usize,pub text_regions:usize,pub halftone_regions:usize,
    pub region_pixels:u64,pub text_slots:usize,pub text_instances:u64,
    pub huffman_bits:usize,pub decisions:usize,pub composed_pixels:u64,pub work_used:u64,
    /// Retained packed page and dictionary images. Transient output and
    /// scratch are constrained before each decode but not retained.
    pub retained_packed_bytes:usize,
}
#[derive(Debug,PartialEq,Eq)]
pub struct DecodedPage{pub info:PageInfo,pub bitmap:Bitmap,pub stats:Stats}
fn charge(stats:&mut Stats,limit:u64,n:u64)->Result<()>{
    let next=stats.work_used.checked_add(n).ok_or(Error::Limit("JBIG2 page work"))?;
    if next>limit{return Err(Error::Limit("JBIG2 page work"));}
    stats.work_used=next;Ok(())
}
fn packed(w:u32,h:u32)->usize{(w as usize).div_ceil(8)*h as usize}
/// Decode and compose immediate text (6/7), halftone (22/23), and generic
/// (38/39) regions in graph order. Honor page default pixel and external
/// operator permissions (7.4.8.5). Reject conflicting direct operators
/// when override is disallowed, rather than silently changing pixels.
/// No caller-supplied graph. Unknown/unsupported kinds fail before decoding.
pub fn decode_pdf_page(globals:&[u8],page:&[u8],limits:Limits)->Result<DecodedPage>{
    if limits.regions>super::MAX_SEGMENTS||limits.page_pixels>bitmap::MAX_PIXELS||
        limits.region_pixels>bitmap::MAX_PIXELS||limits.patterns>pattern::MAX_PATTERNS||
        limits.pattern_pixels>bitmap::MAX_PIXELS||limits.grid_cells>halftone::MAX_GRID_CELLS||
        limits.text_slots>text_region::MAX_INSTANCES as usize||
        limits.text_instances>u64::from(text_region::MAX_INSTANCES)||
        limits.huffman_bits>text_region::MAX_TEXT_DECISIONS||limits.decisions>halftone::MAX_DECISIONS||
        limits.packed_bytes>bitmap::MAX_PACKED_BYTES||limits.work>bitmap::MAX_COMPOSE_WORK{
        return Err(Error::Limit("JBIG2 page limits"));
    }
    let input=globals.len().checked_add(page.len()).ok_or(Error::Limit("JBIG2 page input"))?;
    if input as u64>limits.work{return Err(Error::Limit("JBIG2 page work"));}
    let graph=super::inspect_pdf_streams(globals,page)?;
    let info=graph.page.clone();
    if info.flags&128!=0{return Err(Error::Invalid("JBIG2 page reserved flag"));}
    if info.flags&32!=0{return Err(Error::Unsupported("JBIG2 page auxiliary buffers"));}
    let height=info.height.ok_or(Error::Unsupported("JBIG2 page unknown height"))?;
    let area=u64::from(info.width)*u64::from(height);
    let page_bytes=packed(info.width,height);
    if area>limits.page_pixels||page_bytes>limits.packed_bytes{return Err(Error::Limit("JBIG2 page canvas"));}
    let default_op=ComposeOp::from_code((info.flags>>3)&3)?;
    let override_allowed=info.flags&64!=0;
    let mut count=0usize;
    let mut setup=input as u64+graph.graph.len() as u64*2+area;
    for node in &graph.graph{
        let seg=graph.segment(node.location).ok_or(Error::Invalid("JBIG2 page segment"))?;
        setup+=node.dependencies.len() as u64;
        match seg.kind{
            0|53|48|49|51=>{},
            16=>{
                if !node.dependencies.is_empty(){return Err(Error::Invalid("JBIG2 pattern references"));}
            },
            6|7|22|23|38|39=>{
                count+=1;
                let data=page.get(seg.payload.clone()).ok_or(Error::Invalid("JBIG2 page region span"))?;
                let (region,_)=bitmap::parse_region(data)?;
                if !override_allowed&&region.op!=default_op{return Err(Error::Invalid("JBIG2 page combination override"));}
                if matches!(seg.kind,38|39)&&!node.dependencies.is_empty(){return Err(Error::Invalid("JBIG2 generic references"));}
                if matches!(seg.kind,22|23)&&(node.dependencies.len()!=1||
                    graph.segment(node.dependencies[0]).is_none_or(|d|d.kind!=16)){
                    return Err(Error::Invalid("JBIG2 halftone dictionary reference"));
                }
            },
            _=>return Err(Error::Unsupported("JBIG2 page segment execution kind")),
        }
    }
    if count>limits.regions||setup>limits.work{return Err(Error::Limit("JBIG2 page count or work"));}
    let mut dl=limits.dictionaries;
    dl.work=dl.work.min(limits.work-setup);
    dl.packed_bytes=dl.packed_bytes.min(limits.packed_bytes-page_bytes);
    let dictionaries=dictionary::execute_pdf_dictionaries_inspected(globals,page,dl,true,true,&graph)?;
    let ds=dictionaries.stats();
    let mut stats=Stats{dictionaries:ds,work_used:setup+ds.work_used,
        retained_packed_bytes:page_bytes+ds.packed_bytes,..Stats::default()};
    let mut canvas=Bitmap::new(info.width,height,info.flags&4!=0)?;
    let mut patterns=BTreeMap::<u32,pattern::PatternDictionary>::new();
    for node in &graph.graph{
        let seg=graph.segment(node.location).ok_or(Error::Invalid("JBIG2 page segment"))?;
        if !matches!(seg.kind,16|6|7|22|23|38|39){continue;}
        let stream=match node.location.source{SegmentSource::Globals=>globals,SegmentSource::Page=>page};
        let data=stream.get(seg.payload.clone()).ok_or(Error::Invalid("JBIG2 page payload span"))?;
        let work=limits.work-stats.work_used;
        let available=limits.packed_bytes-stats.retained_packed_bytes;
        let decisions=limits.decisions-stats.decisions;
        if seg.kind==16{
            let d=pattern::decode(data,pattern::Limits{patterns:limits.patterns-stats.patterns,
                pixels:limits.pattern_pixels-stats.pattern_pixels,packed_bytes:available,work,decisions})?;
            charge(&mut stats,limits.work,d.work_used)?;
            stats.pattern_dictionaries+=1;stats.patterns+=d.header.count;
            stats.pattern_pixels+=u64::from(d.header.collective_width)*u64::from(d.header.height);
            stats.retained_packed_bytes+=d.packed_bytes;
            if let pattern::EntropyStats::Arithmetic{decisions}=d.entropy{stats.decisions+=decisions;}
            patterns.insert(seg.number,d);continue;
        }
        let (region,_)=bitmap::parse_region(data)?;
        let pixels=u64::from(region.width)*u64::from(region.height);
        if pixels>limits.region_pixels-stats.region_pixels{return Err(Error::Limit("JBIG2 page region pixels"));}
        let image=match seg.kind{
            6|7=>{
                let d=text_region::execute_text_segment_inspected(globals,page,&graph,node,&dictionaries,
                    text_region::TextSegmentLimits{dictionaries:dl,work:work+ds.work_used,
                        packed_bytes:available+ds.packed_bytes,symbol_slots:limits.text_slots-stats.text_slots,
                        decisions,huffman_bits:limits.huffman_bits-stats.huffman_bits},0,
                        limits.region_pixels-stats.region_pixels,limits.text_instances-stats.text_instances)?;
                charge(&mut stats,limits.work,d.work_used-ds.work_used)?;
                stats.text_regions+=1;stats.text_slots+=d.symbol_slots;stats.text_instances+=u64::from(d.text.stats.instances);
                stats.huffman_bits+=d.huffman_bits_used;stats.decisions+=d.decisions_used;d.text.bitmap
            },
            22|23=>{
                let dep=graph.segment(node.dependencies[0]).ok_or(Error::Invalid("JBIG2 page halftone reference"))?;
                let dict=patterns.get(&dep.number).ok_or(Error::Invalid("JBIG2 page pattern dictionary"))?;
                let d=halftone::decode(data,dict,halftone::Limits{grid_cells:limits.grid_cells-stats.grid_cells,
                    pixels:limits.region_pixels-stats.region_pixels,packed_bytes:available,decisions,work})?;
                charge(&mut stats,limits.work,d.work_used)?;
                stats.halftone_regions+=1;stats.decisions+=d.decisions;
                stats.grid_cells+=u64::from(d.header.grid.width)*u64::from(d.header.grid.height);d.bitmap
            },
            38|39=>{
                let (h,payload)=generic::parse(data)?;
                let len=packed(region.width,region.height);
                // MMR conversion holds source and destination at once.
                let (scratch,overhead,bound)=match h.coding{
                    generic::Coding::Mmr=>(len*2,0,None),
                    generic::Coding::Arithmetic{neighborhood,typical_prediction}=>{
                        let rows=if typical_prediction{u64::from(region.height)}else{0};
                        let upper=pixels+rows;
                        if upper>decisions as u64{return Err(Error::Limit("JBIG2 page generic decisions bound"));}
                        (len,neighborhood.template().context_count() as u64,
                            Some(pixels*(neighborhood.template().bits() as u64+1)+rows))
                    },
                };
                if scratch>available{return Err(Error::Limit("JBIG2 page generic packed budget"));}
                let remaining=work.checked_sub(overhead).ok_or(Error::Limit("JBIG2 page generic work"))?;
                let d=generic::decode_region(data,remaining)?;
                let cost=match d.stats{
                    generic::DecodeStats::Arithmetic{decisions,..}=>{
                        stats.decisions+=decisions;
                        bound.ok_or(Error::Invalid("JBIG2 page generic coding"))?
                    },
                    generic::DecodeStats::Mmr{mode_counts,..}=>pixels*2+payload.len() as u64*8+
                        32*mode_counts.iter().sum::<u64>()+len as u64*3,
                };
                charge(&mut stats,limits.work,cost+overhead)?;
                stats.generic_regions+=1;d.bitmap
            },
            _=>return Err(Error::Unsupported("JBIG2 page region kind")),
        };
        stats.region_pixels+=pixels;
        let left=limits.work-stats.work_used;
        let mut budget=ComposeBudget::new(left)?;
        canvas.compose_region(&image,region,&mut budget)?;
        let used=left-budget.remaining();
        charge(&mut stats,limits.work,used)?;stats.composed_pixels+=used;
    }
    Ok(DecodedPage{info,bitmap:canvas,stats})
}

#[cfg(test)]
mod tests{
    use super::*;
    const GLOBAL:&[u8]=&[0x00,0x00,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x00,0x18,0x00,0x01,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x01,0xe9,0xcb,0xf4,0x00,0x26,0xaf,0x04,0xbf,0xf0,0x78,0x2f,0xe0,0x00,0x40];
    const PREFIX:&[u8]=&[0x00,0x00,0x00,0x01,0x30,0x00,0x01,0x00,0x00,0x00,0x13,0x00,0x00,0x00,0x40,0x00,0x00,0x00,0x38,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x00,0x00,0x02,0x00,0x01,0x01,0x00,0x00,0x00,0x1c,0x00,0x01,0x00,0x00,0x00,0x02,0x00,0x00,0x00,0x02,0xe5,0xcd,0xf8,0x00,0x79,0xe0,0x84,0x10,0x81,0xf0,0x82,0x10,0x86,0x10,0x79,0xf0,0x00,0x80];
    const TEXT:&[u8]=&[0x00,0x00,0x00,0x03,0x07,0x42,0x00,0x02,0x01,0x00,0x00,0x00,0x31,0x00,0x00,0x00,0x25,0x00,0x00,0x00,0x08,0x00,0x00,0x00,0x04,0x00,0x00,0x00,0x01,0x00,0x0c,0x09,0x00,0x10,0x00,0x00,0x00,0x05,0x01,0x10,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x0c,0x40,0x07,0x08,0x70,0x41,0xd0];
    const PATTERN:&[u8]=&[6,4,4,0,0,0,15,144,113,107,109,153,167,170,73,125,242,229,72,31,220,104,188,110,64,187,255,172];
    const HALF:&[u8]=&[0,0,0,32,0,0,0,36,0,0,0,16,0,0,0,15,0,2,0,0,0,8,0,0,0,9,0,0,0,0,0,0,0,0,4,0,0,0,135,203,130,30,102,164,20,235,60,74,21,250,204,214,243,177,111,76,237,191,167,191,255,172];
    const EXPECTED:&[u8]=&[
        0,0,0,0,0,0,0,0,
        7,135,143,15,15,0,0,0,
        8,64,72,128,144,128,0,0,
        8,7,200,143,144,0,0,0,
        8,8,72,144,144,0,0,0,
        8,72,79,16,144,128,0,0,
        7,135,200,15,143,0,0,0,
        0,0,8,0,0,0,0,0,
        0,0,8,0,0,0,0,0,
        0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,
        0,0,0,0,2,34,0,0,
        0,0,2,102,102,238,0,0,
        0,0,0,2,102,102,0,0,
        0,0,0,0,0,4,0,0,
        0,0,0,0,34,34,0,0,
        0,0,0,102,110,238,0,0,
        0,0,0,38,102,103,0,0,
        0,0,0,0,0,68,0,0,
        0,0,0,2,34,38,0,0,
        0,0,102,102,238,238,0,0,
        0,0,2,102,102,127,0,0,
        0,0,0,0,4,68,0,0,
        0,0,0,34,34,102,0,0,
        0,0,102,110,238,238,0,0,
        0,0,38,102,103,255,0,0,
        0,0,0,0,68,70,0,0,
        0,0,2,34,38,102,0,0,
        0,0,102,238,238,239,0,0,
        0,0,102,102,127,255,0,0,
        0,0,0,4,68,102,0,0,
        0,0,34,34,102,110,0,0,
        0,0,110,238,238,255,0,0,
        0,0,102,103,255,255,0,0,
        0,0,0,68,70,102,0,0,
        0,0,34,38,102,239,0,0,
        0,0,238,238,239,255,0,0,
        0,0,102,127,255,255,0,0,
        0,0,4,68,102,102,0,0,
        0,0,34,102,110,255,0,0,
        0,0,238,238,255,255,0,0,
        0,0,103,255,255,255,0,0,
        0,0,68,70,102,103,0,0,
        0,0,38,102,239,255,0,0,
        0,0,238,239,255,255,0,0,
        0,0,127,255,255,255,0,0,
        0,0,68,102,102,127,0,0,
        0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,
    ];
    fn seg(n:u8,kind:u8,refs:&[u8],body:&[u8])->Vec<u8>{
        let mut b=vec![0,0,0,n,kind,(refs.len() as u8)<<5];b.extend(refs);
        b.push(1);b.extend((body.len() as u32).to_be_bytes());b.extend(body);b
    }
    fn info(w:u32,h:u32,flags:u8)->Vec<u8>{
        let mut b=Vec::new();for n in [w,h,0,0]{b.extend(n.to_be_bytes());}
        b.extend([flags,0,0]);seg(1,48,&[],&b)
    }
    fn white(w:u32,h:u32,x:u32,y:u32,op:u8)->Vec<u8>{
        let mut b=Vec::new();for n in [w,h,x,y]{b.extend(n.to_be_bytes());}
        b.extend([op,1]); // vertical zero per white row; optional EOFB
        let mut data=vec![255;(h as usize).div_ceil(8)];
        if h%8!=0{let last=data.len()-1;data[last]=255u8<<(8-h%8);}
        b.extend(data);b
    }
    fn mixed()->Vec<u8>{
        // Original Annex H text and halftone bodies, synthetic page wrapper.
        let mut p=PREFIX.to_vec();p[27]=0x41;
        p.extend(TEXT);p.extend(seg(4,16,&[],PATTERN));p.extend(seg(5,23,&[4],HALF));
        p.extend(seg(6,39,&[],&white(8,4,16,17,4)));p.extend(seg(7,49,&[],&[]));p
    }
    #[test]
    fn page_mixed_direct_regions_exact_pixels(){
        let p=mixed();let l=Limits::default();let d=decode_pdf_page(GLOBAL,&p,l).unwrap();
        assert_eq!((d.bitmap.width(),d.bitmap.height()),(64,56));
        assert_eq!(d.bitmap.packed(),EXPECTED);
        let s=d.stats;
        assert_eq!((s.generic_regions,s.text_regions,s.halftone_regions,s.pattern_dictionaries),(1,1,1,1));
        assert_eq!((s.region_pixels,s.composed_pixels,s.patterns,s.grid_cells),(1480,1480,16,72));
        assert_eq!((s.text_slots,s.text_instances,s.huffman_bits,s.decisions),(3,5,192,544));
        let exact=Limits{regions:3,page_pixels:3584,region_pixels:1480,patterns:16,pattern_pixels:256,
            grid_cells:72,text_slots:3,text_instances:5,huffman_bits:192,decisions:544,work:s.work_used,..l};
        assert_eq!(decode_pdf_page(GLOBAL,&p,exact).unwrap(),d);
        for cap in [Limits{regions:2,..exact},Limits{page_pixels:3583,..exact},Limits{region_pixels:1479,..exact},
            Limits{patterns:15,..exact},Limits{grid_cells:71,..exact},Limits{text_slots:2,..exact},
            Limits{decisions:543,..exact},Limits{work:s.work_used-1,..exact},Limits{packed_bytes:447,..exact}]{
            assert!(decode_pdf_page(GLOBAL,&p,cap).is_err());
        }
    }
    #[test]
    fn page_default_and_ordered_operators(){
        let l=Limits::default();
        for (op,flags,expected) in [(0,0,0),(1,12,0x0f),(2,20,0xff),(3,28,0x0f)]{
            let mut p=info(8,1,flags);
            p.extend(seg(2,38,&[],&white(4,1,0,0,op)));
            let d=decode_pdf_page(&[],&p,l).unwrap();
            assert_eq!(d.bitmap.packed(),[expected]);
        }
        let mut p=info(8,1,0x44);
        p.extend(seg(2,39,&[],&white(5,1,0,0,4)));
        p.extend(seg(3,38,&[],&white(4,1,2,0,3)));
        assert_eq!(decode_pdf_page(&[],&p,l).unwrap().bitmap.packed(),[0x3b]);
        let mut p=info(9,2,4);
        assert_eq!(decode_pdf_page(&[],&p,l).unwrap().bitmap.packed(),[255,128,255,128]);
        p.extend(seg(2,38,&[],&white(3,1,0,0,4)));
        assert!(decode_pdf_page(&[],&p,l).is_err());
    }
    #[test]
    fn page_unsupported_and_bad_dependencies_fail_closed(){
        let l=Limits::default();
        for kind in [4,20,36,40,42,43,52,62]{
            let mut p=info(8,1,0);p.extend(seg(2,kind,&[],&[]));
            assert!(decode_pdf_page(&[],&p,l).is_err());
        }
        for flags in [32,128]{
            assert!(decode_pdf_page(&[],&info(8,1,flags),l).is_err());
        }
        let mut p=info(8,1,0);p.extend(seg(2,16,&[],PATTERN));p.extend(seg(3,38,&[2],&white(8,1,0,0,0)));
        assert!(decode_pdf_page(&[],&p,l).is_err());
        let mut p=mixed();let end=p.len();p[end-12]=0;
        // Invalid final MMR byte before EOP.
        assert!(decode_pdf_page(GLOBAL,&p,l).is_err());
    }
}

#[cfg(test)]
mod annex_h_full_page_tests{
    use super::*;
    // T.88 (02/2000) Annex H.1 pp.122–142. Original segment bytes,
    // including headers, retention flags, associations, and EOP/EOF.
    // Split into independent embedded pages and applicable globals.
    // No entropy re-encoding, synthetic segment wrappers or page flags.
    // https://www.itu.int/rec/T-REC-T.88-200002-S/en
    const GLOBAL1:&[u8]=&[
        0x00,0x00,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x00,0x18,0x00,0x01,0x00,0x00,0x00,
        0x01,0x00,0x00,0x00,0x01,0xe9,0xcb,0xf4,0x00,0x26,0xaf,0x04,0xbf,0xf0,0x78,0x2f,
        0xe0,0x00,0x40,
    ];
    const PAGE1:&[u8]=&[
        0x00,0x00,0x00,0x01,0x30,0x00,0x01,0x00,0x00,0x00,0x13,0x00,0x00,0x00,0x40,0x00,
        0x00,0x00,0x38,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x00,
        0x00,0x02,0x00,0x01,0x01,0x00,0x00,0x00,0x1c,0x00,0x01,0x00,0x00,0x00,0x02,0x00,
        0x00,0x00,0x02,0xe5,0xcd,0xf8,0x00,0x79,0xe0,0x84,0x10,0x81,0xf0,0x82,0x10,0x86,
        0x10,0x79,0xf0,0x00,0x80,0x00,0x00,0x00,0x03,0x07,0x42,0x00,0x02,0x01,0x00,0x00,
        0x00,0x31,0x00,0x00,0x00,0x25,0x00,0x00,0x00,0x08,0x00,0x00,0x00,0x04,0x00,0x00,
        0x00,0x01,0x00,0x0c,0x09,0x00,0x10,0x00,0x00,0x00,0x05,0x01,0x10,0x00,0x00,0x00,
        0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x0c,0x40,0x07,0x08,
        0x70,0x41,0xd0,0x00,0x00,0x00,0x04,0x27,0x00,0x01,0x00,0x00,0x00,0x2c,0x00,0x00,
        0x00,0x36,0x00,0x00,0x00,0x2c,0x00,0x00,0x00,0x04,0x00,0x00,0x00,0x0b,0x00,0x01,
        0x26,0xa0,0x71,0xce,0xa7,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
        0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xf8,0xf0,0x00,0x00,0x00,0x05,0x10,0x01,
        0x01,0x00,0x00,0x00,0x2d,0x01,0x04,0x04,0x00,0x00,0x00,0x0f,0x20,0xd1,0x84,0x61,
        0x18,0x45,0xf2,0xf9,0x7c,0x8f,0x11,0xc3,0x9e,0x45,0xf2,0xf9,0x7d,0x42,0x85,0x0a,
        0xaa,0x84,0x62,0x2f,0xee,0xec,0x44,0x62,0x22,0x35,0x2a,0x0a,0x83,0xb9,0xdc,0xee,
        0x77,0x80,0x00,0x00,0x00,0x06,0x17,0x20,0x05,0x01,0x00,0x00,0x00,0x57,0x00,0x00,
        0x00,0x20,0x00,0x00,0x00,0x24,0x00,0x00,0x00,0x10,0x00,0x00,0x00,0x0f,0x00,0x01,
        0x00,0x00,0x00,0x08,0x00,0x00,0x00,0x09,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
        0x04,0x00,0x00,0x00,0xaa,0xaa,0xaa,0xaa,0x80,0x08,0x00,0x80,0x36,0xd5,0x55,0x6b,
        0x5a,0xd4,0x00,0x40,0x04,0x2e,0xe9,0x52,0xd2,0xd2,0xd2,0x8a,0xa5,0x4a,0x00,0x20,
        0x02,0x23,0xe0,0x95,0x24,0xb4,0x92,0x8a,0x4a,0x92,0x54,0x92,0xd2,0x4a,0x29,0x2a,
        0x49,0x40,0x04,0x00,0x40,0x00,0x00,0x00,0x07,0x31,0x00,0x01,0x00,0x00,0x00,0x00,
    ];
    const PAGE2:&[u8]=&[
        0x00,0x00,0x00,0x08,0x30,0x00,0x02,0x00,0x00,0x00,0x13,0x00,0x00,0x00,0x40,0x00,
        0x00,0x00,0x38,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x00,
        0x00,0x09,0x00,0x01,0x02,0x00,0x00,0x00,0x1b,0x08,0x00,0x02,0xff,0x00,0x00,0x00,
        0x02,0x00,0x00,0x00,0x02,0x4f,0xe7,0x8c,0x20,0x0e,0x1d,0xc7,0xcf,0x01,0x11,0xc4,
        0xb2,0x6f,0xff,0xac,0x00,0x00,0x00,0x0a,0x07,0x40,0x00,0x09,0x02,0x00,0x00,0x00,
        0x1f,0x00,0x00,0x00,0x25,0x00,0x00,0x00,0x08,0x00,0x00,0x00,0x04,0x00,0x00,0x00,
        0x01,0x00,0x0c,0x08,0x00,0x00,0x00,0x05,0x8d,0x6e,0x5a,0x12,0x40,0x85,0xff,0xac,
        0x00,0x00,0x00,0x0b,0x27,0x00,0x02,0x00,0x00,0x00,0x23,0x00,0x00,0x00,0x36,0x00,
        0x00,0x00,0x2c,0x00,0x00,0x00,0x04,0x00,0x00,0x00,0x0b,0x00,0x08,0x03,0xff,0xfd,
        0xff,0x02,0xfe,0xfe,0xfe,0x04,0xee,0xed,0x87,0xfb,0xcb,0x2b,0xff,0xac,0x00,0x00,
        0x00,0x0c,0x10,0x01,0x02,0x00,0x00,0x00,0x1c,0x06,0x04,0x04,0x00,0x00,0x00,0x0f,
        0x90,0x71,0x6b,0x6d,0x99,0xa7,0xaa,0x49,0x7d,0xf2,0xe5,0x48,0x1f,0xdc,0x68,0xbc,
        0x6e,0x40,0xbb,0xff,0xac,0x00,0x00,0x00,0x0d,0x17,0x20,0x0c,0x02,0x00,0x00,0x00,
        0x3e,0x00,0x00,0x00,0x20,0x00,0x00,0x00,0x24,0x00,0x00,0x00,0x10,0x00,0x00,0x00,
        0x0f,0x00,0x02,0x00,0x00,0x00,0x08,0x00,0x00,0x00,0x09,0x00,0x00,0x00,0x00,0x00,
        0x00,0x00,0x00,0x04,0x00,0x00,0x00,0x87,0xcb,0x82,0x1e,0x66,0xa4,0x14,0xeb,0x3c,
        0x4a,0x15,0xfa,0xcc,0xd6,0xf3,0xb1,0x6f,0x4c,0xed,0xbf,0xa7,0xbf,0xff,0xac,0x00,
        0x00,0x00,0x0e,0x31,0x00,0x02,0x00,0x00,0x00,0x00,
    ];
    const GLOBAL3:&[u8]=&[
        0x00,0x00,0x00,0x10,0x00,0x01,0x00,0x00,0x00,0x00,0x16,0x08,0x00,0x02,0xff,0x00,
        0x00,0x00,0x01,0x00,0x00,0x00,0x01,0x4f,0xe7,0x8d,0x68,0x1b,0x14,0x2f,0x3f,0xff,
        0xac,
    ];
    const PAGE3:&[u8]=&[
        0x00,0x00,0x00,0x0f,0x30,0x00,0x03,0x00,0x00,0x00,0x13,0x00,0x00,0x00,0x25,0x00,
        0x00,0x00,0x08,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x00,
        0x00,0x11,0x00,0x21,0x10,0x03,0x00,0x00,0x00,0x20,0x08,0x02,0x02,0xff,0xff,0xff,
        0xff,0xff,0x00,0x00,0x00,0x03,0x00,0x00,0x00,0x02,0x4f,0xe9,0xd7,0xd5,0x90,0xc3,
        0xb5,0x26,0xa7,0xfb,0x6d,0x14,0x98,0x3f,0xff,0xac,0x00,0x00,0x00,0x12,0x07,0x20,
        0x11,0x03,0x00,0x00,0x00,0x25,0x00,0x00,0x00,0x25,0x00,0x00,0x00,0x08,0x00,0x00,
        0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x8c,0x12,0x00,0x00,0x00,0x04,0xa9,0x5c,0x8b,
        0xf4,0xc3,0x7d,0x96,0x6a,0x28,0xe5,0x76,0x8f,0xff,0xac,0x00,0x00,0x00,0x13,0x31,
        0x00,0x03,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x14,0x33,0x00,0x00,0x00,0x00,0x00,
        0x00,
    ];
    // Expected page independently constructed from the normative H.5,
    // H.6 and H.11 reference images, using placements in H.1 item 17.
    // H.6: 54x44 rectangle, two-pixel black border. Not decoder output.
    const EXPECTED12:&[u8]=&[
        0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
        0x07,0x87,0x8f,0x0f,0x0f,0x00,0x00,0x00,
        0x08,0x40,0x48,0x80,0x90,0x80,0x00,0x00,
        0x08,0x07,0xc8,0x8f,0x90,0x00,0x00,0x00,
        0x08,0x08,0x48,0x90,0x90,0x00,0x00,0x00,
        0x08,0x48,0x4f,0x10,0x90,0x80,0x00,0x00,
        0x07,0x87,0xc8,0x0f,0x8f,0x00,0x00,0x00,
        0x00,0x00,0x08,0x00,0x00,0x00,0x00,0x00,
        0x00,0x00,0x08,0x00,0x00,0x00,0x00,0x00,
        0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
        0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
        0x0f,0xff,0xff,0xff,0xff,0xff,0xff,0xc0,
        0x0f,0xff,0xff,0xff,0xff,0xff,0xff,0xc0,
        0x0c,0x00,0x00,0x00,0x00,0x00,0x00,0xc0,
        0x0c,0x00,0x00,0x00,0x00,0x00,0x00,0xc0,
        0x0c,0x00,0x00,0x00,0x02,0x22,0x00,0xc0,
        0x0c,0x00,0x02,0x66,0x66,0xee,0x00,0xc0,
        0x0c,0x00,0x00,0x02,0x66,0x66,0x00,0xc0,
        0x0c,0x00,0x00,0x00,0x00,0x04,0x00,0xc0,
        0x0c,0x00,0x00,0x00,0x22,0x22,0x00,0xc0,
        0x0c,0x00,0x26,0x66,0x6e,0xee,0x00,0xc0,
        0x0c,0x00,0x00,0x26,0x66,0x67,0x00,0xc0,
        0x0c,0x00,0x00,0x00,0x00,0x44,0x00,0xc0,
        0x0c,0x00,0x00,0x02,0x22,0x26,0x00,0xc0,
        0x0c,0x00,0x66,0x66,0xee,0xee,0x00,0xc0,
        0x0c,0x00,0x02,0x66,0x66,0x7f,0x00,0xc0,
        0x0c,0x00,0x00,0x00,0x04,0x44,0x00,0xc0,
        0x0c,0x00,0x00,0x22,0x22,0x66,0x00,0xc0,
        0x0c,0x00,0x66,0x6e,0xee,0xee,0x00,0xc0,
        0x0c,0x00,0x26,0x66,0x67,0xff,0x00,0xc0,
        0x0c,0x00,0x00,0x00,0x44,0x46,0x00,0xc0,
        0x0c,0x00,0x02,0x22,0x26,0x66,0x00,0xc0,
        0x0c,0x00,0x66,0xee,0xee,0xef,0x00,0xc0,
        0x0c,0x00,0x66,0x66,0x7f,0xff,0x00,0xc0,
        0x0c,0x00,0x00,0x04,0x44,0x66,0x00,0xc0,
        0x0c,0x00,0x22,0x22,0x66,0x6e,0x00,0xc0,
        0x0c,0x00,0x6e,0xee,0xee,0xff,0x00,0xc0,
        0x0c,0x00,0x66,0x67,0xff,0xff,0x00,0xc0,
        0x0c,0x00,0x00,0x44,0x46,0x66,0x00,0xc0,
        0x0c,0x00,0x22,0x26,0x66,0xef,0x00,0xc0,
        0x0c,0x00,0xee,0xee,0xef,0xff,0x00,0xc0,
        0x0c,0x00,0x66,0x7f,0xff,0xff,0x00,0xc0,
        0x0c,0x00,0x04,0x44,0x66,0x66,0x00,0xc0,
        0x0c,0x00,0x22,0x66,0x6e,0xff,0x00,0xc0,
        0x0c,0x00,0xee,0xee,0xff,0xff,0x00,0xc0,
        0x0c,0x00,0x67,0xff,0xff,0xff,0x00,0xc0,
        0x0c,0x00,0x44,0x46,0x66,0x67,0x00,0xc0,
        0x0c,0x00,0x26,0x66,0xef,0xff,0x00,0xc0,
        0x0c,0x00,0xee,0xef,0xff,0xff,0x00,0xc0,
        0x0c,0x00,0x7f,0xff,0xff,0xff,0x00,0xc0,
        0x0c,0x00,0x44,0x66,0x66,0x7f,0x00,0xc0,
        0x0c,0x00,0x00,0x00,0x00,0x00,0x00,0xc0,
        0x0c,0x00,0x00,0x00,0x00,0x00,0x00,0xc0,
        0x0f,0xff,0xff,0xff,0xff,0xff,0xff,0xc0,
        0x0f,0xff,0xff,0xff,0xff,0xff,0xff,0xc0,
        0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
    ];
    const EXPECTED3:&[u8]=&[
        0x78,0x78,0xf0,0xf0,0xf0,
        0x84,0x04,0x88,0x09,0x08,
        0x80,0x7c,0x88,0xf9,0x00,
        0x80,0x84,0x89,0x09,0x00,
        0x84,0x84,0xf1,0x09,0x08,
        0x78,0x7c,0x80,0xf8,0xf0,
        0x00,0x00,0x80,0x00,0x00,
        0x00,0x00,0x80,0x00,0x00,
    ];
    #[test]
    fn annex_h_original_complete_pages_mmr_arithmetic(){
        let l=Limits::default();
        let a=decode_pdf_page(GLOBAL1,PAGE1,l).unwrap();
        let b=decode_pdf_page(GLOBAL1,PAGE2,l).unwrap();
        for d in [&a,&b]{
            assert_eq!((d.info.width,d.info.height,d.info.flags),(64,Some(56),1));
            assert_eq!(d.bitmap.packed(),EXPECTED12);
            assert_eq!(d.bitmap.packed().iter().map(|v|v.count_ones()).sum::<u32>(),1027);
            assert_eq!((d.stats.generic_regions,d.stats.text_regions,d.stats.halftone_regions,d.stats.patterns),(1,1,1,16));
            assert_eq!((d.stats.region_pixels,d.stats.composed_pixels,d.stats.grid_cells),(3824,3824,72));
        }
        assert_eq!((a.info.page,b.info.page),(1,2));
        assert_eq!(a.bitmap,b.bitmap);
        assert_eq!(a.stats.decisions,0);assert_eq!(a.stats.huffman_bits,192);
        assert!(b.stats.decisions>544);assert_eq!(b.stats.huffman_bits,0);
    }
    #[test]
    fn annex_h_original_complete_page_aggregate_refined_text(){
        let l=Limits::default();
        let d=decode_pdf_page(GLOBAL3,PAGE3,l).unwrap();
        assert_eq!((d.info.page,d.info.width,d.info.height,d.info.flags),(3,37,Some(8),1));
        assert_eq!(d.bitmap.packed(),EXPECTED3);
        assert_eq!((d.stats.text_regions,d.stats.text_instances,d.stats.text_slots),(1,4,3));
        assert_eq!((d.stats.generic_regions,d.stats.halftone_regions,d.stats.patterns),(0,0,0));
        assert_eq!((d.stats.region_pixels,d.stats.composed_pixels),(296,296));
        assert!(d.stats.decisions>0);
        assert_eq!(d.stats.dictionaries.arithmetic_aggregation,1);
    }
    #[test]
    fn annex_h_original_full_page_budget_and_corruption(){
        for (globals,page) in [(GLOBAL1,PAGE1),(GLOBAL1,PAGE2),(GLOBAL3,PAGE3)]{
            let l=Limits::default();let d=decode_pdf_page(globals,page,l).unwrap();
            let exact=Limits{work:d.stats.work_used,region_pixels:d.stats.region_pixels,
                page_pixels:u64::from(d.info.width)*u64::from(d.info.height.unwrap()),..l};
            assert_eq!(decode_pdf_page(globals,page,exact).unwrap(),d);
            assert!(decode_pdf_page(globals,page,Limits{work:exact.work-1,..exact}).is_err());
            assert!(decode_pdf_page(globals,page,Limits{region_pixels:exact.region_pixels-1,..exact}).is_err());
            assert!(decode_pdf_page(&[],page,l).is_err());
            // Last control header cannot be partially present. Known-length
            // embedded streams may legitimately omit EOP/EOF entirely.
            assert!(decode_pdf_page(globals,&page[..page.len()-1],l).is_err());
            let mut bad=page.to_vec();bad[27]|=128;
            assert!(decode_pdf_page(globals,&bad,l).is_err());
        }
        let bad=GLOBAL1.to_vec();
        assert!(decode_pdf_page(&bad,PAGE3,Limits::default()).is_err());
    }
    #[test]
    fn annex_h_original_sequential_file_structure(){
        let mut file=b"\x97JB2\r\n\x1a\n".to_vec();
        file.extend([1,0,0,0,3]);
        file.extend(GLOBAL1);file.extend(PAGE1);file.extend(PAGE2);
        file.extend(&PAGE3[..30]);file.extend(GLOBAL3);file.extend(&PAGE3[30..]);
        assert_eq!(file.len(),0x35c);
        let a=super::super::inspect(&file).unwrap();
        assert_eq!(a.container,super::super::Container::SequentialFile);
        assert_eq!(a.declared_pages,Some(3));assert_eq!(a.segments.len(),21);
        assert_eq!(a.segments.iter().map(|s|s.number).collect::<Vec<_>>(),(0..21).collect::<Vec<_>>());
        assert_eq!(a.segments[4].header.start,0xb3);
        assert_eq!(a.segments[7].header.start,0x185);
        assert_eq!(a.segments[11].header.start,0x200);
        assert_eq!(a.segments[15].header.start,0x2aa);
        assert_eq!(a.segments[20].header.start,0x351);
        assert!(a.external_references.is_empty());
    }
}
