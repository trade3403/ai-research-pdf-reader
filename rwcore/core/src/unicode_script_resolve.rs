//! Conservative contextual script itemization in logical scalar order.
//! A product policy on UCD Script/Script_Extensions, not UAX #9 bidi,
//! grapheme segmentation, paired-bracket resolution or OpenType shaping.
use crate::{Error,ScriptSet,UnicodeScriptProperties,ScriptSpan,SourceLimits};
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ScriptConflictPolicy{
    /// Leave a character with conflicting compatible anchors unresolved.
    Preserve,
    /// Select the compatible logical left anchor when both disagree.
    PreferLeft,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ScriptResolutionKind{Fixed,Context,Unresolved,Unknown}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ResolvedScriptScalar<'a>{
    pub character:char,pub byte_start:usize,pub byte_end:usize,
    pub primary:&'a str,pub script:&'a str,pub kind:ScriptResolutionKind,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ResolvedScriptRun<'a>{
    pub scalars:Vec<ResolvedScriptScalar<'a>>,pub spans:Vec<ScriptSpan<'a>>,
}
#[derive(Debug,Clone,Copy)]
pub struct ScriptResolveLimits{
    pub source:SourceLimits,
    /// One unit per scalar per classification, left/right pass and output.
    pub max_work:usize,
}
impl Default for ScriptResolveLimits{
    fn default()->Self{Self{source:Default::default(),max_work:262144}}
}
#[derive(Clone,Copy)]
struct Candidate<'a>{
    primary:&'a str,set:ScriptSet<'a>,fixed:Option<&'a str>,barrier:bool,
}
fn neutral(s:&str)->bool{matches!(s,"Common"|"Inherited")}
impl<'a> Candidate<'a>{
    fn permits(self,s:&str)->bool{
        !self.barrier&&(matches!(self.set,ScriptSet::Primary(p) if neutral(p))||self.set.contains(s))
    }
    fn propagate(self,anchor:Option<&'a str>)->Option<&'a str>{
        if self.barrier{return None;}
        self.fixed.or_else(||anchor.filter(|s|self.permits(s)))
    }
}
fn classify<'a>(ch:char,p:&'a UnicodeScriptProperties)->Candidate<'a>{
    let primary=p.primary.script(ch);let set=p.extensions.scripts(ch,&p.primary);
    let barrier=primary=="Unknown";
    let fixed=if barrier{None}
        else if !neutral(primary)&&set.contains(primary){Some(primary)}
        else if set.len()==1{set.get(0).filter(|s|!neutral(s)&&*s!="Unknown")}
        else{None};
    Candidate{primary,set,fixed,barrier}
}
/// Fixed anchors are strong primary values admitted by their extension set,
/// or singleton explicit sets. Common/Inherited without explicit sets are
/// unconstrained; multi-script sets constrain propagation. Unknown and
/// incompatible sets stop propagation in that direction. Both passes use
/// only original fixed anchors, not context-assigned guesses. With no
/// compatible anchor, retain the primary label and mark it Unresolved.
/// Spans group adjacent equal labels; consult scalar kinds for ambiguity.
/// No reordering, normalization or mutation of the input.
pub fn resolve_script_runs<'a>(text:&str,p:&'a UnicodeScriptProperties,policy:ScriptConflictPolicy,
    l:ScriptResolveLimits)->Result<ResolvedScriptRun<'a>,Error>{
    if text.len()>l.source.max_bytes{return Err(Error::Limit("script resolve bytes"));}
    let n=text.chars().count();
    if n>l.source.max_scalars{return Err(Error::Limit("script resolve scalars"));}
    if n.checked_mul(4).ok_or(Error::Limit("script resolve work"))?>l.max_work{
        return Err(Error::Limit("script resolve work"));
    }
    let mut candidates=Vec::new();let mut left=Vec::new();let mut right=Vec::new();
    let mut scalars=Vec::new();let mut spans:Vec<ScriptSpan<'a>>=Vec::new();
    candidates.try_reserve_exact(n).map_err(|_|Error::Limit("script resolve allocation"))?;
    left.try_reserve_exact(n).map_err(|_|Error::Limit("script resolve allocation"))?;
    right.try_reserve_exact(n).map_err(|_|Error::Limit("script resolve allocation"))?;
    scalars.try_reserve_exact(n).map_err(|_|Error::Limit("script resolve allocation"))?;
    for ch in text.chars(){candidates.push(classify(ch,p));}
    let mut anchor=None;
    for &c in &candidates{anchor=c.propagate(anchor);left.push(anchor);}
    right.resize(n,None);anchor=None;
    for (i,&c) in candidates.iter().enumerate().rev(){anchor=c.propagate(anchor);right[i]=anchor;}
    for (i,(at,ch)) in text.char_indices().enumerate(){
        let c=candidates[i];
        let (script,kind)=if c.barrier{(c.primary,ScriptResolutionKind::Unknown)}
        else if let Some(s)=c.fixed{(s,ScriptResolutionKind::Fixed)}
        else{
            let chosen=match (left[i],right[i]){
                (Some(a),Some(b)) if a==b||policy==ScriptConflictPolicy::PreferLeft=>Some(a),
                (Some(_),Some(_))=>None,
                (Some(a),None)|(None,Some(a))=>Some(a),
                (None,None)=>None,
            };
            chosen.map_or((c.primary,ScriptResolutionKind::Unresolved),|s|(s,ScriptResolutionKind::Context))
        };
        let end=at+ch.len_utf8();
        scalars.push(ResolvedScriptScalar{character:ch,byte_start:at,byte_end:end,primary:c.primary,script,kind});
        if let Some(prev)=spans.last_mut().filter(|s|s.script==script){
            prev.scalar_end=i+1;prev.byte_end=end;
        }else{
            if spans.len()>=l.source.max_glyphs{return Err(Error::Limit("script resolve spans"));}
            spans.try_reserve(1).map_err(|_|Error::Limit("script resolve allocation"))?;
            spans.push(ScriptSpan{script,scalar_start:i,scalar_end:i+1,byte_start:at,byte_end:end});
        }
    }
    Ok(ResolvedScriptRun{scalars,spans})
}
