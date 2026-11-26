//! Bounded exact scalar provenance ledger for explicit glyph edits.
//! Independent of GSUB scheduling. Noncontiguous ligature components
//! retain intervening glyphs; source sets do not approximate byte spans.
use crate::{Error,SourceRun};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ProvenanceLimits {
    pub max_glyphs:usize,
    pub max_refs:usize,
    pub max_sources_per_glyph:usize,
    pub max_work:usize,
}
impl Default for ProvenanceLimits {
    fn default()->Self{Self{max_glyphs:65536,max_refs:262144,max_sources_per_glyph:4096,max_work:4194304}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct TrackedGlyph {
    pub glyph_id:u16,
    sources:Vec<usize>,
}
impl TrackedGlyph {
    pub fn sources(&self)->&[usize]{&self.sources}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ProvenanceLedger {
    glyphs:Vec<TrackedGlyph>,
    scalar_count:usize,
    glyph_domain:u32,
    refs:usize,
    work:usize,
    limits:ProvenanceLimits,
}
fn add(a:usize,b:usize)->Result<usize,Error>{a.checked_add(b).ok_or(Error::Limit("provenance arithmetic"))}
fn mul(a:usize,b:usize)->Result<usize,Error>{a.checked_mul(b).ok_or(Error::Limit("provenance arithmetic"))}
fn reserve<T>(v:&mut Vec<T>,n:usize)->Result<(),Error>{v.try_reserve_exact(n).map_err(|_|Error::Limit("provenance allocation"))}
fn copy_sources(s:&[usize])->Result<Vec<usize>,Error>{
    let mut out=Vec::new();reserve(&mut out,s.len())?;out.extend_from_slice(s);Ok(out)
}
impl ProvenanceLedger {
    pub fn from_run(run:&SourceRun,glyph_domain:u32,l:ProvenanceLimits)->Result<Self,Error>{
        if glyph_domain==0||glyph_domain>65536{return Err(Error::Invalid("provenance glyph domain"));}
        let n=run.glyphs().len();
        if n>l.max_glyphs||n>l.max_refs||n>l.max_work||(n>0&&l.max_sources_per_glyph==0){
            return Err(Error::Limit("provenance seed"));
        }
        let mut glyphs=Vec::new();reserve(&mut glyphs,n)?;
        for g in run.glyphs(){
            if u32::from(g.glyph_id)>=glyph_domain||g.scalar_index>=run.scalars().len(){
                return Err(Error::Invalid("provenance seed"));
            }
            glyphs.push(TrackedGlyph{glyph_id:g.glyph_id,sources:copy_sources(&[g.scalar_index])?});
        }
        Ok(Self{glyphs,scalar_count:run.scalars().len(),glyph_domain,refs:n,work:n,limits:l})
    }
    pub fn glyphs(&self)->&[TrackedGlyph]{&self.glyphs}
    pub fn scalar_count(&self)->usize{self.scalar_count}
    pub fn reference_count(&self)->usize{self.refs}
    pub fn work(&self)->usize{self.work}
    fn check_id(&self,id:u16)->Result<(),Error>{
        if u32::from(id)>=self.glyph_domain{return Err(Error::Invalid("provenance glyph id"));}Ok(())
    }
    fn budget(&self,n:usize,refs:usize,work:usize)->Result<usize,Error>{
        let total=add(self.work,work)?;
        if n>self.limits.max_glyphs||refs>self.limits.max_refs||total>self.limits.max_work{
            return Err(Error::Limit("provenance budget"));
        }Ok(total)
    }
    /// Replace one glyph with a nonempty sequence, duplicating exact
    /// source sets. No deletion: orphan policy is a later API.
    pub fn expand(&mut self,at:usize,ids:&[u16])->Result<(),Error>{
        let g=self.glyphs.get(at).ok_or(Error::Invalid("provenance position"))?;
        if ids.is_empty(){return Err(Error::Invalid("provenance empty expansion"));}
        let n=add(self.glyphs.len()-1,ids.len())?;
        let copies=mul(g.sources.len(),ids.len())?;
        let refs=add(self.refs-g.sources.len(),copies)?;
        let work=self.budget(n,refs,add(add(self.glyphs.len(),ids.len())?,copies)?)?;
        for &id in ids{self.check_id(id)?;}
        let mut replacement=Vec::new();reserve(&mut replacement,ids.len())?;
        for &id in ids{replacement.push(TrackedGlyph{glyph_id:id,sources:copy_sources(&g.sources)?});}
        // Reserve final capacity before modifying the ledger.
        self.glyphs.try_reserve_exact(ids.len()-1).map_err(|_|Error::Limit("provenance allocation"))?;
        self.glyphs.splice(at..at+1,replacement);
        self.refs=refs;self.work=work;Ok(())
    }
    /// Merge strictly ascending physical positions (at least two).
    /// First position retains the ligature; unselected gaps are preserved.
    /// Deduplicate exact scalar indices in ascending logical order.
    pub fn contract(&mut self,positions:&[usize],id:u16)->Result<(),Error>{
        self.check_id(id)?;
        if positions.len()<2||positions.len()>self.glyphs.len()||
            positions.windows(2).any(|w|w[0]>=w[1])||
            positions.last().copied().unwrap_or(usize::MAX)>=self.glyphs.len(){
            return Err(Error::Invalid("provenance components"));
        }
        let mut count=0;
        for &p in positions{count=add(count,self.glyphs[p].sources.len())?;}
        // Charge scan plus conservative sorting bound before allocation.
        let factor=(usize::BITS-count.leading_zeros()) as usize;
        let charge=add(add(self.glyphs.len(),positions.len())?,mul(count,add(factor,2)?)?)?;
        let n=self.glyphs.len()-positions.len()+1;
        let work=self.budget(n,self.refs,charge)?;
        let mut sources=Vec::new();reserve(&mut sources,count)?;
        for &p in positions{sources.extend_from_slice(&self.glyphs[p].sources);}
        sources.sort_unstable();sources.dedup();
        if sources.len()>self.limits.max_sources_per_glyph{return Err(Error::Limit("provenance source set"));}
        let refs=add(self.refs-count,sources.len())?;
        // Compact in one linear pass, retaining all ignored gaps.
        let mut next=Vec::new();reserve(&mut next,n)?;
        let first=positions[0];
        let mut selected=positions.iter().copied().peekable();
        let mut merged=Some(TrackedGlyph{glyph_id:id,sources});
        for (i,g) in self.glyphs.drain(..).enumerate(){
            if selected.peek()==Some(&i){
                selected.next();
                if i==first{next.push(merged.take().expect("first component"));}
            }else{next.push(g);}
        }
        self.glyphs=next;self.refs=refs;self.work=work;Ok(())
    }
}
