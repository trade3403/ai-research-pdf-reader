//! GDEF 1.0/1.2/1.3 directory, bounded glyph class definitions and mark sets.
//! Other child offsets are validated at the entry only, not decoded.
use crate::{Error,GlyphClassDef,parse_glyph_class_def,MarkGlyphSets,MarkGlyphSetLimits,parse_mark_glyph_sets};

#[derive(Debug,Clone,Copy)]
pub struct GdefLimits{
    pub max_bytes:usize,
    pub max_class_records:usize,
    pub max_assigned_glyphs:u32,
}
impl Default for GdefLimits{
    fn default()->Self{Self{max_bytes:1024*1024,max_class_records:65535,max_assigned_glyphs:131072}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GdefDirectory{
    pub minor_version:u16,
    pub glyph_class_def:Option<usize>,
    pub attach_list:Option<usize>,
    pub lig_caret_list:Option<usize>,
    pub mark_attach_class_def:Option<usize>,
    pub mark_glyph_sets_def:Option<usize>,
    pub item_var_store:Option<usize>,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum GdefGlyphClass{Unclassified,Base,Ligature,Mark,Component}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Gdef{
    directory:GdefDirectory,
    glyph_count:u32,
    glyph_classes:Option<GlyphClassDef>,
    mark_attach_classes:Option<GlyphClassDef>,
    mark_sets:Option<MarkGlyphSets>,
}
impl Gdef{
    pub fn mark_sets(&self)->Option<&MarkGlyphSets>{self.mark_sets.as_ref()}
    pub fn directory(&self)->&GdefDirectory{&self.directory}
    pub fn glyph_classes(&self)->Option<&GlyphClassDef>{self.glyph_classes.as_ref()}
    pub fn mark_attach_classes(&self)->Option<&GlyphClassDef>{self.mark_attach_classes.as_ref()}
    /// Missing table or implicit class zero is unclassified; out-of-face is None.
    pub fn glyph_class(&self,id:u16)->Option<GdefGlyphClass>{
        if u32::from(id)>=self.glyph_count{return None;}
        let c=self.glyph_classes.as_ref().and_then(|x|x.class(id)).unwrap_or(0);
        Some(match c{
            0=>GdefGlyphClass::Unclassified,1=>GdefGlyphClass::Base,
            2=>GdefGlyphClass::Ligature,3=>GdefGlyphClass::Mark,
            4=>GdefGlyphClass::Component,_=>return None,
        })
    }
    /// Independent MarkAttachClassDef domain (0..=255). A class number alone
    /// does not classify the glyph as a mark. Missing table gives class zero.
    pub fn mark_attachment_class(&self,id:u16)->Option<u16>{
        if u32::from(id)>=self.glyph_count{return None;}
        Some(self.mark_attach_classes.as_ref().and_then(|x|x.class(id)).unwrap_or(0))
    }
}
fn slice(b:&[u8],p:usize,n:usize)->Result<&[u8],Error>{
    b.get(p..p.checked_add(n).ok_or(Error::Limit("gdef offset"))?).ok_or(Error::Invalid("gdef truncated"))
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{
    Ok(u16::from_be_bytes(slice(b,p,2)?.try_into().unwrap()))
}
fn offset(b:&[u8],p:usize,header:usize,min:usize)->Result<Option<usize>,Error>{
    if p==0{return Ok(None);}
    if p<header{return Err(Error::Invalid("gdef offset into header"));}
    slice(b,p,min)?;
    Ok(Some(p))
}
/// Nullable offsets are relative to the GDEF table. Unknown minor versions
/// are rejected. Deferred children are not certified by directory parsing.
pub fn parse_gdef_directory(b:&[u8],limits:GdefLimits)->Result<GdefDirectory,Error>{
    if b.len()>limits.max_bytes{return Err(Error::Limit("gdef bytes"));}
    let major=word(b,0)?;
    let minor=word(b,2)?;
    if major!=1{return Err(Error::Unsupported("gdef version"));}
    let header=match minor{0=>12,2=>14,3=>18,_=>return Err(Error::Unsupported("gdef version"))};
    slice(b,0,header)?;
    let read=|at,min|offset(b,usize::from(word(b,at)?),header,min);
    let glyph_class_def=read(4,4)?;
    let attach_list=read(6,4)?;
    let lig_caret_list=read(8,4)?;
    let mark_attach_class_def=read(10,4)?;
    let mark_glyph_sets_def=if minor>=2{read(12,4)?}else{None};
    let item_var_store=if minor==3{
        let p=usize::try_from(u32::from_be_bytes(slice(b,14,4)?.try_into().unwrap())).map_err(|_|Error::Limit("gdef variation offset"))?;
        offset(b,p,header,8)?
    }else{None};
    Ok(GdefDirectory{minor_version:minor,glyph_class_def,attach_list,lig_caret_list,mark_attach_class_def,mark_glyph_sets_def,item_var_store})
}
// Charge both ClassDefs cumulatively, including aliased offsets, by stored
// record count and explicit glyph slots (including explicitly stored zeros).
fn class_cost(b:&[u8])->Result<(usize,u32),Error>{
    match word(b,0)?{
        1=>{let n=usize::from(word(b,4)?);slice(b,6,2*n)?;Ok((n,n as u32))}
        2=>{
            let n=usize::from(word(b,2)?);slice(b,4,6*n)?;
            let mut total=0u32;
            for i in 0..n{
                let p=4+6*i;
                let a=u32::from(word(b,p)?);let z=u32::from(word(b,p+2)?);
                if a>z{return Err(Error::Invalid("gdef class range"));}
                total=total.checked_add(z-a+1).ok_or(Error::Limit("gdef class glyphs"))?;
            }
            Ok((n,total))
        }
        _=>Err(Error::Unsupported("gdef class format")),
    }
}
/// Parse class definitions and mark sets with their default independent budgets.
pub fn parse_gdef(b:&[u8],glyph_count:u32,limits:GdefLimits)->Result<Gdef,Error>{
    parse_gdef_with_mark_set_limits(b,glyph_count,limits,MarkGlyphSetLimits::default())
}
/// Explicit mark-set budgets; the GDEF parent byte limit still applies.
pub fn parse_gdef_with_mark_set_limits(b:&[u8],glyph_count:u32,limits:GdefLimits,mark_limits:MarkGlyphSetLimits)->Result<Gdef,Error>{
    if glyph_count==0||glyph_count>65536{return Err(Error::Invalid("gdef glyph domain"));}
    let directory=parse_gdef_directory(b,limits)?;
    let mut records=limits.max_class_records;
    let mut assigned=limits.max_assigned_glyphs;
    let mut decode=|at:Option<usize>,class_limit:u32|->Result<Option<GlyphClassDef>,Error>{
        let Some(at)=at else{return Ok(None);};
        let child=&b[at..];
        let (n,g)=class_cost(child)?;
        records=records.checked_sub(n).ok_or(Error::Limit("gdef class records"))?;
        assigned=assigned.checked_sub(g).ok_or(Error::Limit("gdef class glyphs"))?;
        Ok(Some(parse_glyph_class_def(child,glyph_count,class_limit,n,g)?))
    };
    let glyph_classes=decode(directory.glyph_class_def,5)?;
    let mark_attach_classes=decode(directory.mark_attach_class_def,256)?;
    let mark_sets=directory.mark_glyph_sets_def.map(|at|parse_mark_glyph_sets(&b[at..],glyph_count,mark_limits)).transpose()?;
    Ok(Gdef{directory,glyph_count,glyph_classes,mark_attach_classes,mark_sets})
}
