//! Native bounded Unicode Script property range lookup (UAX #24/#44).
//! Input is an explicit Scripts.txt snapshot, not Unicode block inference.
//! No script extensions, alias normalization, bidi or script resolution yet.
use crate::Error;
#[derive(Debug,Clone,Copy)]
pub struct ScriptDataLimits{
    pub max_bytes:usize,pub max_records:usize,pub max_name_bytes:usize,
}
impl Default for ScriptDataLimits{
    fn default()->Self{Self{max_bytes:2*1024*1024,max_records:32768,max_name_bytes:64}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
struct ScriptRange{first:u32,last:u32,name:String}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct UnicodeScripts{ranges:Vec<ScriptRange>}
fn codepoint(s:&str)->Result<u32,Error>{
    let s=s.trim();
    if s.is_empty()||s.len()>6||!s.bytes().all(|b|b.is_ascii_hexdigit()){
        return Err(Error::Invalid("unicode script code point"));
    }
    let n=u32::from_str_radix(s,16).map_err(|_|Error::Invalid("unicode script code point"))?;
    if n>0x10ffff{return Err(Error::Invalid("unicode script code point"));}
    Ok(n)
}
fn range(s:&str)->Result<(u32,u32),Error>{
    let (a,b)=match s.split_once(".."){
        Some((a,b))=>(codepoint(a)?,codepoint(b)?),
        None=>{let a=codepoint(s)?;(a,a)}
    };
    if a>b{return Err(Error::Invalid("unicode script reversed range"));}
    Ok((a,b))
}
fn fields(s:&str)->Result<((u32,u32),&str),Error>{
    let (r,n)=s.split_once(';').ok_or(Error::Invalid("unicode script fields"))?;
    Ok((range(r)?,n.trim()))
}
/// Parse the canonical two-field UCD Scripts.txt format. Entries can be
/// grouped by script rather than code point. Only the global Unknown
/// default is supported; other @missing directives are rejected.
pub fn parse_unicode_scripts(text:&str,l:ScriptDataLimits)->Result<UnicodeScripts,Error>{
    if text.len()>l.max_bytes{return Err(Error::Limit("unicode script bytes"));}
    let mut ranges=Vec::new();let mut default_seen=false;
    for raw in text.lines(){
        let line=raw.trim();
        if let Some(m)=line.strip_prefix('#').map(str::trim).and_then(|s|s.strip_prefix("@missing:")){
            let ((a,b),name)=fields(m.split('#').next().unwrap_or(""))?;
            if default_seen||a!=0||b!=0x10ffff||name!="Unknown"{
                return Err(Error::Invalid("unicode script default"));
            }
            default_seen=true;continue;
        }
        let line=line.split('#').next().unwrap_or("").trim();
        if line.is_empty(){continue;}
        if ranges.len()>=l.max_records{return Err(Error::Limit("unicode script records"));}
        let ((first,last),name)=fields(line)?;
        if name.len()>l.max_name_bytes{return Err(Error::Limit("unicode script name"));}
        if name.is_empty()||!name.as_bytes()[0].is_ascii_alphabetic()||
            !name.bytes().all(|b|b.is_ascii_alphabetic()||b.is_ascii_digit()||b==b'_'){
            return Err(Error::Invalid("unicode script name"));
        }
        if first<=0xdfff&&last>=0xd800&&name!="Unknown"{
            return Err(Error::Invalid("unicode script surrogate range"));
        }
        let mut owned=String::new();owned.try_reserve_exact(name.len()).map_err(|_|Error::Limit("unicode script allocation"))?;
        owned.push_str(name);
        ranges.try_reserve(1).map_err(|_|Error::Limit("unicode script allocation"))?;
        ranges.push(ScriptRange{first,last,name:owned});
    }
    ranges.sort_unstable_by_key(|r|r.first);
    if ranges.windows(2).any(|w|w[0].last>=w[1].first){
        return Err(Error::Invalid("unicode script overlapping ranges"));
    }
    Ok(UnicodeScripts{ranges})
}
impl UnicodeScripts{
    pub fn range_count(&self)->usize{self.ranges.len()}
    /// Query only valid Unicode scalars. Missing entries return Unknown.
    pub fn script(&self,ch:char)->&str{
        let n=u32::from(ch);
        let i=self.ranges.partition_point(|r|r.last<n);
        self.ranges.get(i).filter(|r|r.first<=n).map_or("Unknown",|r|r.name.as_str())
    }
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ScriptSpan<'a>{
    pub script:&'a str,pub scalar_start:usize,pub scalar_end:usize,
    pub byte_start:usize,pub byte_end:usize,
}
/// Raw adjacent equal Script values in logical UTF-8 order. Common and
/// Inherited are intentionally not resolved to neighboring scripts.
/// Limits bound bytes, scalars and output spans separately.
pub fn raw_script_spans<'a>(text:&str,scripts:&'a UnicodeScripts,l:crate::SourceLimits)
    ->Result<Vec<ScriptSpan<'a>>,Error>{
    if text.len()>l.max_bytes{return Err(Error::Limit("unicode script source bytes"));}
    let mut out:Vec<ScriptSpan<'a>>=Vec::new();
    for (i,(at,ch)) in text.char_indices().enumerate(){
        if i>=l.max_scalars{return Err(Error::Limit("unicode script scalars"));}
        let script=scripts.script(ch);let end=at+ch.len_utf8();
        if let Some(prev)=out.last_mut().filter(|r|r.script==script){
            prev.scalar_end=i+1;prev.byte_end=end;continue;
        }
        // Reuse SourceLimits.max_glyphs as the explicit output record cap.
        if out.len()>=l.max_glyphs{return Err(Error::Limit("unicode script spans"));}
        out.try_reserve(1).map_err(|_|Error::Limit("unicode script allocation"))?;
        out.push(ScriptSpan{script,scalar_start:i,scalar_end:i+1,byte_start:at,byte_end:end});
    }
    Ok(out)
}

/// The bundled, immutable Unicode Script property snapshot.
pub const UNICODE_SCRIPT_VERSION:&str="17.0.0";
pub const BUNDLED_UNICODE_SCRIPTS:&str=include_str!("../data/unicode/17.0.0/Scripts.txt");
/// Parse the bundled full Script property dataset using the same native
/// bounded parser as external snapshots. Call once and reuse the index.
/// No runtime filesystem access, external engine or network required.
pub fn bundled_unicode_scripts()->Result<UnicodeScripts,Error>{
    parse_unicode_scripts(BUNDLED_UNICODE_SCRIPTS,ScriptDataLimits::default())
}
