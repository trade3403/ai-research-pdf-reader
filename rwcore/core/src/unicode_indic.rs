//! Bounded native lookup of Unicode 17.0.0 Indic syllabic and positional
//! properties. No syllable segmentation or glyph reordering is implied.
use crate::Error;
trait Property:Copy+Eq{
    const DEFAULT:Self;
    fn parse(s:&str)->Result<Self,Error>;
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum IndicSyllabicCategory{Other,Bindu,Visarga,Avagraha,Nukta,Virama,PureKiller,ReorderingKiller,InvisibleStacker,VowelIndependent,VowelDependent,Vowel,ConsonantPlaceholder,Consonant,ConsonantDead,ConsonantWithStacker,ConsonantPrefixed,ConsonantPrecedingRepha,ConsonantInitialPostfixed,ConsonantSucceedingRepha,ConsonantSubjoined,ConsonantMedial,ConsonantFinal,ConsonantHeadLetter,ModifyingLetter,ToneLetter,ToneMark,GeminationMark,CantillationMark,RegisterShifter,SyllableModifier,ConsonantKiller,NonJoiner,Joiner,NumberJoiner,Number,BrahmiJoiningNumber}
impl IndicSyllabicCategory{
    pub fn name(self)->&'static str{match self{
        Self::Other=>"Other",
        Self::Bindu=>"Bindu",
        Self::Visarga=>"Visarga",
        Self::Avagraha=>"Avagraha",
        Self::Nukta=>"Nukta",
        Self::Virama=>"Virama",
        Self::PureKiller=>"Pure_Killer",
        Self::ReorderingKiller=>"Reordering_Killer",
        Self::InvisibleStacker=>"Invisible_Stacker",
        Self::VowelIndependent=>"Vowel_Independent",
        Self::VowelDependent=>"Vowel_Dependent",
        Self::Vowel=>"Vowel",
        Self::ConsonantPlaceholder=>"Consonant_Placeholder",
        Self::Consonant=>"Consonant",
        Self::ConsonantDead=>"Consonant_Dead",
        Self::ConsonantWithStacker=>"Consonant_With_Stacker",
        Self::ConsonantPrefixed=>"Consonant_Prefixed",
        Self::ConsonantPrecedingRepha=>"Consonant_Preceding_Repha",
        Self::ConsonantInitialPostfixed=>"Consonant_Initial_Postfixed",
        Self::ConsonantSucceedingRepha=>"Consonant_Succeeding_Repha",
        Self::ConsonantSubjoined=>"Consonant_Subjoined",
        Self::ConsonantMedial=>"Consonant_Medial",
        Self::ConsonantFinal=>"Consonant_Final",
        Self::ConsonantHeadLetter=>"Consonant_Head_Letter",
        Self::ModifyingLetter=>"Modifying_Letter",
        Self::ToneLetter=>"Tone_Letter",
        Self::ToneMark=>"Tone_Mark",
        Self::GeminationMark=>"Gemination_Mark",
        Self::CantillationMark=>"Cantillation_Mark",
        Self::RegisterShifter=>"Register_Shifter",
        Self::SyllableModifier=>"Syllable_Modifier",
        Self::ConsonantKiller=>"Consonant_Killer",
        Self::NonJoiner=>"Non_Joiner",
        Self::Joiner=>"Joiner",
        Self::NumberJoiner=>"Number_Joiner",
        Self::Number=>"Number",
        Self::BrahmiJoiningNumber=>"Brahmi_Joining_Number",
    }}
}
impl Property for IndicSyllabicCategory{
    const DEFAULT:Self=Self::Other;
    fn parse(s:&str)->Result<Self,Error>{match s{
        "Other"=>Ok(Self::Other),
        "Bindu"=>Ok(Self::Bindu),
        "Visarga"=>Ok(Self::Visarga),
        "Avagraha"=>Ok(Self::Avagraha),
        "Nukta"=>Ok(Self::Nukta),
        "Virama"=>Ok(Self::Virama),
        "Pure_Killer"=>Ok(Self::PureKiller),
        "Reordering_Killer"=>Ok(Self::ReorderingKiller),
        "Invisible_Stacker"=>Ok(Self::InvisibleStacker),
        "Vowel_Independent"=>Ok(Self::VowelIndependent),
        "Vowel_Dependent"=>Ok(Self::VowelDependent),
        "Vowel"=>Ok(Self::Vowel),
        "Consonant_Placeholder"=>Ok(Self::ConsonantPlaceholder),
        "Consonant"=>Ok(Self::Consonant),
        "Consonant_Dead"=>Ok(Self::ConsonantDead),
        "Consonant_With_Stacker"=>Ok(Self::ConsonantWithStacker),
        "Consonant_Prefixed"=>Ok(Self::ConsonantPrefixed),
        "Consonant_Preceding_Repha"=>Ok(Self::ConsonantPrecedingRepha),
        "Consonant_Initial_Postfixed"=>Ok(Self::ConsonantInitialPostfixed),
        "Consonant_Succeeding_Repha"=>Ok(Self::ConsonantSucceedingRepha),
        "Consonant_Subjoined"=>Ok(Self::ConsonantSubjoined),
        "Consonant_Medial"=>Ok(Self::ConsonantMedial),
        "Consonant_Final"=>Ok(Self::ConsonantFinal),
        "Consonant_Head_Letter"=>Ok(Self::ConsonantHeadLetter),
        "Modifying_Letter"=>Ok(Self::ModifyingLetter),
        "Tone_Letter"=>Ok(Self::ToneLetter),
        "Tone_Mark"=>Ok(Self::ToneMark),
        "Gemination_Mark"=>Ok(Self::GeminationMark),
        "Cantillation_Mark"=>Ok(Self::CantillationMark),
        "Register_Shifter"=>Ok(Self::RegisterShifter),
        "Syllable_Modifier"=>Ok(Self::SyllableModifier),
        "Consonant_Killer"=>Ok(Self::ConsonantKiller),
        "Non_Joiner"=>Ok(Self::NonJoiner),
        "Joiner"=>Ok(Self::Joiner),
        "Number_Joiner"=>Ok(Self::NumberJoiner),
        "Number"=>Ok(Self::Number),
        "Brahmi_Joining_Number"=>Ok(Self::BrahmiJoiningNumber),
        _=>Err(Error::Invalid("Indic property value")),
    }}
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum IndicPositionalCategory{NotApplicable,Right,Left,VisualOrderLeft,LeftAndRight,Top,Bottom,TopAndBottom,TopAndRight,TopAndLeft,TopAndLeftAndRight,BottomAndRight,BottomAndLeft,TopAndBottomAndRight,TopAndBottomAndLeft,Overstruck}
impl IndicPositionalCategory{
    pub fn name(self)->&'static str{match self{
        Self::NotApplicable=>"Not_Applicable",
        Self::Right=>"Right",
        Self::Left=>"Left",
        Self::VisualOrderLeft=>"Visual_Order_Left",
        Self::LeftAndRight=>"Left_And_Right",
        Self::Top=>"Top",
        Self::Bottom=>"Bottom",
        Self::TopAndBottom=>"Top_And_Bottom",
        Self::TopAndRight=>"Top_And_Right",
        Self::TopAndLeft=>"Top_And_Left",
        Self::TopAndLeftAndRight=>"Top_And_Left_And_Right",
        Self::BottomAndRight=>"Bottom_And_Right",
        Self::BottomAndLeft=>"Bottom_And_Left",
        Self::TopAndBottomAndRight=>"Top_And_Bottom_And_Right",
        Self::TopAndBottomAndLeft=>"Top_And_Bottom_And_Left",
        Self::Overstruck=>"Overstruck",
    }}
}
impl Property for IndicPositionalCategory{
    const DEFAULT:Self=Self::NotApplicable;
    fn parse(s:&str)->Result<Self,Error>{match s{
        "Not_Applicable"=>Ok(Self::NotApplicable),
        "Right"=>Ok(Self::Right),
        "Left"=>Ok(Self::Left),
        "Visual_Order_Left"=>Ok(Self::VisualOrderLeft),
        "Left_And_Right"=>Ok(Self::LeftAndRight),
        "Top"=>Ok(Self::Top),
        "Bottom"=>Ok(Self::Bottom),
        "Top_And_Bottom"=>Ok(Self::TopAndBottom),
        "Top_And_Right"=>Ok(Self::TopAndRight),
        "Top_And_Left"=>Ok(Self::TopAndLeft),
        "Top_And_Left_And_Right"=>Ok(Self::TopAndLeftAndRight),
        "Bottom_And_Right"=>Ok(Self::BottomAndRight),
        "Bottom_And_Left"=>Ok(Self::BottomAndLeft),
        "Top_And_Bottom_And_Right"=>Ok(Self::TopAndBottomAndRight),
        "Top_And_Bottom_And_Left"=>Ok(Self::TopAndBottomAndLeft),
        "Overstruck"=>Ok(Self::Overstruck),
        _=>Err(Error::Invalid("Indic property value")),
    }}
}

#[derive(Debug,Clone,Copy)]
pub struct IndicDataLimits{pub max_bytes:usize,pub max_ranges:usize}
impl Default for IndicDataLimits{
    fn default()->Self{Self{max_bytes:2*1024*1024,max_ranges:32768}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
struct Range<T>{first:u32,last:u32,value:T}
#[derive(Debug,Clone,PartialEq,Eq)]
struct Index<T>{ranges:Vec<Range<T>>}
fn point(s:&str)->Result<u32,Error>{
    let s=s.trim();
    if s.is_empty()||s.len()>6||!s.bytes().all(|b|b.is_ascii_hexdigit()){
        return Err(Error::Invalid("Indic property codepoint"));
    }
    let n=u32::from_str_radix(s,16).map_err(|_|Error::Invalid("Indic property codepoint"))?;
    if n>0x10ffff{return Err(Error::Invalid("Indic property codepoint"));}Ok(n)
}
fn record<T:Property>(s:&str)->Result<Range<T>,Error>{
    let (r,v)=s.split_once(';').ok_or(Error::Invalid("Indic property fields"))?;
    let (first,last)=if let Some((a,b))=r.split_once(".."){(point(a)?,point(b)?)}else{let a=point(r)?;(a,a)};
    if first>last{return Err(Error::Invalid("Indic property reversed range"));}
    Ok(Range{first,last,value:T::parse(v.trim())?})
}
fn parse<T:Property>(text:&str,l:IndicDataLimits)->Result<Index<T>,Error>{
    if text.len()>l.max_bytes{return Err(Error::Limit("Indic property bytes"));}
    let mut ranges=Vec::new();let mut missing=false;
    for raw in text.lines(){
        let raw=raw.trim();
        if let Some(m)=raw.strip_prefix('#').map(str::trim).and_then(|s|s.strip_prefix("@missing:")){
            let r=record::<T>(m.split('#').next().unwrap_or(""))?;
            if missing||r.first!=0||r.last!=0x10ffff||r.value!=T::DEFAULT{
                return Err(Error::Invalid("Indic property default"));
            }missing=true;continue;
        }
        let s=raw.split('#').next().unwrap_or("").trim();if s.is_empty(){continue;}
        if ranges.len()>=l.max_ranges{return Err(Error::Limit("Indic property ranges"));}
        let r=record::<T>(s)?;
        if r.first<=0xdfff&&r.last>=0xd800&&r.value!=T::DEFAULT{
            return Err(Error::Invalid("Indic property surrogate"));
        }
        ranges.try_reserve(1).map_err(|_|Error::Limit("Indic property allocation"))?;
        ranges.push(r);
    }
    ranges.sort_unstable_by_key(|r|r.first);
    if ranges.windows(2).any(|w|w[0].last>=w[1].first){return Err(Error::Invalid("Indic property overlap"));}
    Ok(Index{ranges})
}
impl<T:Property> Index<T>{
    fn get(&self,ch:char)->T{
        let n=u32::from(ch);let i=self.ranges.partition_point(|r|r.last<n);
        self.ranges.get(i).filter(|r|r.first<=n).map_or(T::DEFAULT,|r|r.value)
    }
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct IndicSyllabicData(Index<IndicSyllabicCategory>);
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct IndicPositionalData(Index<IndicPositionalCategory>);
impl IndicSyllabicData{
    pub fn get(&self,ch:char)->IndicSyllabicCategory{self.0.get(ch)}
    pub fn range_count(&self)->usize{self.0.ranges.len()}
}
impl IndicPositionalData{
    pub fn get(&self,ch:char)->IndicPositionalCategory{self.0.get(ch)}
    pub fn range_count(&self)->usize{self.0.ranges.len()}
}
pub fn parse_indic_syllabic(text:&str,l:IndicDataLimits)->Result<IndicSyllabicData,Error>{Ok(IndicSyllabicData(parse(text,l)?))}
pub fn parse_indic_positional(text:&str,l:IndicDataLimits)->Result<IndicPositionalData,Error>{Ok(IndicPositionalData(parse(text,l)?))}
pub const BUNDLED_INDIC_SYLLABIC:&str=include_str!("../data/unicode/17.0.0/IndicSyllabicCategory.txt");
pub const BUNDLED_INDIC_POSITIONAL:&str=include_str!("../data/unicode/17.0.0/IndicPositionalCategory.txt");
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct IndicProperties{pub syllabic:IndicSyllabicData,pub positional:IndicPositionalData}
pub fn bundled_indic_properties()->Result<IndicProperties,Error>{
    Ok(IndicProperties{syllabic:parse_indic_syllabic(BUNDLED_INDIC_SYLLABIC,Default::default())?,
        positional:parse_indic_positional(BUNDLED_INDIC_POSITIONAL,Default::default())?})
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct IndicScalar{
    pub character:char,pub byte_start:usize,pub byte_end:usize,
    pub syllabic:IndicSyllabicCategory,pub positional:IndicPositionalCategory,
}
/// Annotate logical scalars without changing order or inventing clusters.
pub fn annotate_indic_scalars(text:&str,p:&IndicProperties,l:crate::SourceLimits)->Result<Vec<IndicScalar>,Error>{
    if text.len()>l.max_bytes{return Err(Error::Limit("Indic source bytes"));}
    let n=text.chars().count();
    if n>l.max_scalars||n>l.max_glyphs{return Err(Error::Limit("Indic source records"));}
    let mut out=Vec::new();out.try_reserve_exact(n).map_err(|_|Error::Limit("Indic source allocation"))?;
    for (at,ch) in text.char_indices(){
        out.push(IndicScalar{character:ch,byte_start:at,byte_end:at+ch.len_utf8(),
            syllabic:p.syllabic.get(ch),positional:p.positional.get(ch)});
    }Ok(out)
}
