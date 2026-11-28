//! Restricted, source-preserving Devanagari shaping cluster grammar.
//! Not a Unicode grapheme iterator or a complete OpenType Indic shaper.
use crate::{Error,IndicProperties,IndicScalar,IndicSyllabicCategory as S,UnicodeScriptProperties,SourceLimits,annotate_indic_scalars};
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum IndicClusterKind{Consonant,Vowel,Placeholder,Broken,Other}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct IndicCluster{
    pub kind:IndicClusterKind,pub scalar_start:usize,pub scalar_end:usize,
    pub byte_start:usize,pub byte_end:usize,
    /// Last admitted consonant, independent vowel or placeholder.
    /// A candidate only; actual shaping base selection is separate.
    pub base_candidate:Option<usize>,pub consonants:usize,
    pub terminal_virama:bool,pub ends_with_non_joiner:bool,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct IndicClusterRun{pub scalars:Vec<IndicScalar>,pub clusters:Vec<IndicCluster>}
#[derive(Debug,Clone,Copy)]
pub struct IndicClusterLimits{
    pub source:SourceLimits,pub max_clusters:usize,pub max_cluster_scalars:usize,pub max_work:usize,
}
impl Default for IndicClusterLimits{
    fn default()->Self{Self{source:Default::default(),max_clusters:65536,max_cluster_scalars:1024,max_work:262144}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
enum Token{C,V,P,N,H,J,Z,M,T,Other}
fn token(s:&IndicScalar,scripts:&UnicodeScriptProperties)->Token{
    if s.character=='\u{200d}'{return Token::J;}
    if s.character=='\u{200c}'{return Token::Z;}
    if s.character=='\u{25cc}'{return Token::P;}
    if !scripts.extensions.scripts(s.character,&scripts.primary).contains("Devanagari"){return Token::Other;}
    match s.syllabic{
        S::Consonant=>Token::C,S::VowelIndependent=>Token::V,
        S::ConsonantPlaceholder=>Token::P,S::Nukta=>Token::N,S::Virama=>Token::H,
        S::VowelDependent=>Token::M,
        S::Bindu|S::Visarga|S::CantillationMark|S::SyllableModifier=>Token::T,
        _=>Token::Other,
    }
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
enum State{Base,Nukta,Virama,Joiner,Matra,Tail,Closed,Broken,Other}
fn begin(t:Token,i:usize,s:&IndicScalar)->(IndicCluster,State){
    let (kind,state,base,consonants)=match t{
        Token::C=>(IndicClusterKind::Consonant,State::Base,Some(i),1),
        Token::V=>(IndicClusterKind::Vowel,State::Base,Some(i),0),
        Token::P=>(IndicClusterKind::Placeholder,State::Base,Some(i),0),
        Token::Other=>(IndicClusterKind::Other,State::Other,None,0),
        _=>(IndicClusterKind::Broken,State::Broken,None,0),
    };
    (IndicCluster{kind,scalar_start:i,scalar_end:i+1,byte_start:s.byte_start,byte_end:s.byte_end,
        base_candidate:base,consonants,terminal_virama:false,ends_with_non_joiner:false},state)
}
fn advance(c:&mut IndicCluster,state:State,t:Token,i:usize,s:&IndicScalar)->Option<State>{
    use State::*;
    let next=match (state,t){
        (Base,Token::N) if c.kind==IndicClusterKind::Consonant=>Nukta,
        (Base|Nukta,Token::H) if c.kind==IndicClusterKind::Consonant||c.kind==IndicClusterKind::Placeholder=>Virama,
        (Virama,Token::J)=>Joiner,
        (Virama,Token::Z)=>Closed,
        (Virama|Joiner,Token::C)=>Base,
        (Base|Nukta,Token::M) if c.kind!=IndicClusterKind::Vowel=>Matra,
        (Base|Nukta|Matra|Tail,Token::T)=>Tail,
        (Broken,Token::N|Token::H|Token::J|Token::Z|Token::M|Token::T)=>Broken,
        _=>return None,
    };
    if t==Token::C{c.base_candidate=Some(i);c.consonants+=1;}
    c.terminal_virama=matches!(next,Virama|Joiner|Closed);
    c.ends_with_non_joiner=next==Closed;
    c.scalar_end=i+1;c.byte_end=s.byte_end;
    Some(next)
}
/// Conservative shaping-cluster policy: C N? (H J? C N?)* followed by
/// optional dependent vowel and trailing modifiers; terminal H or H+join
/// control is retained. ZWNJ after H closes the cluster. Independent
/// vowels accept trailing modifiers; dotted circle is an explicit
/// placeholder. Orphan marks/controls form Broken clusters, never acquire
/// a fabricated base. Unhandled or foreign scalars form individual Other
/// clusters. No reordering, normalization or inserted dotted circle.
/// The work budget charges four units per input scalar (annotation,
/// classification and at most two state transitions).
pub fn analyze_devanagari_clusters(text:&str,p:&IndicProperties,scripts:&UnicodeScriptProperties,
    l:IndicClusterLimits)->Result<IndicClusterRun,Error>{
    if text.len()>l.source.max_bytes{return Err(Error::Limit("Indic cluster bytes"));}
    let n=text.chars().count();
    if n.checked_mul(4).ok_or(Error::Limit("Indic cluster work"))?>l.max_work{return Err(Error::Limit("Indic cluster work"));}
    let scalars=annotate_indic_scalars(text,p,l.source)?;
    let mut clusters=Vec::new();let mut current:Option<(IndicCluster,State)>=None;
    for (i,s) in scalars.iter().enumerate(){
        let t=token(s,scripts);
        if let Some((c,state))=&mut current{
            if let Some(next)=advance(c,*state,t,i,s){
                if c.scalar_end-c.scalar_start>l.max_cluster_scalars{return Err(Error::Limit("Indic cluster length"));}
                *state=next;continue;
            }
        }
        if let Some((c,_))=current.take(){clusters.push(c);}
        if clusters.len()>=l.max_clusters{return Err(Error::Limit("Indic cluster count"));}
        if l.max_cluster_scalars==0{return Err(Error::Limit("Indic cluster length"));}
        clusters.try_reserve(1).map_err(|_|Error::Limit("Indic cluster allocation"))?;
        current=Some(begin(t,i,s));
    }
    if let Some((c,_))=current{clusters.push(c);}
    Ok(IndicClusterRun{scalars,clusters})
}
