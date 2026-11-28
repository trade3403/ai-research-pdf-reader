//! Restricted Devanagari pre-shaping analysis and provisional reorder plan.
//! This does NOT reorder SourceRun, execute GSUB, or decide actual glyph
//! bases/reph from a font. The permutation is a candidate for a later
//! glyph-stage operation, never a replacement for logical GSUB input.
use crate::{Error,IndicProperties,UnicodeScriptProperties,IndicClusterLimits,
    IndicClusterRun,IndicClusterKind,IndicSyllabicCategory as S,
    IndicPositionalCategory as P,analyze_devanagari_clusters};
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum IndicReorderStatus{
    Identity,SimplePrebaseCandidate,NeedsFont,Unsupported, Broken,Other,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum IndicRephCandidate{
    None,
    /// Logical RA + virama, to be tested against rphf in a later stage.
    Prefix{ra:usize,virama:usize},
    /// Explicit join control inhibits the implicit-prefix policy.
    JoinControl,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct IndicClusterReorder{
    pub scalar_start:usize,pub scalar_end:usize,
    pub base_candidate:Option<usize>,pub reph:IndicRephCandidate,
    pub prebase_matra:Option<usize>,pub status:IndicReorderStatus,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct IndicReorderPlan{
    pub logical:IndicClusterRun,pub clusters:Vec<IndicClusterReorder>,
    /// Proposed visual scalar order, identity for all deferred clusters.
    /// Indices reference original logical scalars; not actual glyph order.
    pub candidate_order:Vec<usize>,
    /// Inverse of candidate_order, indexed by original logical scalar.
    pub candidate_inverse:Vec<usize>,
}
#[derive(Debug,Clone,Copy)]
pub struct IndicReorderLimits{
    pub clusters:IndicClusterLimits,
    /// Bound output records independently of analysis.
    pub max_scalars:usize,pub max_work:usize,
}
impl Default for IndicReorderLimits{
    fn default()->Self{Self{clusters:Default::default(),max_scalars:65536,max_work:262144}}
}
fn reserve<T>(v:&mut Vec<T>,n:usize)->Result<(),Error>{
    v.try_reserve_exact(n).map_err(|_|Error::Limit("Indic reorder allocation"))
}
/// Analyze simple single-base left matras and recognize implicit reph
/// candidates. Complex consonant chains, terminal virama, join controls,
/// mixed-position matras and reph are deferred rather than guessed.
/// No scalar may move across a cluster boundary. No scalar is inserted,
/// removed or normalized; broken and foreign clusters remain in order.
/// Work charges four units per scalar for planning and inverse generation,
/// independently of the cluster analysis budget.
pub fn plan_devanagari_reorder(text:&str,p:&IndicProperties,scripts:&UnicodeScriptProperties,
    l:IndicReorderLimits)->Result<IndicReorderPlan,Error>{
    if text.len()>l.clusters.source.max_bytes{return Err(Error::Limit("Indic reorder bytes"));}
    let n=text.chars().count();
    if n>l.max_scalars{return Err(Error::Limit("Indic reorder scalars"));}
    if n.checked_mul(4).ok_or(Error::Limit("Indic reorder work"))?>l.max_work{
        return Err(Error::Limit("Indic reorder work"));
    }
    let logical=analyze_devanagari_clusters(text,p,scripts,l.clusters)?;
    let mut clusters=Vec::new();let mut order=Vec::new();let mut inverse=Vec::new();
    reserve(&mut clusters,logical.clusters.len())?;reserve(&mut order,n)?;reserve(&mut inverse,n)?;
    for c in &logical.clusters{
        let a=c.scalar_start;let b=c.scalar_end;let s=&logical.scalars[a..b];
        let mut reph=IndicRephCandidate::None;
        if c.kind==IndicClusterKind::Consonant&&s.len()>=3&&s[0].character=='\u{0930}'&&s[1].character=='\u{094d}'{
            if matches!(s[2].character,'\u{200c}'|'\u{200d}'){reph=IndicRephCandidate::JoinControl;}
            else if s[2].syllabic==S::Consonant{reph=IndicRephCandidate::Prefix{ra:a,virama:a+1};}
        }
        let mut prebase=None;let mut special_matra=false;let mut control=false;
        for (i,v) in s.iter().enumerate(){
            if matches!(v.character,'\u{200c}'|'\u{200d}'){control=true;}
            if v.syllabic==S::VowelDependent{
                match v.positional{
                    P::Left=>prebase=Some(a+i),
                    P::Right|P::Top|P::Bottom=>{},
                    _=>special_matra=true,
                }
            }
        }
        let status=match c.kind{
            IndicClusterKind::Broken=>IndicReorderStatus::Broken,
            IndicClusterKind::Other=>IndicReorderStatus::Other,
            _ if special_matra=>IndicReorderStatus::Unsupported,
            _ if control||c.terminal_virama||c.consonants>1||reph!=IndicRephCandidate::None=>IndicReorderStatus::NeedsFont,
            _ if prebase.is_some()&&c.base_candidate==Some(a)=>IndicReorderStatus::SimplePrebaseCandidate,
            _ if prebase.is_some()=>IndicReorderStatus::NeedsFont,
            _=>IndicReorderStatus::Identity,
        };
        if status==IndicReorderStatus::SimplePrebaseCandidate{
            let m=prebase.ok_or(Error::Invalid("Indic reorder matra"))?;
            order.push(m);order.extend((a..b).filter(|&i|i!=m));
        }else{order.extend(a..b);}
        clusters.push(IndicClusterReorder{scalar_start:a,scalar_end:b,
            base_candidate:c.base_candidate,reph,prebase_matra:prebase,status});
    }
    inverse.resize(n,usize::MAX);
    for (at,&i) in order.iter().enumerate(){
        let slot=inverse.get_mut(i).ok_or(Error::Invalid("Indic reorder index"))?;
        if *slot!=usize::MAX{return Err(Error::Invalid("Indic reorder duplicate"));}
        *slot=at;
    }
    if order.len()!=n||inverse.contains(&usize::MAX){return Err(Error::Invalid("Indic reorder permutation"));}
    Ok(IndicReorderPlan{logical,clusters,candidate_order:order,candidate_inverse:inverse})
}
