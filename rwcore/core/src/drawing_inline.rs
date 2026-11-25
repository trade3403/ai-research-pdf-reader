//! C2.4c-1: namespace-aware inline picture metadata and inert media binding.
use crate::{Error,Limits,MediaCatalog,MediaBinding};
use crate::opc::{xml,named};
use roxmltree::Node;

const W:&str="http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const WS:&str="http://purl.oclc.org/ooxml/wordprocessingml/main";
const WP:&str="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
const WPS:&str="http://purl.oclc.org/ooxml/drawingml/wordprocessingDrawing";
const A:&str="http://schemas.openxmlformats.org/drawingml/2006/main";
const AS:&str="http://purl.oclc.org/ooxml/drawingml/main";
const PIC:&str="http://schemas.openxmlformats.org/drawingml/2006/picture";
const PICS:&str="http://purl.oclc.org/ooxml/drawingml/picture";
const R:&str="http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const RS:&str="http://purl.oclc.org/ooxml/officeDocument/relationships";
const MAX_EMU:u64=27_273_042_316_900;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct PictureExtent{pub cx:u64,pub cy:u64}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct PictureIdentity{
    pub id:u32,
    pub name:String,
    pub description:Option<String>,
    pub title:Option<String>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct InlinePicture{
    pub extent:PictureExtent,
    pub document_properties:PictureIdentity,
    pub picture_properties:PictureIdentity,
    pub embed:Option<String>,
    pub link:Option<String>,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct InlineMediaBindings<'a>{
    pub embedded:Option<MediaBinding<'a>>,
    pub linked:Option<MediaBinding<'a>>,
}
/// 635 EMU = one twip. Nearest integer, ties upward if representable.
pub fn emu_to_twips(emu:u64)->Result<u32,Error>{
    let q=emu/635;
    let rounded=q+u64::from(emu%635>=318);
    u32::try_from(rounded).map_err(|_|Error::Limit("picture twips"))
}
impl PictureExtent{
    pub fn twips(self)->Result<(u32,u32),Error>{
        Ok((emu_to_twips(self.cx)?,emu_to_twips(self.cy)?))
    }
}
fn one<'a,'i>(parent:Node<'a,'i>,ns:&str,name:&str)->Result<Node<'a,'i>,Error>{
    let mut iter=parent.children().filter(|n|named(*n,ns,name));
    let node=iter.next().ok_or(Error::Invalid("missing inline picture element"))?;
    if iter.next().is_some(){return Err(Error::Invalid("duplicate inline picture element"));}
    Ok(node)
}
fn bounded(value:Option<&str>,limit:usize)->Result<Option<String>,Error>{
    value.map(|v|if v.len()>limit{Err(Error::Limit("picture attribute bytes"))}else{Ok(v.to_owned())}).transpose()
}
fn identity(n:Node<'_, '_>)->Result<PictureIdentity,Error>{
    let id=n.attribute("id").ok_or(Error::Invalid("missing picture id"))?
        .parse::<u32>().map_err(|_|Error::Invalid("invalid picture id"))?;
    let name=bounded(n.attribute("name"),4096)?.ok_or(Error::Invalid("missing picture name"))?;
    Ok(PictureIdentity{id,name,description:bounded(n.attribute("descr"),16384)?,
        title:bounded(n.attribute("title"),4096)?})
}
fn positive(n:Node<'_, '_>,key:&str)->Result<u64,Error>{
    let v=n.attribute(key).ok_or(Error::Invalid("missing picture extent"))?
        .parse::<u64>().map_err(|_|Error::Invalid("invalid picture extent"))?;
    if v==0||v>MAX_EMU{return Err(Error::Invalid("picture extent range"));}
    Ok(v)
}
fn reference(n:Node<'_, '_>,ns:&str,key:&str)->Result<Option<String>,Error>{
    let v=bounded(n.attribute((ns,key)),1024)?;
    if v.as_ref().is_some_and(|s|s.is_empty()||s.chars().any(char::is_whitespace)){
        return Err(Error::Invalid("invalid picture relationship id"));
    }
    Ok(v)
}
/// Accept a standalone wp:inline or a w:drawing containing one wp:inline.
/// Exact transitional or strict namespaces are required throughout the selected
/// path. Non-picture graphics and floating anchors are not interpreted.
/// This is a metadata subset; other attributes/effects are not validated.
pub fn parse_inline_picture(bytes:&[u8],limits:Limits)->Result<InlinePicture,Error>{
    if bytes.len()>limits.max_part_bytes{return Err(Error::Limit("part bytes"));}
    let tree=xml(bytes,limits)?;
    let root=tree.root_element();
    if named(root,W,"drawing")||named(root,WS,"drawing"){
        parse_drawing_node(root)
    }else{parse_inline_picture_node(root)}
}
fn parse_drawing_node(root:Node<'_, '_>)->Result<InlinePicture,Error>{
    let wp=if named(root,W,"drawing"){WP}
        else if named(root,WS,"drawing"){WPS}
        else{return Err(Error::Invalid("drawing namespace"));};
    if root.children().filter(|n|n.is_element()).count()!=1{
        return Err(Error::Unsupported("multiple drawing children"));
    }
    parse_inline_picture_node(one(root,wp,"inline")?)
}
pub(crate) fn append_run_picture(run:&mut crate::Run,node:Node<'_, '_>)->Result<(),Error>{
    let picture=parse_drawing_node(node)?;
    run.pictures.push(crate::InlinePictureAt{text_offset:run.text.len(),picture});
    Ok(())
}
pub(crate) fn parse_inline_picture_node(inline:Node<'_, '_>)->Result<InlinePicture,Error>{
    let (wp,a,pic,r)=if named(inline,WP,"inline"){(WP,A,PIC,R)}
        else if named(inline,WPS,"inline"){(WPS,AS,PICS,RS)}
        else{return Err(Error::Unsupported("inline picture root"));};
    parse_picture_metadata(inline,wp,a,pic,r)
}
/// Shared payload parsing; an anchor is never accepted by the inline API.
pub(crate) fn parse_anchor_picture_node(anchor:Node<'_, '_>)->Result<InlinePicture,Error>{
    let (wp,a,pic,r)=if named(anchor,WP,"anchor"){(WP,A,PIC,R)}
        else if named(anchor,WPS,"anchor"){(WPS,AS,PICS,RS)}
        else{return Err(Error::Unsupported("floating picture root"));};
    parse_picture_metadata(anchor,wp,a,pic,r)
}
fn parse_picture_metadata(inline:Node<'_, '_>,wp:&str,a:&str,pic:&str,r:&str)->Result<InlinePicture,Error>{
    let extent=one(inline,wp,"extent")?;
    let extent=PictureExtent{cx:positive(extent,"cx")?,cy:positive(extent,"cy")?};
    let document_properties=identity(one(inline,wp,"docPr")?)?;
    let graphic=one(inline,a,"graphic")?;
    let data=one(graphic,a,"graphicData")?;
    if data.attribute("uri")!=Some(pic){return Err(Error::Unsupported("non-picture graphic data"));}
    if data.children().filter(|n|n.is_element()).count()!=1{
        return Err(Error::Unsupported("multiple graphic data children"));
    }
    let picture=one(data,pic,"pic")?;
    let nv=one(picture,pic,"nvPicPr")?;
    let picture_properties=identity(one(nv,pic,"cNvPr")?)?;
    let fill=one(picture,pic,"blipFill")?;
    let blip=one(fill,a,"blip")?;
    let embed=reference(blip,r,"embed")?;
    let link=reference(blip,r,"link")?;
    if embed.is_none()&&link.is_none(){return Err(Error::Invalid("missing picture relationship"));}
    Ok(InlinePicture{extent,document_properties,picture_properties,embed,link})
}
impl InlinePicture{
    /// Resolve each reference using its actual source part. Embedded references
    /// must be internal; linked references must be external in this subset.
    /// Both may coexist; the external string remains inert.
    pub fn bind<'a>(&self,catalog:&'a MediaCatalog,source:&str)->Result<InlineMediaBindings<'a>,Error>{
        let embedded=match self.embed.as_deref(){
            None=>None,
            Some(id)=>match catalog.resolve(source,id){
                Some(b@MediaBinding::Embedded{..})=>Some(b),
                Some(_)=>return Err(Error::Invalid("embedded picture target mode")),
                None=>return Err(Error::Invalid("missing embedded picture resource")),
            },
        };
        let linked=match self.link.as_deref(){
            None=>None,
            Some(id)=>match catalog.resolve(source,id){
                Some(b@MediaBinding::External{..})=>Some(b),
                Some(_)=>return Err(Error::Invalid("linked picture target mode")),
                None=>return Err(Error::Invalid("missing linked picture resource")),
            },
        };
        Ok(InlineMediaBindings{embedded,linked})
    }
}
