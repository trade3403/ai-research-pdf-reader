//! Bounded automatic source mapping for simple, unambiguous contractions.
//! This is not a general OpenType ligature history implementation.
use crate::{Error,Gdef,GdefGlyphClass,GposLigatureSource,ProvenanceLedger};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct LigatureMappingLimits{
    pub max_glyphs:usize,
    pub max_scalars:usize,
    pub max_work:usize,
}
impl Default for LigatureMappingLimits{
    fn default()->Self{Self{max_glyphs:65536,max_scalars:65536,max_work:4194304}}
}
fn charge(left:&mut usize,n:usize)->Result<(),Error>{
    *left=left.checked_sub(n).ok_or(Error::Limit("ligature mapping work"))?;Ok(())
}
/// Convert traced GSUB output to explicit occurrence-specific GPOS sources.
/// Restricted to a one-to-one partition of input scalars, with each
/// contraction component representing one scalar in strictly ascending
/// logical order. Surviving GDEF marks between components map to the
/// preceding component; adjacent trailing marks map to the last component.
/// Nested/split contractions, shared scalar ownership, unknown ligature
/// identity and nonmonotonic input are rejected. No visual reordering.
pub fn map_gpos_ligature_sources(ledger:&ProvenanceLedger,gdef:&Gdef,l:LigatureMappingLimits)
    ->Result<Vec<GposLigatureSource>,Error>{
    if ledger.ligature_trace_stats().is_none(){return Err(Error::Invalid("ligature mapping requires trace"));}
    let glyphs=ledger.glyphs();let n=ledger.scalar_count();
    if glyphs.len()>l.max_glyphs||n>l.max_scalars{return Err(Error::Limit("ligature mapping size"));}
    let mut left=l.max_work;charge(&mut left,n)?;charge(&mut left,glyphs.len())?;
    let mut used=Vec::new();used.try_reserve_exact(n).map_err(|_|Error::Limit("ligature mapping allocation"))?;used.resize(n,false);
    let mut out=Vec::new();out.try_reserve_exact(glyphs.len()).map_err(|_|Error::Limit("ligature mapping allocation"))?;
    // Verify a true partition before inferring gap ownership. Expansion
    // with duplicated scalar sources cannot establish component ownership.
    for g in glyphs{
        charge(&mut left,g.sources().len())?;
        if g.sources().is_empty(){return Err(Error::Invalid("ligature mapping empty source"));}
        for &s in g.sources(){
            let flag=used.get_mut(s).ok_or(Error::Invalid("ligature mapping scalar"))?;
            if *flag{return Err(Error::Unsupported("ligature mapping shared scalar"));}
            *flag=true;
        }
    }
    if used.iter().any(|v|!*v){return Err(Error::Unsupported("ligature mapping missing scalar"));}
    // next is the first not-yet-accounted-for scalar in logical order;
    // active component positions are consumed lazily as surviving gaps pass.
    let mut next=0usize;
    let mut active:Option<(&crate::LigatureLineage,usize)>=None;
    for g in glyphs{
        let class=gdef.glyph_class(g.glyph_id).ok_or(Error::Invalid("ligature mapping glyph domain"))?;
        if let Some(lineage)=g.ligature_lineage(){
            if let Some((old,i))=active{
                if i+1!=old.components().len(){return Err(Error::Unsupported("ligature mapping interrupted gap"));}
            }
            let c=lineage.components();
            charge(&mut left,c.len())?;
            if lineage.ambiguous()||class!=GdefGlyphClass::Ligature||lineage.id()==0||
                c.len()<2||c.len()>65535||c.len()!=g.sources().len()||
                c.iter().zip(g.sources()).any(|(a,b)|a.as_slice()!=[*b])||
                g.sources().first().copied()!=Some(next){
                return Err(Error::Unsupported("ligature mapping ambiguous components"));
            }
            out.push(GposLigatureSource::Ligature{id:lineage.id(),components:c.len()});
            next=next.checked_add(1).ok_or(Error::Limit("ligature mapping scalar"))?;
            let mut i=0;
            while i+1<c.len()&&c[i+1][0]==next{
                charge(&mut left,1)?;i+=1;next+=1;
            }
            active=Some((lineage,i));
        }else{
            if class==GdefGlyphClass::Ligature||g.sources()!=[next]{
                return Err(Error::Unsupported("ligature mapping unordered source"));
            }
            next+=1;
            if let Some((lineage,mut i))=active{
                let c=lineage.components();
                if class==GdefGlyphClass::Mark{
                    out.push(GposLigatureSource::Mark{id:lineage.id(),component:i});
                    while i+1<c.len()&&c[i+1][0]==next{
                        charge(&mut left,1)?;i+=1;next+=1;
                    }
                    active=Some((lineage,i));
                }else{
                    if i+1!=c.len(){return Err(Error::Unsupported("ligature mapping nonmark gap"));}
                    active=None;out.push(GposLigatureSource::Unmapped);
                }
            }else{out.push(GposLigatureSource::Unmapped);}
        }
    }
    if next!=n{return Err(Error::Unsupported("ligature mapping incomplete order"));}
    Ok(out)
}
