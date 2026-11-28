//! Bounded native page executor for embedded, resolved-height
//! T.88 direct and refinement regions. All unsupported segment kinds fail closed.
//! Auxiliary region buffers follow T.88 8.2; general segment retention flags
//! are not yet executed. The symbol dictionary arena is decoded once up front.
use crate::{Error,Result};
use super::{bitmap::{self,Bitmap,ComposeBudget,ComposeOp},dictionary,generic,pattern,halftone,text_region,refinement,PageInfo,SegmentSource};
use std::collections::BTreeMap;

#[derive(Debug,Clone,Copy)]
pub struct Limits{
    pub dictionaries:dictionary::ExecutionLimits,
    pub regions:usize,pub page_pixels:u64,pub region_pixels:u64,
    pub patterns:u32,pub pattern_pixels:u64,pub grid_cells:u64,
    pub text_slots:usize,pub text_instances:u64,pub huffman_bits:usize,
    /// Across pattern, generic, text, halftone and refinement regions; symbol
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
    pub refinement_regions:usize,pub intermediate_regions:usize,
    pub auxiliary_consumed:usize,pub peak_auxiliary_bytes:usize,
    pub region_pixels:u64,pub text_slots:usize,pub text_instances:u64,
    pub huffman_bits:usize,pub decisions:usize,pub composed_pixels:u64,pub work_used:u64,
    /// Retained packed page, dictionary images and any live auxiliary buffers.
    /// Transient output and scratch are constrained before each decode.
    pub retained_packed_bytes:usize,
    pub released_symbol_dictionaries:usize,pub released_symbol_bitmaps:usize,pub released_symbol_bytes:usize,
    pub released_pattern_dictionaries:usize,pub released_pattern_bytes:usize,
}
#[derive(Debug,PartialEq,Eq)]
pub struct DecodedPage{pub info:PageInfo,pub bitmap:Bitmap,pub stats:Stats}
fn charge(stats:&mut Stats,limit:u64,n:u64)->Result<()>{
    let next=stats.work_used.checked_add(n).ok_or(Error::Limit("JBIG2 page work"))?;
    if next>limit{return Err(Error::Limit("JBIG2 page work"));}
    stats.work_used=next;Ok(())
}
fn packed(w:u32,h:u32)->usize{(w as usize).div_ceil(8)*h as usize}
fn stripe_end(body:&[u8])->Result<u32>{
    let bytes:[u8;4]=body.try_into().map_err(|_|Error::Invalid("JBIG2 stripe end layout"))?;
    u32::from_be_bytes(bytes).checked_add(1).ok_or(Error::Invalid("JBIG2 stripe end row"))
}
// Unknown page height is recovered from explicit EOS rows before allocating
// any canvas. No inference from untrusted region dimensions or PDF scaling.
fn resolve_striped_height(graph:&super::PdfInspection,page:&[u8],limits:Limits)->Result<u32>{
    let info=&graph.page;
    if !info.striped||info.max_stripe_size==0{return Err(Error::Invalid("JBIG2 unknown stripe size"));}
    let mut height=0u32;
    let mut pending=false;
    for s in &graph.page_stream.segments{
        if matches!(s.kind,6|7|22|23|38|39|42|43){pending=true;}
        if s.kind!=50{continue;}
        let end=stripe_end(page.get(s.payload.clone()).ok_or(Error::Invalid("JBIG2 stripe span"))?)?;
        if end<=height||end-height>u32::from(info.max_stripe_size){
            return Err(Error::Invalid("JBIG2 stripe end row"));
        }
        if end>super::MAX_SIDE||u64::from(info.width)*u64::from(end)>limits.page_pixels||
            u64::from(info.width).div_ceil(8)*u64::from(end)>limits.packed_bytes as u64{
            return Err(Error::Limit("JBIG2 recovered page dimensions"));
        }
        height=end;pending=false;
    }
    if height==0||pending{return Err(Error::Invalid("JBIG2 unknown height requires final stripe end"));}
    Ok(height)
}
/// Decode direct and refinement regions in graph order. Intermediate
/// outputs are never painted until an immediate refinement consumes them.
/// Honor page default pixel and external operator permissions (7.4.8.5),
/// including mandatory REPLACE for page-reference refinement.
/// No caller-supplied graph. Unknown/unsupported kinds fail before decoding.
pub fn decode_pdf_page(globals:&[u8],page:&[u8],limits:Limits)->Result<DecodedPage>{
    decode_pdf_page_impl(globals,page,limits,false,false)
}
/// Opt-in physical reclamation after the final actual consumer in the
/// immutable inspected page graph. No cross-image cache or changes to
/// wire retention semantics. Dictionary decode itself still conservatively
/// retains all bitmap imports until the dictionary pass completes; dead
/// GB/GR context snapshots are released during that pass.
pub fn decode_pdf_page_reclaimed(globals:&[u8],page:&[u8],limits:Limits)->Result<DecodedPage>{
    decode_pdf_page_impl(globals,page,limits,true,false)
}
/// PDF XObject compatibility: some deployed encoders omit the auxiliary
/// and refinement resource hints. Allocate only from the inspected segment
/// graph, not these hints. All dependencies, operators, region identity,
/// consumption rules and resource caps remain identical to the strict API.
pub(crate) fn decode_pdf_image_compatible(globals:&[u8],page:&[u8],limits:Limits)->Result<DecodedPage>{
    decode_pdf_page_impl(globals,page,limits,true,true)
}
/// Test-only entry to exercise the actual PDF XObject page path with
/// libFuzzer instrumentation. Not part of the default release API.
#[cfg(feature="fuzzing")]
#[doc(hidden)]
pub fn fuzz_pdf_image(globals:&[u8],page:&[u8],limits:Limits)->Result<DecodedPage>{
    decode_pdf_image_compatible(globals,page,limits)
}
fn record_release(stats:&mut Stats,r:dictionary::Reclaimed){
    stats.released_symbol_dictionaries+=r.dictionaries;
    stats.released_symbol_bitmaps+=r.bitmaps;stats.released_symbol_bytes+=r.bytes;
    stats.retained_packed_bytes-=r.bytes;
}
fn decode_pdf_page_impl(globals:&[u8],page:&[u8],limits:Limits,reclaim:bool,allow_missing_hints:bool)->Result<DecodedPage>{
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
    let mut graph=if allow_missing_hints{super::inspect_pdf_image_streams(globals,page)?}
        else{super::inspect_pdf_streams(globals,page)?};
    let recovered_height=graph.page.height.is_none();
    let recovery_work=if recovered_height{
        let cost=graph.page_stream.segments.len() as u64;
        if cost>limits.work-input as u64{return Err(Error::Limit("JBIG2 page height scan work"));}
        let height=resolve_striped_height(&graph,page,limits)?;
        graph.page.height=Some(height);
        cost
    }else{0};
    let info=graph.page.clone();
    if info.flags&128!=0{return Err(Error::Invalid("JBIG2 page reserved flag"));}
    let height=info.height.ok_or(Error::Unsupported("JBIG2 page unknown height"))?;
    if info.striped&&info.max_stripe_size==0{return Err(Error::Invalid("JBIG2 zero stripe size"));}
    let area=u64::from(info.width)*u64::from(height);
    let page_bytes=packed(info.width,height);
    if area>limits.page_pixels||page_bytes>limits.packed_bytes{return Err(Error::Limit("JBIG2 page canvas"));}
    let default_op=ComposeOp::from_code((info.flags>>3)&3)?;
    let override_allowed=info.flags&64!=0;
    let mut count=0usize;
    let mut setup=input as u64+graph.graph.len() as u64*2+area+recovery_work;
    for node in &graph.graph{
        let seg=graph.segment(node.location).ok_or(Error::Invalid("JBIG2 page segment"))?;
        setup+=node.dependencies.len() as u64;
        match seg.kind{
            0|53|48|49|51=>{},
            50 if info.striped=>{},
            62 if allow_missing_hints=>{}, // optional metadata already validated by graph
            16=>{
                if !node.dependencies.is_empty(){return Err(Error::Invalid("JBIG2 pattern references"));}
            },
            4|6|7|20|22|23|36|38|39|40|42|43=>{
                count+=1;
                let data=page.get(seg.payload.clone()).ok_or(Error::Invalid("JBIG2 page region span"))?;
                let (region,_)=bitmap::parse_region(data)?;
                if page.get(seg.header.end-4..seg.header.end)==Some(&[255u8;4][..]){
                    // Inspection plus the effective-header scan are bounded
                    // by the segment bytes, in addition to entropy work.
                    setup+=data.len() as u64*2;
                }
                let intermediate=matches!(seg.kind,4|20|36|40);
                let refined=matches!(seg.kind,40|42|43);
                if !allow_missing_hints&&intermediate&&info.flags&32==0{return Err(Error::Invalid("JBIG2 page auxiliary flag"));}
                if !allow_missing_hints&&refined&&info.flags&2==0{return Err(Error::Invalid("JBIG2 page refinement flag"));}
                let page_reference=refined&&node.dependencies.is_empty();
                if page_reference{
                    if intermediate||region.op!=ComposeOp::Replace{return Err(Error::Invalid("JBIG2 page refinement reference"));}
                }else if !override_allowed&&region.op!=default_op{return Err(Error::Invalid("JBIG2 page combination override"));}
                if refined&&!page_reference&&(node.dependencies.len()!=1||
                    graph.segment(node.dependencies[0]).is_none_or(|d|!matches!(d.kind,4|20|36|40))){
                    return Err(Error::Invalid("JBIG2 refinement dependency kind"));
                }
                if matches!(seg.kind,36|38|39)&&!node.dependencies.is_empty(){return Err(Error::Invalid("JBIG2 generic references"));}
                if matches!(seg.kind,20|22|23)&&(node.dependencies.len()!=1||
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
    let mut dictionaries=if reclaim{
        dictionary::execute_pdf_dictionaries_inspected_reclaimed_contexts(globals,page,dl,true,true,&graph)?
    }else{
        dictionary::execute_pdf_dictionaries_inspected(globals,page,dl,true,true,&graph)?
    };
    let ds=dictionaries.stats();
    let mut stats=Stats{dictionaries:ds,work_used:setup+ds.work_used,
        retained_packed_bytes:page_bytes+ds.packed_bytes,..Stats::default()};
    let mut uses=BTreeMap::<u32,usize>::new();
    let mut reclaimer=None;
    if reclaim{
        // Charge graph enumeration, slot counting and release traversal.
        // Cumulative dictionary symbol_slots upper-bounds export slots.
        let cost=graph.graph.len() as u64*3+ds.symbol_slots as u64*3+
            u64::from(ds.new_symbols)*2+
            graph.graph.iter().map(|n|n.dependencies.len() as u64*3).sum::<u64>();
        charge(&mut stats,limits.work,cost)?;
        for node in &graph.graph{
            let seg=graph.segment(node.location).ok_or(Error::Invalid("JBIG2 reclaim segment"))?;
            if matches!(seg.kind,4|6|7|20|22|23){
                for &loc in &node.dependencies{
                    let dep=graph.segment(loc).ok_or(Error::Invalid("JBIG2 reclaim dependency"))?;
                    if matches!(dep.kind,0|16){*uses.entry(dep.number).or_default()+=1;}
                }
            }
        }
        let (state,freed)=dictionaries.prepare_page_reclaim(&uses)?;
        record_release(&mut stats,freed);
        reclaimer=Some(state);
    }
    let mut canvas=Bitmap::new(info.width,height,info.flags&4!=0)?;
    let mut patterns=BTreeMap::<u32,pattern::PatternDictionary>::new();
    let mut auxiliary=BTreeMap::<u32,(bitmap::RegionInfo,Bitmap)>::new();
    let mut auxiliary_bytes=0usize;
    // Absolute page coordinates, exclusive completed-row boundary.
    let mut stripe_start=0u32;
    let mut stripe_written_end=0u32;
    for node in &graph.graph{
        let seg=graph.segment(node.location).ok_or(Error::Invalid("JBIG2 page segment"))?;
        if seg.kind==50{
            let body=page.get(seg.payload.clone()).ok_or(Error::Invalid("JBIG2 stripe span"))?;
            let end=stripe_end(body)?;
            if end<=stripe_start||end>height||end-stripe_start>u32::from(info.max_stripe_size)||stripe_written_end>end{
                return Err(Error::Invalid("JBIG2 stripe end row"));
            }
            stripe_start=end;stripe_written_end=end;
            continue;
        }
        if !matches!(seg.kind,16|4|6|7|20|22|23|36|38|39|40|42|43){continue;}
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
            if reclaim&&!uses.contains_key(&seg.number){
                stats.released_pattern_dictionaries+=1;stats.released_pattern_bytes+=d.packed_bytes;
                stats.retained_packed_bytes-=d.packed_bytes;
            }else{patterns.insert(seg.number,d);}
            continue;
        }
        let generic_data=if matches!(seg.kind,36|38|39){
            let unknown=stream.get(seg.header.end-4..seg.header.end)==Some(&[255u8;4][..]);
            Some(generic::parse_segment(data,unknown)?)
        }else{None};
        let region=if let Some((h,_))=&generic_data{h.region}else{bitmap::parse_region(data)?.0};
        // Only immediate regions write the page. Check the effective
        // clipped vertical interval before entropy execution.
        if recovered_height&&!matches!(seg.kind,4|20|36|40)&&
            (region.y>=height||region.y.saturating_add(region.height)>height){
            return Err(Error::Invalid("JBIG2 region exceeds recovered height"));
        }
        if info.striped&&!matches!(seg.kind,4|20|36|40)&&region.x<info.width&&region.y<height{
            let end=region.y.saturating_add(region.height).min(height);
            if region.y<stripe_start||end-stripe_start>u32::from(info.max_stripe_size){
                return Err(Error::Invalid("JBIG2 region crosses stripe boundary"));
            }
            stripe_written_end=stripe_written_end.max(end);
        }
        let pixels=u64::from(region.width)*u64::from(region.height);
        if pixels>limits.region_pixels-stats.region_pixels{return Err(Error::Limit("JBIG2 page region pixels"));}
        let image=match seg.kind{
            4|6|7=>{
                let d=text_region::execute_text_segment_inspected(globals,page,&graph,node,&dictionaries,
                    text_region::TextSegmentLimits{dictionaries:dl,work:work+ds.work_used,
                        packed_bytes:available+ds.packed_bytes,symbol_slots:limits.text_slots-stats.text_slots,
                        decisions,huffman_bits:limits.huffman_bits-stats.huffman_bits},0,
                        limits.region_pixels-stats.region_pixels,limits.text_instances-stats.text_instances)?;
                charge(&mut stats,limits.work,d.work_used-ds.work_used)?;
                stats.text_regions+=1;stats.text_slots+=d.symbol_slots;stats.text_instances+=u64::from(d.text.stats.instances);
                stats.huffman_bits+=d.huffman_bits_used;stats.decisions+=d.decisions_used;d.text.bitmap
            },
            20|22|23=>{
                let dep=graph.segment(node.dependencies[0]).ok_or(Error::Invalid("JBIG2 page halftone reference"))?;
                let dict=patterns.get(&dep.number).ok_or(Error::Invalid("JBIG2 page pattern dictionary"))?;
                let d=halftone::decode(data,dict,halftone::Limits{grid_cells:limits.grid_cells-stats.grid_cells,
                    pixels:limits.region_pixels-stats.region_pixels,packed_bytes:available,decisions,work})?;
                charge(&mut stats,limits.work,d.work_used)?;
                stats.halftone_regions+=1;stats.decisions+=d.decisions;
                stats.grid_cells+=u64::from(d.header.grid.width)*u64::from(d.header.grid.height);d.bitmap
            },
            36|38|39=>{
                let (h,payload)=generic_data.ok_or(Error::Invalid("JBIG2 page generic header"))?;
                let len=packed(region.width,region.height);
                // MMR conversion holds source and destination at once.
                let (scratch,overhead,bound)=match &h.coding{
                    generic::Coding::Mmr=>(len*2,0,None),
                    generic::Coding::Arithmetic{neighborhood,typical_prediction}=>{
                        let rows=if *typical_prediction{u64::from(region.height)}else{0};
                        let upper=pixels+rows;
                        if upper>decisions as u64{return Err(Error::Limit("JBIG2 page generic decisions bound"));}
                        (len,neighborhood.template().context_count() as u64,
                            Some(pixels*(neighborhood.template().bits() as u64+1)+rows))
                    },
                };
                if scratch>available{return Err(Error::Limit("JBIG2 page generic packed budget"));}
                let remaining=work.checked_sub(overhead).ok_or(Error::Limit("JBIG2 page generic work"))?;
                let d=generic::decode_region_body(h,payload,remaining)?;
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
            40|42|43=>{
                let len=packed(region.width,region.height);
                let d=if let Some(&loc)=node.dependencies.first(){
                    let dep=graph.segment(loc).ok_or(Error::Invalid("JBIG2 refinement dependency"))?;
                    // The old auxiliary remains live during decode: target
                    // must fit in additional packed budget.
                    if len>available{return Err(Error::Limit("JBIG2 auxiliary refinement packed budget"));}
                    let (r,src)=auxiliary.get(&dep.number).ok_or(Error::Invalid("JBIG2 consumed auxiliary buffer"))?;
                    let d=refinement::decode_segment(seg,stream,
                        refinement::RegionReference::Auxiliary{segment:dep,region:*r,bitmap:src},
                        work,decisions)?;
                    let (_,old)=auxiliary.remove(&dep.number).ok_or(Error::Invalid("JBIG2 auxiliary removal"))?;
                    let released=old.packed().len();
                    auxiliary_bytes-=released;stats.retained_packed_bytes-=released;
                    stats.auxiliary_consumed+=1;
                    d
                }else{
                    // Current-page crop and target coexist. Enforce caller
                    // cap, not just the lower-level global hard limit.
                    if len*2>available{return Err(Error::Limit("JBIG2 page refinement packed budget"));}
                    refinement::decode_segment(seg,stream,
                        refinement::RegionReference::Page{number:info.page,bitmap:&canvas},work,decisions)?
                };
                charge(&mut stats,limits.work,d.stats.work_bound)?;
                stats.decisions+=d.stats.decisions;stats.refinement_regions+=1;
                d.bitmap
            },
            _=>return Err(Error::Unsupported("JBIG2 page region kind")),
        };
        if let Some(state)=reclaimer.as_mut(){
            if matches!(seg.kind,4|6|7|20|22|23){
                for &loc in &node.dependencies{
                    let dep=graph.segment(loc).ok_or(Error::Invalid("JBIG2 reclaim dependency"))?;
                    if !matches!(dep.kind,0|16){continue;}
                    let count=uses.get_mut(&dep.number).ok_or(Error::Invalid("JBIG2 reclaim missing use"))?;
                    *count=count.checked_sub(1).ok_or(Error::Invalid("JBIG2 reclaim use underflow"))?;
                    if *count==0{
                        uses.remove(&dep.number);
                        if dep.kind==0{
                            let freed=dictionaries.release_page_dictionary(dep.number,state)?;
                            record_release(&mut stats,freed);
                        }else{
                            let old=patterns.remove(&dep.number).ok_or(Error::Invalid("JBIG2 reclaim pattern"))?;
                            stats.released_pattern_dictionaries+=1;stats.released_pattern_bytes+=old.packed_bytes;
                            stats.retained_packed_bytes-=old.packed_bytes;
                        }
                    }
                }
            }
        }
        stats.region_pixels+=pixels;
        if matches!(seg.kind,4|20|36|40){
            let bytes=image.packed().len();
            auxiliary_bytes+=bytes;stats.retained_packed_bytes+=bytes;
            stats.peak_auxiliary_bytes=stats.peak_auxiliary_bytes.max(auxiliary_bytes);
            stats.intermediate_regions+=1;
            auxiliary.insert(seg.number,(region,image));
            continue;
        }
        let left=limits.work-stats.work_used;
        let mut budget=ComposeBudget::new(left)?;
        canvas.compose_region(&image,region,&mut budget)?;
        let used=left-budget.remaining();
        charge(&mut stats,limits.work,used)?;stats.composed_pixels+=used;
    }
    // Known-height embedded streams may omit a final stripe marker;
    // any remaining implicit stripe must still fit the declared maximum.
    if info.striped&&height-stripe_start>u32::from(info.max_stripe_size){
        return Err(Error::Invalid("JBIG2 unfinished stripe"));
    }
    if !auxiliary.is_empty(){return Err(Error::Invalid("JBIG2 unconsumed auxiliary buffers"));}
    if reclaim&&(!uses.is_empty()||!patterns.is_empty()||stats.retained_packed_bytes!=page_bytes){
        return Err(Error::Invalid("JBIG2 page reclamation incomplete"));
    }
    Ok(DecodedPage{info,bitmap:canvas,stats})
}

/// Strict opt-in lifetime validation before native page decoding. Both
/// passes share one work cap. General dictionary allocation is still
/// conservative; this audit does not pretend to physically evict them.
#[derive(Debug,PartialEq,Eq)]
pub struct AuditedPage{
    pub decoded:DecodedPage,pub retention:super::retention::Audit,pub work_used:u64,
}
pub fn decode_pdf_page_audited(globals:&[u8],page:&[u8],mut limits:Limits)->Result<AuditedPage>{
    let retention=super::retention::audit_pdf_streams(globals,page,limits.work)?;
    limits.work=limits.work.checked_sub(retention.work_used).ok_or(Error::Limit("JBIG2 audited page work"))?;
    let decoded=decode_pdf_page(globals,page,limits)?;
    let work_used=retention.work_used+decoded.stats.work_used;
    Ok(AuditedPage{decoded,retention,work_used})
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
    fn pdf_image_supported_lifecycle_and_striped_boundary(){
        let l=Limits::default();
        let mut p=info(8,4,0);
        p.extend(seg(2,39,&[],&white(8,4,0,0,0)));
        let a=decode_pdf_image_compatible(&[],&p,l).unwrap();
        assert_eq!(a.bitmap.packed(),[0;4]);
        let mut ended=p.clone();
        ended.extend(seg(3,49,&[],&[]));
        assert_eq!(decode_pdf_image_compatible(&[],&ended,l).unwrap().bitmap,a.bitmap);
        for height in [4,u32::MAX]{
            let mut stripe=info(8,height,0);
            stripe[28..30].copy_from_slice(&0x8004u16.to_be_bytes());
            stripe.extend(seg(2,39,&[],&white(8,4,0,0,0)));
            stripe.extend(seg(3,50,&[],&3u32.to_be_bytes()));
            assert!(matches!(decode_pdf_page(&[],&stripe,l),Err(Error::Unsupported("JBIG2 PDF striped page"))));
            let d=decode_pdf_image_compatible(&[],&stripe,l).unwrap();
            assert_eq!(d.bitmap,a.bitmap);
            assert_eq!(d.info.height,Some(4));
        }
        let mut unknown=info(8,u32::MAX,0);
        unknown.extend(seg(2,39,&[],&white(8,4,0,0,0)));
        assert!(matches!(decode_pdf_image_compatible(&[],&unknown,l),
            Err(Error::Invalid("JBIG2 unknown unstriped height"))));
        let mut stripe_end=p;
        stripe_end.extend(seg(3,50,&[],&3u32.to_be_bytes()));
        assert!(matches!(decode_pdf_image_compatible(&[],&stripe_end,l),
            Err(Error::Unsupported("JBIG2 PDF stripe end"))));
    }
    fn striped_info(w:u32,h:u32,flags:u8,max:u16)->Vec<u8>{
        let mut p=info(w,h,flags);p[28..30].copy_from_slice(&(0x8000|max).to_be_bytes());p
    }
    #[test]
    fn pdf_image_known_height_stripes_absolute_coordinates(){
        let l=Limits::default();
        let mut p=striped_info(8,4,68,2);
        p.extend(seg(2,39,&[],&white(8,2,0,0,0)));
        p.extend(seg(3,50,&[],&1u32.to_be_bytes()));
        p.extend(seg(4,39,&[],&white(8,2,0,2,4)));
        p.extend(seg(5,50,&[],&3u32.to_be_bytes()));
        let d=decode_pdf_image_compatible(&[],&p,l).unwrap();
        assert_eq!(d.bitmap.packed(),[255,255,0,0]);
        assert_eq!(d.stats.region_pixels,32);
        assert_eq!(d.stats.retained_packed_bytes,4);
        assert!(decode_pdf_image_compatible(&[],&p,Limits{page_pixels:31,..l}).is_err());
        assert!(decode_pdf_image_compatible(&[],&p,Limits{work:d.stats.work_used-1,..l}).is_err());
        p.truncate(p.len()-15);
        assert_eq!(decode_pdf_image_compatible(&[],&p,l).unwrap().bitmap,d.bitmap);
    }
    #[test]
    fn pdf_image_unknown_height_stripes_recover_exact_pixels(){
        let l=Limits::default();
        let mut p=striped_info(8,u32::MAX,68,2);
        p.extend(seg(2,39,&[],&white(8,2,0,0,0)));
        p.extend(seg(3,50,&[],&1u32.to_be_bytes()));
        p.extend(seg(4,39,&[],&white(8,2,0,2,4)));
        p.extend(seg(5,50,&[],&3u32.to_be_bytes()));
        let d=decode_pdf_image_compatible(&[],&p,l).unwrap();
        assert_eq!(d.info.height,Some(4));
        assert_eq!(d.bitmap.packed(),[255,255,0,0]);
        assert_eq!(d.stats.region_pixels,32);
        assert_eq!(d.stats.retained_packed_bytes,4);
        let mut known=p.clone();known[15..19].copy_from_slice(&4u32.to_be_bytes());
        assert_eq!(decode_pdf_image_compatible(&[],&known,l).unwrap().bitmap,d.bitmap);
        assert!(decode_pdf_image_compatible(&[],&p,Limits{page_pixels:31,..l}).is_err());
        assert!(decode_pdf_image_compatible(&[],&p,Limits{packed_bytes:3,..l}).is_err());
        assert!(decode_pdf_image_compatible(&[],&p,Limits{work:d.stats.work_used-1,..l}).is_err());
        p.extend(seg(6,49,&[],&[]));
        assert_eq!(decode_pdf_image_compatible(&[],&p,l).unwrap().bitmap,d.bitmap);
    }
    #[test]
    fn pdf_image_unknown_height_requires_bounded_final_eos(){
        let l=Limits::default();let p=striped_info(8,u32::MAX,0,2);
        assert!(decode_pdf_image_compatible(&[],&p,l).is_err());
        for row in [2u32,u32::MAX]{
            let mut bad=p.clone();bad.extend(seg(2,50,&[],&row.to_be_bytes()));
            assert!(decode_pdf_image_compatible(&[],&bad,l).is_err());
        }
        let mut incomplete=p.clone();incomplete.extend(seg(2,50,&[],&1u32.to_be_bytes()));
        incomplete.extend(seg(3,39,&[],&white(8,2,0,2,0)));
        assert!(decode_pdf_image_compatible(&[],&incomplete,l).is_err());
        let mut duplicate=p.clone();duplicate.extend(seg(2,50,&[],&1u32.to_be_bytes()));
        duplicate.extend(seg(3,50,&[],&1u32.to_be_bytes()));
        assert!(decode_pdf_image_compatible(&[],&duplicate,l).is_err());
        let mut oversized=p;oversized.extend(seg(2,39,&[],&white(8,3,0,0,0)));
        oversized.extend(seg(3,50,&[],&1u32.to_be_bytes()));
        assert!(decode_pdf_image_compatible(&[],&oversized,l).is_err());
    }
    #[test]
    fn pdf_image_stripes_reject_invalid_extent_and_revisit(){
        let l=Limits::default();
        let p=striped_info(8,4,0,2);
        for row in [2u32,4,u32::MAX]{
            let mut bad=p.clone();bad.extend(seg(2,50,&[],&row.to_be_bytes()));
            assert!(decode_pdf_image_compatible(&[],&bad,l).is_err());
        }
        let mut short=p.clone();short.extend(seg(2,50,&[],&[0;3]));
        assert!(decode_pdf_image_compatible(&[],&short,l).is_err());
        let mut refs=p.clone();refs.extend(seg(2,50,&[1],&1u32.to_be_bytes()));
        assert!(decode_pdf_image_compatible(&[],&refs,l).is_err());
        let mut over=p.clone();over.extend(seg(2,39,&[],&white(8,3,0,0,0)));
        assert!(decode_pdf_image_compatible(&[],&over,l).is_err());
        let mut early=p.clone();early.extend(seg(2,39,&[],&white(8,2,0,0,0)));
        early.extend(seg(3,50,&[],&0u32.to_be_bytes()));
        assert!(decode_pdf_image_compatible(&[],&early,l).is_err());
        let mut revisit=p.clone();revisit.extend(seg(2,50,&[],&1u32.to_be_bytes()));
        let mut duplicate=revisit.clone();duplicate.extend(seg(3,50,&[],&1u32.to_be_bytes()));
        assert!(decode_pdf_image_compatible(&[],&duplicate,l).is_err());
        revisit.extend(seg(3,39,&[],&white(8,2,0,1,0)));
        assert!(decode_pdf_image_compatible(&[],&revisit,l).is_err());
        assert!(decode_pdf_image_compatible(&[],&p,l).is_err());
        assert!(decode_pdf_image_compatible(&[],&striped_info(8,4,0,0),l).is_err());
    }
    #[test]
    fn pdf_image_multiple_pages_and_unsupported_extensions_fail_closed(){
        let l=Limits::default();
        let mut p=info(8,4,0);
        p.extend(seg(2,39,&[],&white(8,4,0,0,0)));
        let mut second=info(8,4,0);
        second[3]=4;second[6]=2;
        let mut multi=p.clone();
        multi.extend(seg(3,49,&[],&[]));multi.extend(second);
        assert!(matches!(decode_pdf_image_compatible(&[],&multi,l),
            Err(Error::Invalid("JBIG2 PDF single page required"))));
        let mut unsupported=p.clone();
        unsupported.extend(seg(3,52,&[],&[]));
        assert!(matches!(decode_pdf_image_compatible(&[],&unsupported,l),
            Err(Error::Unsupported("JBIG2 page segment execution kind"))));
        let mut short=p.clone();short.extend(seg(3,62,&[],&[]));
        assert!(matches!(decode_pdf_image_compatible(&[],&short,l),
            Err(Error::Invalid("JBIG2 extension header"))));
        let mut closed=p;closed.extend(seg(3,49,&[],&[]));
        closed.extend(seg(4,39,&[],&white(8,4,0,0,0)));
        assert!(matches!(decode_pdf_image_compatible(&[],&closed,l),
            Err(Error::Invalid("JBIG2 PDF segment after page end"))));
    }
    #[test]
    fn pdf_image_optional_extensions_preserve_pixels_and_dependencies(){
        let l=Limits::default();
        let mut p=info(8,4,4);
        p.extend(seg(2,39,&[],&white(8,4,0,0,0)));
        let a=decode_pdf_image_compatible(&[],&p,l).unwrap();
        let mut ext=p.clone();
        ext.extend(seg(3,62,&[],&[0x20,0,0,0,b'A',0,b'B',0,0]));
        ext.extend(seg(4,62,&[1,2],&[0x20,0,0,2,0,65,0,0,0,66,0,0,0,0]));
        ext.extend(seg(5,62,&[4],&[0x40,0,0,1,0xff]));
        let b=decode_pdf_image_compatible(&[],&ext,l).unwrap();
        assert_eq!(a.bitmap,b.bitmap);assert_eq!(a.stats.region_pixels,b.stats.region_pixels);
        assert_eq!(a.stats.decisions,b.stats.decisions);
        assert!(b.stats.work_used>a.stats.work_used);
        assert!(decode_pdf_image_compatible(&[],&ext,Limits{work:b.stats.work_used-1,..l}).is_err());
        assert!(decode_pdf_page(&[],&ext,l).is_err());
        let mut consume=ext;consume.extend(seg(6,39,&[5],&white(8,4,0,0,0)));
        assert!(matches!(decode_pdf_image_compatible(&[],&consume,l),
            Err(Error::Invalid("JBIG2 PDF reference to ignored extension"))));
    }
    #[test]
    fn pdf_image_extension_rejects_necessary_and_bad_references(){
        let l=Limits::default();let p=info(8,4,0);
        for (body,error) in [([0xa0,0,0,0],Error::Unsupported("JBIG2 necessary extension")),
                            ([0x80,0,0,0],Error::Invalid("JBIG2 extension necessary flags"))]{
            let mut bad=p.clone();bad.extend(seg(2,62,&[],&body));
            assert_eq!(decode_pdf_image_compatible(&[],&bad,l).unwrap_err(),error);
        }
        for n in 0..4{
            let mut bad=p.clone();bad.extend(seg(2,62,&[],&[0;4][..n]));
            assert!(matches!(decode_pdf_image_compatible(&[],&bad,l),Err(Error::Invalid("JBIG2 extension header"))));
        }
        let mut dangling=p.clone();dangling.extend(seg(3,62,&[2],&[0;4]));
        assert!(matches!(decode_pdf_image_compatible(&[],&dangling,l),
            Err(Error::Invalid("JBIG2 PDF unresolved reference"))));
        let mut after=p;after.extend(seg(2,49,&[],&[]));after.extend(seg(3,62,&[1],&[0;4]));
        assert!(matches!(decode_pdf_image_compatible(&[],&after,l),
            Err(Error::Invalid("JBIG2 PDF segment after page end"))));
    }
    #[test]
    fn page_indeterminate_arithmetic_generic_rows_and_budgets(){
        let mut body=Vec::new();for n in [13u32,8,0,0]{body.extend(n.to_be_bytes());}
        body.extend([0,0,3,255,253,255,2,254,254,254]);
        body.extend([0xc8,0xbd,0xda,0xa7,0xe0,0x69,0x6a,0x8f,0x21,0xc2,0x15,0xfe,0xff,0x7f,0xff,0xac]);
        let mut known=info(13,8,0);known.extend(seg(2,38,&[],&body));
        known.extend(seg(3,49,&[],&[]));
        let l=Limits::default();
        let a=decode_pdf_image_compatible(&[],&known,l).unwrap();
        body[4..8].copy_from_slice(&10u32.to_be_bytes());body.extend(8u32.to_be_bytes());
        let mut unknown=seg(2,38,&[],&body);unknown[7..11].copy_from_slice(&u32::MAX.to_be_bytes());
        let mut p=info(13,8,0);p.extend(&unknown);p.extend(seg(3,49,&[],&[]));
        let b=decode_pdf_image_compatible(&[],&p,l).unwrap();
        assert_eq!(a.bitmap,b.bitmap);assert_eq!(b.stats.region_pixels,104);
        assert_eq!(b.stats.decisions,104);
        assert_eq!(b.bitmap.packed(),[0x91,0x30,0xe0,0xe0,0,0,0xe0,0xe0,0x91,0x30,0x4a,0x50,0xaa,0xa8,0x5b,0x48]);
        assert!(decode_pdf_image_compatible(&[],&p,Limits{region_pixels:104,decisions:104,..l}).is_ok());
        assert!(matches!(decode_pdf_image_compatible(&[],&p,Limits{region_pixels:103,..l}),Err(Error::Limit(_))));
        assert!(matches!(decode_pdf_image_compatible(&[],&p,Limits{decisions:103,..l}),Err(Error::Limit(_))));
        let mut wrong=info(13,8,0);unknown[4]=39;wrong.extend(unknown);
        assert!(matches!(decode_pdf_image_compatible(&[],&wrong,l),Err(Error::Unsupported("JBIG2 indeterminate segment length"))));
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

    fn refine_body(w:u32,h:u32,x:u32,y:u32,op:u8,flags:u8,at:&[u8],entropy:&[u8])->Vec<u8>{
        let mut b=Vec::new();for n in [w,h,x,y]{b.extend(n.to_be_bytes());}
        b.extend([op,flags]);b.extend(at);b.extend(entropy);b
    }
    fn source_region(x:u32,y:u32)->Vec<u8>{
        // Independently encoded MMR rows [0,124,124,124,0,0] at width 7.
        refine_body(7,6,x,y,0,1,&[],&[145,207,248,224])
    }
    #[test]
    fn page_auxiliary_refinement_exact_pixels_and_budget(){
        let mut p=info(7,6,0x22);
        let src=source_region(0,0);
        assert_eq!(generic::decode_region(&src,10000).unwrap().bitmap.packed(),[0,124,124,124,0,0]);
        p.extend(seg(2,36,&[],&src));
        let r=refine_body(7,6,0,0,0,0,&[255;4],&[192,34,140,221,147,255,172]);
        p.extend(seg(3,43,&[2],&r));
        let d=decode_pdf_page(&[],&p,Limits::default()).unwrap();
        assert_eq!(d.bitmap.packed(),[170,56,56,198,170,84]);
        assert_eq!((d.stats.generic_regions,d.stats.refinement_regions,d.stats.intermediate_regions,
            d.stats.auxiliary_consumed,d.stats.peak_auxiliary_bytes,d.stats.retained_packed_bytes),(1,1,1,1,6,6));
        assert_eq!((d.stats.region_pixels,d.stats.composed_pixels,d.stats.decisions),(84,42,42));
        let exact=Limits{packed_bytes:18,work:d.stats.work_used,decisions:42,region_pixels:84,regions:2,..Limits::default()};
        assert_eq!(decode_pdf_page(&[],&p,exact).unwrap(),d);
        for cap in [Limits{packed_bytes:17,..exact},Limits{work:exact.work-1,..exact},
            Limits{decisions:41,..exact},Limits{region_pixels:83,..exact},Limits{regions:1,..exact}]{
            assert!(decode_pdf_page(&[],&p,cap).is_err());
        }
        let mut duplicate=p.clone();duplicate.extend(seg(4,43,&[2],&r));
        assert!(decode_pdf_page(&[],&duplicate,Limits::default()).is_err());
        let mut mismatch=info(7,6,0x62);mismatch.extend(seg(2,36,&[],&src));
        let mut bad=r.clone();bad[16]=2;mismatch.extend(seg(3,43,&[2],&bad));
        assert!(decode_pdf_page(&[],&mismatch,Limits::default()).is_err());
    }
    #[test]
    fn page_auxiliary_chain_transfers_and_final_consumption(){
        let mut p=info(9,1,0x26); // black page; white auxiliary is not drawn until final refinement
        p.extend(seg(2,36,&[],&white(9,1,0,0,0)));
        let r=refine_body(9,1,0,0,0,3,&[],&[255,172]);
        p.extend(seg(3,40,&[2],&r));
        assert!(decode_pdf_page(&[],&p,Limits::default()).is_err()); // dangling
        let mut stale=p.clone();stale.extend(seg(4,42,&[2],&r));
        assert!(decode_pdf_page(&[],&stale,Limits::default()).is_err());
        p.extend(seg(4,42,&[3],&r));
        let d=decode_pdf_page(&[],&p,Limits::default()).unwrap();
        assert_eq!(d.bitmap.packed(),[255,128]); // OR with white
        assert_eq!((d.stats.refinement_regions,d.stats.intermediate_regions,d.stats.auxiliary_consumed,
            d.stats.peak_auxiliary_bytes,d.stats.decisions,d.stats.composed_pixels),(2,2,2,2,2,9));
        for flags in [0x06,0x24]{
            let mut bad=p.clone();bad[27]=flags;
            assert!(decode_pdf_page(&[],&bad,Limits::default()).is_err());
        }
    }
    #[test]
    fn pdf_image_missing_resource_hints_preserves_pixels_and_limits(){
        let l=Limits::default();
        let mut p=info(7,6,0x22);
        p.extend(seg(2,36,&[],&source_region(0,0)));
        let r=refine_body(7,6,0,0,0,0,&[255;4],&[192,34,140,221,147,255,172]);
        p.extend(seg(3,43,&[2],&r));
        let strict=decode_pdf_page_reclaimed(&[],&p,l).unwrap();
        for flags in [0,2,32]{
            let mut sample=p.clone();sample[27]=flags;
            assert!(decode_pdf_page(&[],&sample,l).is_err());
            assert!(decode_pdf_page_reclaimed(&[],&sample,l).is_err());
            let d=decode_pdf_image_compatible(&[],&sample,l).unwrap();
            assert_eq!(d.bitmap,strict.bitmap);
            assert_eq!(d.stats,strict.stats);
            assert_eq!(d.info.flags,flags);
            assert_eq!(d.bitmap.packed(),[170,56,56,198,170,84]);
            assert!(decode_pdf_image_compatible(&[],&sample,Limits{packed_bytes:17,..l}).is_err());
            assert!(decode_pdf_image_compatible(&[],&sample,Limits{work:d.stats.work_used-1,..l}).is_err());
        }
    }
    #[test]
    fn pdf_image_missing_resource_hints_does_not_relax_graph_or_operator(){
        let l=Limits::default();
        let mut p=info(7,6,0);
        p.extend(seg(2,36,&[],&source_region(0,0)));
        assert!(decode_pdf_image_compatible(&[],&p,l).is_err()); // dangling
        let r=refine_body(7,6,0,0,0,0,&[255;4],&[192,34,140,221,147,255,172]);
        let mut wrong=p.clone();wrong.extend(seg(3,43,&[1],&r));
        assert!(decode_pdf_image_compatible(&[],&wrong,l).is_err());
        let mut op=r.clone();op[16]=2;
        let mut bad=p.clone();bad.extend(seg(3,43,&[2],&op));
        assert!(decode_pdf_image_compatible(&[],&bad,l).is_err());
        p.extend(seg(3,43,&[2],&r));
        p.extend(seg(4,43,&[2],&r));
        assert!(decode_pdf_image_compatible(&[],&p,l).is_err()); // stale
        let mut reserved=info(7,6,128);
        reserved.extend(seg(2,39,&[],&source_region(0,0)));
        assert!(decode_pdf_image_compatible(&[],&reserved,l).is_err());
    }
    #[test]
    fn page_current_crop_refinement_order_and_budget(){
        let mut p=info(13,11,2); // no operator override; page reference REPLACE is mandatory regardless
        p.extend(seg(2,39,&[],&source_region(3,2)));
        let r=refine_body(7,6,3,2,4,2,&[255,0,0,0],&[62,159,39,101,31,255,172]);
        p.extend(seg(3,43,&[],&r));
        let d=decode_pdf_page(&[],&p,Limits::default()).unwrap();
        let mut expected=vec![0u8;22];
        for (y,row) in [170u8,170,254,170,84,0].into_iter().enumerate(){
            for x in 0..7{if row&(128>>x)!=0{
                let px=x+3;expected[(y+2)*2+px/8]|=128>>(px%8);
            }}
        }
        assert_eq!(d.bitmap.packed(),expected);assert_eq!(d.stats.decisions,38);
        assert_eq!((d.stats.refinement_regions,d.stats.auxiliary_consumed,d.stats.composed_pixels),(1,0,84));
        let exact=Limits{packed_bytes:34,decisions:38,work:d.stats.work_used,..Limits::default()};
        assert_eq!(decode_pdf_page(&[],&p,exact).unwrap(),d);
        assert!(decode_pdf_page(&[],&p,Limits{packed_bytes:33,..exact}).is_err());
        assert!(decode_pdf_page(&[],&p,Limits{decisions:37,..exact}).is_err());
        let mut after=p.clone();after.extend(seg(4,39,&[],&source_region(3,2)));
        assert_ne!(decode_pdf_page(&[],&after,Limits::default()).unwrap().bitmap,d.bitmap);
        let mut bad=info(13,11,2);let mut r=r;r[11]=12;bad.extend(seg(2,43,&[],&r));
        assert!(decode_pdf_page(&[],&bad,Limits::default()).is_err());
    }

    #[test]
    fn page_mixed_intermediate_sources_and_retention_audit(){
        let mut p=info(27,1,0x66);
        let mut text=Vec::new();for n in [9u32,1,0,0]{text.extend(n.to_be_bytes());}
        text.push(4);text.extend(0x11u16.to_be_bytes());text.extend([0,0]);
        text.extend([0;4]);text.extend([0;19]);
        let mut a=seg(2,4,&[],&text);a[5]|=1;p.extend(a);
        let mut dict=seg(3,16,&[],&[1,1,1,0,0,0,0,128]);dict[5]|=1;p.extend(dict);
        let mut half=Vec::new();for n in [9u32,1,9,0]{half.extend(n.to_be_bytes());}
        half.extend([4,1]);for n in [9u32,1,0,0]{half.extend(n.to_be_bytes());}
        half.extend([1,0,0,0]);
        let mut a=seg(4,20,&[3],&half);a[5]|=1;p.extend(a);
        let mut a=seg(5,36,&[],&white(9,1,18,0,4));a[5]|=1;p.extend(a);
        for (id,dep,x) in [(6,2,0),(7,4,9),(8,5,18)]{
            let r=refine_body(9,1,x,0,4,3,&[],&[255,172]);
            p.extend(seg(id,43,&[dep],&r));
        }
        let d=decode_pdf_page_audited(&[],&p,Limits::default()).unwrap();
        assert_eq!(d.decoded.bitmap.packed(),[0;4]);
        let s=d.decoded.stats;
        assert_eq!((s.text_regions,s.halftone_regions,s.generic_regions,s.refinement_regions),(1,1,1,3));
        assert_eq!((s.intermediate_regions,s.auxiliary_consumed,s.peak_auxiliary_bytes,s.composed_pixels,s.decisions),(3,3,6,27,3));
        assert_eq!((d.retention.retained,d.retention.released,d.retention.live_page_at_end),(4,4,0));
        assert_eq!(decode_pdf_page_audited(&[],&p,Limits{work:d.work_used,..Limits::default()}).unwrap(),d);
        assert!(decode_pdf_page_audited(&[],&p,Limits{work:d.work_used-1,..Limits::default()}).is_err());
        let mut bad=p;bad[35]&=0xfe; // segment 2's retain-self flag
        assert!(decode_pdf_page_audited(&[],&bad,Limits::default()).is_err());
        assert_eq!(decode_pdf_page(&[],&bad,Limits::default()).unwrap().bitmap.packed(),[0;4]);
    }

    #[test]
    fn page_reclaimed_pattern_peak_and_unused_dictionary(){
        let mut p=info(32,36,0);
        p.extend(seg(2,16,&[],PATTERN));p.extend(seg(3,23,&[2],HALF));
        p.extend(seg(4,16,&[],PATTERN));p.extend(seg(5,23,&[4],HALF));
        p.extend(seg(6,16,&[],PATTERN)); // unused, still decoded
        let old=decode_pdf_page(&[],&p,Limits::default()).unwrap();
        let d=decode_pdf_page_reclaimed(&[],&p,Limits::default()).unwrap();
        assert_eq!(d.bitmap,old.bitmap);
        assert_eq!((d.stats.released_pattern_dictionaries,d.stats.released_pattern_bytes,d.stats.retained_packed_bytes),(3,192,144));
        // 144 page + 64 dictionary + 297 active region, no earlier dictionary.
        let exact=Limits{packed_bytes:505,work:d.stats.work_used,..Limits::default()};
        assert_eq!(decode_pdf_page_reclaimed(&[],&p,exact).unwrap(),d);
        assert!(decode_pdf_page(&[],&p,exact).is_err());
        assert!(decode_pdf_page_reclaimed(&[],&p,Limits{packed_bytes:504,..exact}).is_err());
        assert!(decode_pdf_page_reclaimed(&[],&p,Limits{work:exact.work-1,..exact}).is_err());
        let mut bad=p;bad.pop();assert!(decode_pdf_page_reclaimed(&[],&bad,Limits::default()).is_err());
    }
    #[test]
    fn page_unsupported_and_bad_dependencies_fail_closed(){
        let l=Limits::default();
        for kind in [4,20,36,40,42,43,52,62]{
            let mut p=info(8,1,0);p.extend(seg(2,kind,&[],&[]));
            assert!(decode_pdf_page(&[],&p,l).is_err());
        }
        // Auxiliary permission alone does not require allocating a buffer.
        assert!(decode_pdf_page(&[],&info(8,1,32),l).is_ok());
        assert!(decode_pdf_page(&[],&info(8,1,128),l).is_err());
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
    fn annex_h_original_full_page_retention_audit(){
        for (g,p,refs,globals) in [(GLOBAL1,PAGE1,3,1),(GLOBAL1,PAGE2,3,0),(GLOBAL3,PAGE3,2,0)]{
            let l=Limits::default();let d=decode_pdf_page_audited(g,p,l).unwrap();
            assert_eq!(d.decoded,decode_pdf_page(g,p,l).unwrap());
            assert_eq!((d.retention.references,d.retention.live_globals_at_end,d.retention.live_page_at_end),(refs,globals,0));
        }
    }

    #[test]
    fn annex_h_reclaimed_page_pixels_and_globals_isolation(){
        for (g,p) in [(GLOBAL1,PAGE1),(GLOBAL1,PAGE2),(GLOBAL3,PAGE3),(GLOBAL1,PAGE1)]{
            let l=Limits::default();
            let old=decode_pdf_page(g,p,l).unwrap();
            let d=decode_pdf_page_reclaimed(g,p,l).unwrap();
            assert_eq!(d.bitmap,old.bitmap);assert_eq!(d.info,old.info);
            assert_eq!(d.stats.retained_packed_bytes,d.bitmap.packed().len());
            assert_eq!(d.stats.released_symbol_bytes,old.stats.dictionaries.packed_bytes);
            assert_eq!(d.stats.released_symbol_bitmaps,old.stats.dictionaries.new_symbols as usize);
            assert_eq!(d.stats.released_symbol_dictionaries,old.stats.dictionaries.dictionaries as usize);
            assert_eq!(d.stats.released_pattern_dictionaries,old.stats.pattern_dictionaries);
        }
        assert!(decode_pdf_page_reclaimed(GLOBAL3,PAGE1,Limits::default()).is_err());
        assert!(decode_pdf_page_reclaimed(&[],PAGE2,Limits::default()).is_err());
    }
    // Deterministic mutation regression, not a substitute for long-running
    // coverage-guided fuzzing. Keep caps small even if a header is corrupted.
    fn mutation_limits()->Limits{
        Limits{page_pixels:4096,region_pixels:8192,regions:32,
            packed_bytes:32768,work:200000,decisions:32768,
            text_slots:128,text_instances:256,huffman_bits:32768,
            patterns:64,pattern_pixels:8192,grid_cells:256,..Limits::default()}
    }
    fn check_mutated_page(g:&[u8],p:&[u8],l:Limits)->bool{
        // The test intentionally does not catch panics: a decoder panic
        // must fail CI. Repeat both success and rejection for determinism.
        let a=decode_pdf_image_compatible(g,p,l);
        let b=decode_pdf_image_compatible(g,p,l);
        match (a,b){
            (Ok(a),Ok(b))=>{
                assert_eq!(a,b);
                let w=a.bitmap.width();let h=a.bitmap.height();
                assert_eq!(a.info.width,w);assert_eq!(a.info.height,Some(h));
                assert!(w>0&&h>0&&u64::from(w)*u64::from(h)<=l.page_pixels);
                assert_eq!(a.bitmap.packed().len(),packed(w,h));
                assert_eq!(a.stats.retained_packed_bytes,a.bitmap.packed().len());
                assert!(a.stats.retained_packed_bytes<=l.packed_bytes);
                assert!(a.stats.work_used<=l.work);
                assert!(a.stats.region_pixels<=l.region_pixels);
                assert!(a.stats.decisions<=l.decisions);
                assert!(a.stats.huffman_bits<=l.huffman_bits);
                true
            }
            (Err(a),Err(b))=>{assert_eq!(format!("{a:?}"),format!("{b:?}"));false}
            _=>panic!("non-deterministic JBIG2 mutation result"),
        }
    }
    #[test]
    fn annex_h_truncation_mutations_are_bounded_and_deterministic(){
        let l=mutation_limits();let(mut accepted,mut rejected)=(0,0);
        for (g,p) in [(GLOBAL1,PAGE1),(GLOBAL1,PAGE2),(GLOBAL3,PAGE3)]{
            assert!(check_mutated_page(g,p,l));
            for n in 0..p.len(){
                if check_mutated_page(g,&p[..n],l){accepted+=1;}else{rejected+=1;}
            }
            for n in 0..g.len(){
                if check_mutated_page(&g[..n],p,l){accepted+=1;}else{rejected+=1;}
            }
        }
        // Complete segment prefixes can be valid embedded pages.
        assert!(accepted>0&&rejected>0);
        assert_eq!(accepted+rejected,2*GLOBAL1.len()+GLOBAL3.len()+PAGE1.len()+PAGE2.len()+PAGE3.len());
    }
    #[test]
    fn annex_h_bit_mutations_are_bounded_and_deterministic(){
        let l=mutation_limits();let(mut count,mut rejected)=(0,0);
        for (g,p) in [(GLOBAL1,PAGE1),(GLOBAL1,PAGE2),(GLOBAL3,PAGE3)]{
            assert!(check_mutated_page(g,p,l));
            for target in [false,true]{
                let original=if target{p}else{g};
                let mut bytes=original.to_vec();
                for i in 0..bytes.len(){
                    // Two deterministic bit flips per byte, covering high
                    // header bits and low arithmetic/Huffman bits.
                    for mask in [1u8,128]{
                        bytes[i]^=mask;
                        let ok=if target{check_mutated_page(g,&bytes,l)}else{check_mutated_page(&bytes,p,l)};
                        count+=1;if !ok{rejected+=1;}
                        bytes[i]^=mask;
                    }
                }
                assert_eq!(bytes.as_slice(),original);
            }
        }
        assert!(rejected>0);
        assert_eq!(count,2*(2*GLOBAL1.len()+GLOBAL3.len()+PAGE1.len()+PAGE2.len()+PAGE3.len()));
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
