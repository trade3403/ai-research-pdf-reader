//! C2.4a: offline image relationship catalog and bounded opaque binary storage.
use crate::{Error, Limits, ResourceKind, Target};
use crate::opc::{open_archive,part};
use std::collections::BTreeMap;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct MediaLimits {
    pub max_resources: usize,
    pub max_resource_bytes: usize,
    pub max_total_bytes: usize,
    pub max_relationships: usize,
}
impl Default for MediaLimits{
    fn default()->Self{
        Self{max_resources:1024,max_resource_bytes:8*1024*1024,
            max_total_bytes:16*1024*1024,max_relationships:16384}
    }
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ImageFormat { Png, Jpeg, Gif, Bmp, Tiff, WebP }
impl ImageFormat{
    pub fn mime(self)->&'static str{
        match self{
            Self::Png=>"image/png",Self::Jpeg=>"image/jpeg",Self::Gif=>"image/gif",
            Self::Bmp=>"image/bmp",Self::Tiff=>"image/tiff",Self::WebP=>"image/webp",
        }
    }
    fn from_mime(mime:&str)->Result<Self,Error>{
        match mime{
            "image/png"=>Ok(Self::Png),"image/jpeg"=>Ok(Self::Jpeg),"image/gif"=>Ok(Self::Gif),
            "image/bmp"=>Ok(Self::Bmp),"image/tiff"=>Ok(Self::Tiff),"image/webp"=>Ok(Self::WebP),
            _=>Err(Error::Unsupported("image content type")),
        }
    }
    /// Preliminary signature check only. Never implies a decodable or safe image.
    pub fn matches_signature(self,bytes:&[u8])->bool{
        match self{
            Self::Png=>bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            Self::Jpeg=>bytes.starts_with(b"\xff\xd8\xff"),
            Self::Gif=>bytes.starts_with(b"GIF87a")||bytes.starts_with(b"GIF89a"),
            Self::Bmp=>bytes.starts_with(b"BM"),
            Self::Tiff=>bytes.starts_with(b"II\x2a\x00")||bytes.starts_with(b"MM\x00\x2a"),
            Self::WebP=>bytes.starts_with(b"RIFF")&&bytes.get(8..12)==Some(b"WEBP".as_slice()),
        }
    }
}
#[derive(Debug,PartialEq,Eq)]
pub struct MediaResource{
    pub format: ImageFormat,
    /// Opaque compressed/container bytes, not decoded pixels.
    pub bytes: Vec<u8>,
}
#[derive(Debug,PartialEq,Eq)]
pub struct MediaCatalog{
    pub main_part: String,
    resources: BTreeMap<String,MediaResource>,
    relationships: BTreeMap<String,BTreeMap<String,Target>>,
    total_bytes:usize,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum MediaBinding<'a>{
    Embedded{part_name:&'a str,resource:&'a MediaResource},
    /// Inert string, never fetched, opened or executed.
    External{target:&'a str},
}
impl MediaCatalog{
    pub fn resources(&self)->&BTreeMap<String,MediaResource>{&self.resources}
    pub fn total_bytes(&self)->usize{self.total_bytes}
    pub fn relationship_count(&self)->usize{self.relationships.values().map(BTreeMap::len).sum()}
    /// IDs are scoped to their owning OPC source part, not globally unique.
    pub fn resolve(&self,source:&str,id:&str)->Option<MediaBinding<'_>>{
        match self.relationships.get(source)?.get(id)?{
            Target::Internal(name)=>Some(MediaBinding::Embedded{part_name:name,
                resource:self.resources.get(name)?}),
            Target::External(target)=>Some(MediaBinding::External{target}),
        }
    }
}

/// Explicit opt-in resource reader. Reuses OPC/ZIP path, graph, expansion and
/// entry safety checks. Loads every internal image relationship target once
/// by canonical part name, including relationships owned by non-main parts.
/// Unreferenced parts are not loaded. Unsupported image MIME or signature
/// mismatch fails this reader; existing text readers remain unchanged.
/// No image decoding, dimension parsing, filesystem extraction or networking.
/// Bounds apply before loading each binary; aggregate counts unique parts.
pub fn load_media(bytes:&[u8],limits:Limits,media_limits:MediaLimits)->Result<MediaCatalog,Error>{
    let mut zip=open_archive(bytes,limits)?;
    let graph=crate::package::graph(&mut zip,limits)?;
    let mut catalog=MediaCatalog{main_part:graph.main_part.clone(),
        resources:BTreeMap::new(),relationships:BTreeMap::new(),total_bytes:0};
    let mut count=0usize;
    for rel in graph.relationships.iter().filter(|r|r.kind==ResourceKind::Image){
        if count>=media_limits.max_relationships{return Err(Error::Limit("media relationships"));}
        count+=1;
        if let Target::Internal(name)=&rel.target{
            if !catalog.resources.contains_key(name){
                if catalog.resources.len()>=media_limits.max_resources{
                    return Err(Error::Limit("media resources"));
                }
                let mime=graph.parts.get(name).and_then(|m|m.as_deref())
                    .ok_or(Error::Invalid("missing image content type"))?;
                let format=ImageFormat::from_mime(mime)?;
                let size={
                    let f=zip.by_name(name).map_err(|_|Error::MissingNamedPart(name.clone()))?;
                    usize::try_from(f.size()).map_err(|_|Error::Limit("media resource bytes"))?
                };
                let cap=limits.max_part_bytes.min(media_limits.max_resource_bytes);
                if size>cap{return Err(Error::Limit("media resource bytes"));}
                let total=catalog.total_bytes.checked_add(size)
                    .filter(|v|*v<=media_limits.max_total_bytes)
                    .ok_or(Error::Limit("total media bytes"))?;
                let data=part(&mut zip,name,Limits{max_part_bytes:cap,..limits})?;
                if data.len()!=size{return Err(Error::Invalid("media size mismatch"));}
                if !format.matches_signature(&data){
                    return Err(Error::Invalid("image signature mismatch"));
                }
                catalog.resources.insert(name.clone(),MediaResource{format,bytes:data});
                catalog.total_bytes=total;
            }
        }
        let ids=catalog.relationships.entry(rel.source.clone()).or_default();
        if ids.insert(rel.id.clone(),rel.target.clone()).is_some(){
            return Err(Error::Invalid("duplicate media relationship"));
        }
    }
    Ok(catalog)
}
