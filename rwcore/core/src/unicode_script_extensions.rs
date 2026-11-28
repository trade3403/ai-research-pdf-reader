//! Unicode Script_Extensions and Script value aliases, UCD 17.0.0.
//! Explicit sets override the primary Script; absent entries inherit it.
//! This is property lookup, not contextual script resolution.
use crate::{Error,UnicodeScripts,bundled_unicode_scripts};
use std::collections::BTreeMap;
#[derive(Debug,Clone,Copy)]
pub struct ScriptExtensionLimits{
    pub max_bytes:usize,pub max_aliases:usize,pub max_ranges:usize,
    pub max_set_members:usize,pub max_total_members:usize,pub max_name_bytes:usize,
}
impl Default for ScriptExtensionLimits{
    fn default()->Self{Self{max_bytes:2*1024*1024,max_aliases:4096,max_ranges:32768,
        max_set_members:256,max_total_members:65536,max_name_bytes:128}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ScriptValue{pub short:String,pub long:String}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ScriptAliases{values:Vec<ScriptValue>,index:BTreeMap<String,usize>,max_name_bytes:usize}
fn alias_key(s:&str,max:usize)->Result<String,Error>{
    if s.len()>max{return Err(Error::Limit("script alias length"));}
    let mut key=String::new();
    key.try_reserve_exact(s.len()).map_err(|_|Error::Limit("script alias allocation"))?;
    for b in s.bytes(){
        match b{b'A'..=b'Z'=>key.push(b.to_ascii_lowercase() as char),
            b'a'..=b'z'|b'0'..=b'9'=>key.push(b as char),
            b'_'|b'-'|b' '|b'\t'|b'\r'|b'\n'|0x0b|0x0c=>{},
            _=>return Err(Error::Invalid("script alias character"))}
    }
    if key.is_empty(){return Err(Error::Invalid("script alias empty"));}
    Ok(key)
}
impl ScriptAliases{
    pub fn values(&self)->&[ScriptValue]{&self.values}
    pub fn resolve(&self,name:&str)->Result<Option<&ScriptValue>,Error>{
        Ok(self.index.get(&alias_key(name,self.max_name_bytes)?).map(|&i|&self.values[i]))
    }
}
/// Read only sc records from PropertyValueAliases.txt; ignore other properties.
pub fn parse_script_aliases(text:&str,l:ScriptExtensionLimits)->Result<ScriptAliases,Error>{
    if text.len()>l.max_bytes{return Err(Error::Limit("script alias bytes"));}
    let mut values=Vec::new();let mut index=BTreeMap::new();
    for raw in text.lines(){
        let line=raw.split('#').next().unwrap_or("").trim();
        if line.is_empty(){continue;}
        let mut fields=line.split(';').map(str::trim);
        if fields.next()!=Some("sc"){continue;}
        let short=fields.next().ok_or(Error::Invalid("script alias short"))?;
        let long=fields.next().ok_or(Error::Invalid("script alias long"))?;
        if short.len()!=4||!short.bytes().all(|b|b.is_ascii_alphabetic()){
            return Err(Error::Invalid("script alias short"));
        }
        let id=values.len();
        if id>=l.max_aliases{return Err(Error::Limit("script alias records"));}
        for name in std::iter::once(short).chain(std::iter::once(long)).chain(fields){
            let key=alias_key(name,l.max_name_bytes)?;
            if let Some(&old)=index.get(&key){
                if old!=id{return Err(Error::Invalid("script alias collision"));}
            }else{
                if index.len()>=l.max_aliases{return Err(Error::Limit("script alias keys"));}
                index.insert(key,id);
            }
        }
        values.try_reserve(1).map_err(|_|Error::Limit("script alias allocation"))?;
        values.push(ScriptValue{short:short.into(),long:long.into()});
    }
    Ok(ScriptAliases{values,index,max_name_bytes:l.max_name_bytes})
}
#[derive(Debug,Clone,PartialEq,Eq)]
struct ExtensionRange{first:u32,last:u32,members:Vec<String>}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct UnicodeScriptExtensions{ranges:Vec<ExtensionRange>}
fn point(s:&str)->Result<u32,Error>{
    let s=s.trim();
    if s.is_empty()||s.len()>6||!s.bytes().all(|b|b.is_ascii_hexdigit()){
        return Err(Error::Invalid("script extension codepoint"));
    }
    let n=u32::from_str_radix(s,16).map_err(|_|Error::Invalid("script extension codepoint"))?;
    if n>0x10ffff{return Err(Error::Invalid("script extension codepoint"));}Ok(n)
}
fn bounds(s:&str)->Result<(u32,u32),Error>{
    let (a,b)=if let Some((a,b))=s.split_once(".."){(point(a)?,point(b)?)}else{let a=point(s)?;(a,a)};
    if a>b{return Err(Error::Invalid("script extension reversed range"));}Ok((a,b))
}
pub fn parse_script_extensions(text:&str,aliases:&ScriptAliases,l:ScriptExtensionLimits)->Result<UnicodeScriptExtensions,Error>{
    if text.len()>l.max_bytes{return Err(Error::Limit("script extension bytes"));}
    let mut ranges=Vec::new();let mut total=0usize;let mut default_seen=false;
    for raw in text.lines(){
        let raw=raw.trim();
        if let Some(m)=raw.strip_prefix('#').map(str::trim).and_then(|s|s.strip_prefix("@missing:")){
            let (r,v)=m.split('#').next().unwrap_or("").split_once(';').ok_or(Error::Invalid("script extension default"))?;
            if default_seen||bounds(r)?!=(0,0x10ffff)||v.trim()!="<script>"{
                return Err(Error::Invalid("script extension default"));
            }default_seen=true;continue;
        }
        let line=raw.split('#').next().unwrap_or("").trim();
        if line.is_empty(){continue;}
        if ranges.len()>=l.max_ranges{return Err(Error::Limit("script extension ranges"));}
        let (r,v)=line.split_once(';').ok_or(Error::Invalid("script extension fields"))?;
        let (first,last)=bounds(r)?;
        if first<=0xdfff&&last>=0xd800{return Err(Error::Invalid("script extension surrogate"));}
        let mut members=Vec::new();
        for name in v.split_ascii_whitespace(){
            if members.len()>=l.max_set_members||total>=l.max_total_members{
                return Err(Error::Limit("script extension members"));
            }
            if name.len()>l.max_name_bytes{return Err(Error::Limit("script extension name"));}
            let resolved=aliases.resolve(name)?.ok_or(Error::Invalid("script extension unknown alias"))?;
            if matches!(resolved.short.as_str(),"Zyyy"|"Zinh"|"Zzzz"){
                return Err(Error::Invalid("script extension special value"));
            }
            members.try_reserve(1).map_err(|_|Error::Limit("script extension allocation"))?;
            members.push(resolved.long.clone());total+=1;
        }
        if members.is_empty(){return Err(Error::Invalid("script extension empty set"));}
        members.sort_unstable();
        if members.windows(2).any(|w|w[0]==w[1]){return Err(Error::Invalid("script extension duplicate member"));}
        ranges.try_reserve(1).map_err(|_|Error::Limit("script extension allocation"))?;
        ranges.push(ExtensionRange{first,last,members});
    }
    ranges.sort_unstable_by_key(|r|r.first);
    if ranges.windows(2).any(|w|w[0].last>=w[1].first){return Err(Error::Invalid("script extension overlap"));}
    Ok(UnicodeScriptExtensions{ranges})
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ScriptSet<'a>{Primary(&'a str),Explicit(&'a [String])}
impl<'a> ScriptSet<'a>{
    pub fn contains(&self,name:&str)->bool{match self{
        Self::Primary(s)=>*s==name,Self::Explicit(s)=>s.binary_search_by(|v|v.as_str().cmp(name)).is_ok()}}
    pub fn len(&self)->usize{match self{Self::Primary(_)=>1,Self::Explicit(s)=>s.len()}}
    pub fn is_empty(&self)->bool{self.len()==0}
    pub fn get(&self,i:usize)->Option<&'a str>{match self{
        Self::Primary(s) if i==0=>Some(*s),Self::Primary(_)=>None,Self::Explicit(s)=>s.get(i).map(String::as_str)}}
}
impl UnicodeScriptExtensions{
    pub fn range_count(&self)->usize{self.ranges.len()}
    pub fn explicit(&self,ch:char)->Option<&[String]>{
        let n=u32::from(ch);let i=self.ranges.partition_point(|r|r.last<n);
        self.ranges.get(i).filter(|r|r.first<=n).map(|r|r.members.as_slice())
    }
    pub fn scripts<'a>(&'a self,ch:char,primary:&'a UnicodeScripts)->ScriptSet<'a>{
        match self.explicit(ch){Some(s)=>ScriptSet::Explicit(s),None=>ScriptSet::Primary(primary.script(ch))}
    }
}
pub const BUNDLED_SCRIPT_ALIASES:&str=include_str!("../data/unicode/17.0.0/PropertyValueAliases.txt");
pub const BUNDLED_SCRIPT_EXTENSIONS:&str=include_str!("../data/unicode/17.0.0/ScriptExtensions.txt");
#[derive(Debug)]
pub struct UnicodeScriptProperties{
    pub primary:UnicodeScripts,pub aliases:ScriptAliases,pub extensions:UnicodeScriptExtensions,
}
pub fn bundled_script_properties()->Result<UnicodeScriptProperties,Error>{
    let primary=bundled_unicode_scripts()?;
    let aliases=parse_script_aliases(BUNDLED_SCRIPT_ALIASES,Default::default())?;
    let extensions=parse_script_extensions(BUNDLED_SCRIPT_EXTENSIONS,&aliases,Default::default())?;
    Ok(UnicodeScriptProperties{primary,aliases,extensions})
}
