//! T.88 7.2.3–7.2.4 logical segment lifetime audit for one embedded page.
//! This is a strict, opt-in validator, not a bitmap allocator or a
//! cross-page globals cache. A deferred non-retain bit describes attached
//! extension segments, not an arbitrary delay until End-of-Page.
use crate::{Error,Result};
use super::{Segment,bitmap,inspect_pdf_streams};
use std::collections::BTreeMap;

#[derive(Debug,Clone,Copy,Default,PartialEq,Eq)]
pub struct Audit{
    pub segments:usize,pub references:usize,pub peak_live:usize,
    pub retained:usize,pub released:usize,pub deferred_chains:usize,
    pub attached_extensions:usize,pub live_globals_at_end:usize,
    pub live_page_at_end:usize,pub work_used:u64,
}
/// Index zero is the current segment. Index i+1 is referred segment i.
/// Both short and long forms store flags in ascending low-bit order.
pub fn retain_bit(s:&Segment,index:usize)->Result<bool>{
    if index>s.referred.len(){return Err(Error::Invalid("JBIG2 retention index"));}
    let b=*s.retention.get(index/8).ok_or(Error::Invalid("JBIG2 retention flag span"))?;
    Ok(b&(1<<(index%8))!=0)
}
fn validate_bits(s:&Segment)->Result<()>{
    let bits=s.referred.len()+1;
    if s.retention.len()!=bits.div_ceil(8){return Err(Error::Invalid("JBIG2 retention flag length"));}
    let rem=bits%8;
    if rem!=0&&s.retention.last().is_some_and(|b|b&!((1u8<<rem)-1)!=0){
        return Err(Error::Invalid("JBIG2 unused retention flags"));
    }
    Ok(())
}
fn charge(a:&mut Audit,limit:u64,n:u64)->Result<()>{
    a.work_used=a.work_used.checked_add(n).ok_or(Error::Limit("JBIG2 retention work"))?;
    if a.work_used>limit{return Err(Error::Limit("JBIG2 retention work"));}
    Ok(())
}
/// Audit the actual input graph. Reject any reference to a segment which
/// was not retained or whose last-reference flag already released it.
/// Deferred chains require contiguous type 62 extensions referring only
/// to their parent; the last extension must clear its parent retain bit.
/// Global lifetime is scoped to this one audit, never mutates shared bytes.
/// End-of-page cleanup is reported separately from bit-directed release.
pub fn audit_pdf_streams(globals:&[u8],page:&[u8],work_limit:u64)->Result<Audit>{
    if work_limit>bitmap::MAX_COMPOSE_WORK{return Err(Error::Limit("JBIG2 retention work"));}
    let input=globals.len().checked_add(page.len()).ok_or(Error::Limit("JBIG2 retention input"))?;
    let mut a=Audit::default();charge(&mut a,work_limit,input as u64)?;
    let graph=inspect_pdf_streams(globals,page)?;
    let mut live=BTreeMap::<u32,u32>::new();
    let mut pending:Option<u32>=None;
    for node in &graph.graph{
        let s=graph.segment(node.location).ok_or(Error::Invalid("JBIG2 retention graph"))?;
        charge(&mut a,work_limit,2+s.referred.len() as u64*2+s.retention.len() as u64)?;
        validate_bits(s)?;
        a.segments+=1;a.references+=s.referred.len();
        if let Some(parent)=pending{
            if s.kind!=62||s.referred.as_slice()!=[parent]||s.deferred_non_retain{
                return Err(Error::Invalid("JBIG2 deferred extension chain"));
            }
            a.attached_extensions+=1;
            if !retain_bit(s,1)?{pending=None;}
        }
        // Validate all references before applying any last-reference flags.
        // Repeated references with the same flag are allowed; conflicting
        // flags are ambiguous and rejected by this strict subset.
        let mut refs=BTreeMap::new();
        for (i,&id) in s.referred.iter().enumerate(){
            if !live.contains_key(&id){return Err(Error::Invalid("JBIG2 reference to nonretained segment"));}
            let retain=retain_bit(s,i+1)?;
            if refs.insert(id,retain).is_some_and(|old|old!=retain){
                return Err(Error::Invalid("JBIG2 conflicting retention flags"));
            }
        }
        for (id,retain) in refs{
            if !retain{live.remove(&id);a.released+=1;}
        }
        let own=retain_bit(s,0)?;
        if s.deferred_non_retain{
            if !own{return Err(Error::Invalid("JBIG2 deferred segment not retained"));}
            pending=Some(s.number);a.deferred_chains+=1;
        }
        if own{
            if matches!(s.kind,48|49|51){return Err(Error::Invalid("JBIG2 retained page control"));}
            live.insert(s.number,s.page);a.retained+=1;
        }
        a.peak_live=a.peak_live.max(live.len());
    }
    if pending.is_some(){return Err(Error::Invalid("JBIG2 unterminated deferred extension chain"));}
    charge(&mut a,work_limit,live.len() as u64)?;
    a.live_globals_at_end=live.values().filter(|&&page|page==0).count();
    a.live_page_at_end=live.len()-a.live_globals_at_end;
    Ok(a)
}

#[cfg(test)]
mod tests{
    use super::*;
    fn seg(n:u8,k:u8,page:u8,refs:&[u8],ret:&[u8],data:&[u8])->Vec<u8>{
        let mut b=vec![0,0,0,n,k];
        if refs.len()<=4{b.push((refs.len() as u8)<<5|ret[0]);}
        else{b.extend((0xe0000000u32|refs.len() as u32).to_be_bytes());b.extend(ret);}
        b.extend(refs);b.push(page);b.extend((data.len() as u32).to_be_bytes());b.extend(data);b
    }
    fn info()->Vec<u8>{
        let mut d=Vec::new();for n in [8u32,1,0,0]{d.extend(n.to_be_bytes());}
        d.extend([0,0,0]);seg(1,48,1,&[],&[0],&d)
    }
    #[test]
    fn retention_short_last_reference_and_global_scope(){
        let g=seg(0,16,0,&[],&[1],&[]);
        let mut p=info();
        p.extend(seg(2,16,1,&[],&[1],&[]));
        p.extend(seg(3,22,1,&[0],&[2],&[]));
        p.extend(seg(4,22,1,&[0],&[0],&[]));
        p.extend(seg(5,22,1,&[2],&[0],&[]));
        let a=audit_pdf_streams(&g,&p,10000).unwrap();
        assert_eq!((a.segments,a.references,a.retained,a.released,a.peak_live),(6,3,2,2,2));
        assert_eq!((a.live_globals_at_end,a.live_page_at_end),(0,0));
        assert_eq!(audit_pdf_streams(&g,&p,a.work_used).unwrap(),a);
        assert!(audit_pdf_streams(&g,&p,a.work_used-1).is_err());
        let mut bad=p.clone();bad.extend(seg(6,22,1,&[0],&[0],&[]));
        assert!(audit_pdf_streams(&g,&bad,10000).is_err());
        assert!(audit_pdf_streams(&seg(0,16,0,&[],&[0],&[]),&p,10000).is_err());
        assert_eq!(audit_pdf_streams(&g,&info(),10000).unwrap().live_globals_at_end,1);
    }
    #[test]
    fn retention_long_flags_and_unused_bits(){
        let mut p=info();
        for n in 2..=9{p.extend(seg(n,16,1,&[],&[1],&[]));}
        let refs=[2,3,4,5,6,7,8,9];
        p.extend(seg(10,62,1,&refs,&[0b10101010,1],&[]));
        let a=audit_pdf_streams(&[],&p,10000).unwrap();
        assert_eq!((a.references,a.retained,a.released,a.live_page_at_end),(8,8,3,5));
        let mut bad=p.clone();bad.extend(seg(11,62,1,&[3],&[0],&[]));
        assert!(audit_pdf_streams(&[],&bad,10000).is_err());
        let mut ok=p;ok.extend(seg(11,62,1,&[2,9],&[0],&[]));
        assert_eq!(audit_pdf_streams(&[],&ok,10000).unwrap().released,5);
        let mut bad=info();bad.extend(seg(2,16,1,&[],&[2],&[]));
        assert!(audit_pdf_streams(&[],&bad,10000).is_err());
        let mut bad=info();for n in 2..=9{bad.extend(seg(n,16,1,&[],&[1],&[]));}
        bad.extend(seg(10,62,1,&refs,&[0xaa,2],&[]));
        assert!(audit_pdf_streams(&[],&bad,10000).is_err());
    }
    #[test]
    fn retention_deferred_attached_extension_chain(){
        let mut p=info();p.extend(seg(2,16|128,1,&[],&[1],&[]));
        let pending=p.clone();
        p.extend(seg(3,62,1,&[2],&[2],&[]));p.extend(seg(4,62,1,&[2],&[0],&[]));
        let a=audit_pdf_streams(&[],&p,10000).unwrap();
        assert_eq!((a.deferred_chains,a.attached_extensions,a.released),(1,2,1));
        assert!(audit_pdf_streams(&[],&pending,10000).is_err());
        let mut bad=pending.clone();bad.extend(seg(3,22,1,&[2],&[0],&[]));
        assert!(audit_pdf_streams(&[],&bad,10000).is_err());
        let mut bad=pending;bad.extend(seg(3,62,1,&[2],&[2],&[]));bad.extend(seg(4,16,1,&[],&[0],&[]));
        assert!(audit_pdf_streams(&[],&bad,10000).is_err());
    }
}
