//! Native Unicode Joining_Type index and basic logical joining analysis.
//! Based on UCD DerivedJoiningType, not Arabic block ranges.
//! Four-form Arabic model; no Syriac special forms, Mongolian shaping,
//! bidi reordering, GSUB feature application or normalization.
use crate::{Error,SourceLimits};
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum JoiningType{NonJoining,JoinCausing,Dual,Right,Left,Transparent}
impl JoiningType{
    fn parse(s:&str)->Result<Self,Error>{match s.trim(){
        "U"|"Non_Joining"=>Ok(Self::NonJoining),
        "C"|"Join_Causing"=>Ok(Self::JoinCausing),
        "D"|"Dual_Joining"=>Ok(Self::Dual),
        "R"|"Right_Joining"=>Ok(Self::Right),
        "L"|"Left_Joining"=>Ok(Self::Left),
        "T"|"Transparent"=>Ok(Self::Transparent),
        _=>Err(Error::Invalid("joining type value"))
    }}
    /// Physical left capability; towards the next logical scalar in
    /// this right-to-left joining model.
    pub fn joins_left(self)->bool{matches!(self,Self::Dual|Self::Left|Self::JoinCausing)}
    pub fn joins_right(self)->bool{matches!(self,Self::Dual|Self::Right|Self::JoinCausing)}
}
#[derive(Debug,Clone,Copy)]
pub struct JoiningDataLimits{pub max_bytes:usize,pub max_ranges:usize}
impl Default for JoiningDataLimits{
    fn default()->Self{Self{max_bytes:2*1024*1024,max_ranges:32768}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
struct JoiningRange{first:u32,last:u32,kind:JoiningType}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct UnicodeJoiningTypes{ranges:Vec<JoiningRange>}
fn point(s:&str)->Result<u32,Error>{
    let s=s.trim();
    if s.is_empty()||s.len()>6||!s.bytes().all(|b|b.is_ascii_hexdigit()){
        return Err(Error::Invalid("joining codepoint"));
    }
    let v=u32::from_str_radix(s,16).map_err(|_|Error::Invalid("joining codepoint"))?;
    if v>0x10ffff{return Err(Error::Invalid("joining codepoint"));}Ok(v)
}
fn record(s:&str)->Result<JoiningRange,Error>{
    let (r,v)=s.split_once(';').ok_or(Error::Invalid("joining fields"))?;
    let (first,last)=if let Some((a,b))=r.split_once(".."){(point(a)?,point(b)?)}else{let a=point(r)?;(a,a)};
    if first>last{return Err(Error::Invalid("joining reversed range"));}
    Ok(JoiningRange{first,last,kind:JoiningType::parse(v)?})
}
pub fn parse_joining_types(text:&str,l:JoiningDataLimits)->Result<UnicodeJoiningTypes,Error>{
    if text.len()>l.max_bytes{return Err(Error::Limit("joining bytes"));}
    let mut ranges=Vec::new();let mut missing=false;
    for raw in text.lines(){
        let raw=raw.trim();
        if let Some(m)=raw.strip_prefix('#').map(str::trim).and_then(|s|s.strip_prefix("@missing:")){
            let r=record(m.split('#').next().unwrap_or(""))?;
            if missing||r.first!=0||r.last!=0x10ffff||r.kind!=JoiningType::NonJoining{
                return Err(Error::Invalid("joining default"));
            }missing=true;continue;
        }
        let s=raw.split('#').next().unwrap_or("").trim();if s.is_empty(){continue;}
        if ranges.len()>=l.max_ranges{return Err(Error::Limit("joining ranges"));}
        let r=record(s)?;
        if r.first<=0xdfff&&r.last>=0xd800&&r.kind!=JoiningType::NonJoining{
            return Err(Error::Invalid("joining surrogate"));
        }
        ranges.try_reserve(1).map_err(|_|Error::Limit("joining allocation"))?;
        ranges.push(r);
    }
    ranges.sort_unstable_by_key(|r|r.first);
    if ranges.windows(2).any(|w|w[0].last>=w[1].first){return Err(Error::Invalid("joining overlap"));}
    Ok(UnicodeJoiningTypes{ranges})
}
impl UnicodeJoiningTypes{
    pub fn range_count(&self)->usize{self.ranges.len()}
    pub fn get(&self,ch:char)->JoiningType{
        let n=u32::from(ch);let i=self.ranges.partition_point(|r|r.last<n);
        self.ranges.get(i).filter(|r|r.first<=n).map_or(JoiningType::NonJoining,|r|r.kind)
    }
}
pub const BUNDLED_JOINING_TYPES:&str=include_str!("../data/unicode/17.0.0/DerivedJoiningType.txt");
pub fn bundled_joining_types()->Result<UnicodeJoiningTypes,Error>{
    parse_joining_types(BUNDLED_JOINING_TYPES,Default::default())
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ArabicJoiningForm{None,Isolated,Initial,Medial,Final}
impl ArabicJoiningForm{
    /// Only the basic four joining features, not a complete feature plan.
    pub fn feature(self)->Option<[u8;4]>{match self{
        Self::None=>None,Self::Isolated=>Some(*b"isol"),Self::Initial=>Some(*b"init"),
        Self::Medial=>Some(*b"medi"),Self::Final=>Some(*b"fina")
    }}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct JoiningScalar{
    pub character:char,pub byte_start:usize,pub byte_end:usize,pub kind:JoiningType,
    pub joins_previous:bool,pub joins_next:bool,pub form:ArabicJoiningForm,
}
#[derive(Debug,Clone,Copy)]
pub struct JoiningAnalysisLimits{
    pub source:SourceLimits,pub max_work:usize,
}
impl Default for JoiningAnalysisLimits{
    fn default()->Self{Self{source:Default::default(),max_work:131072}}
}
/// Analyze a caller-selected logical RTL joining run, without script
/// itemization. Transparent scalars are skipped for neighbor selection,
/// but their source positions remain. Join-causing scalars participate
/// without a form feature; non-joining scalars break links. The source
/// glyph cap is reused as a bound on output scalar records.
pub fn analyze_arabic_joining(text:&str,data:&UnicodeJoiningTypes,l:JoiningAnalysisLimits)->Result<Vec<JoiningScalar>,Error>{
    if text.len()>l.source.max_bytes{return Err(Error::Limit("joining source bytes"));}
    let n=text.chars().count();
    if n>l.source.max_scalars||n>l.source.max_glyphs{return Err(Error::Limit("joining source records"));}
    if n.checked_mul(2).ok_or(Error::Limit("joining work"))?>l.max_work{return Err(Error::Limit("joining work"));}
    let mut out:Vec<JoiningScalar>=Vec::new();
    out.try_reserve_exact(n).map_err(|_|Error::Limit("joining allocation"))?;
    let mut previous:Option<usize>=None;
    for (at,ch) in text.char_indices(){
        let kind=data.get(ch);
        let mut joins_previous=false;
        if kind!=JoiningType::Transparent{
            if let Some(i)=previous{
                if out[i].kind.joins_left()&&kind.joins_right(){
                    out[i].joins_next=true;joins_previous=true;
                }
            }
            previous=Some(out.len());
        }
        out.push(JoiningScalar{character:ch,byte_start:at,byte_end:at+ch.len_utf8(),kind,
            joins_previous,joins_next:false,form:ArabicJoiningForm::None});
    }
    for s in &mut out{
        if matches!(s.kind,JoiningType::Dual|JoiningType::Right|JoiningType::Left){
            s.form=match (s.joins_previous,s.joins_next){
                (false,false)=>ArabicJoiningForm::Isolated,(false,true)=>ArabicJoiningForm::Initial,
                (true,false)=>ArabicJoiningForm::Final,(true,true)=>ArabicJoiningForm::Medial,
            };
        }
    }
    Ok(out)
}
