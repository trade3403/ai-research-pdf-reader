//! C2.4f: integrate picture metadata into existing document and resource views.
//! All parsing uses the already bounded XML tree; no fragment serialization.
use crate::{AnchorLayoutContext,Block,Document,Error,FloatingPicture,FloatingPlacement,
    ImageDimensions,ImageLimits,InlineMediaBindings,InlinePicture,InlinePictureAt,
    Limits,MediaBinding,MediaCatalog,MediaLimits,PictureGeometry,PictureMapping,Run};
use crate::opc::named;
use roxmltree::Node;

const W:&str="http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const WS:&str="http://purl.oclc.org/ooxml/wordprocessingml/main";
const WP:&str="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
const WPS:&str="http://purl.oclc.org/ooxml/drawingml/wordprocessingDrawing";
const A:&str="http://schemas.openxmlformats.org/drawingml/2006/main";
const AS:&str="http://purl.oclc.org/ooxml/drawingml/main";
const PIC:&str="http://schemas.openxmlformats.org/drawingml/2006/picture";
const PICS:&str="http://purl.oclc.org/ooxml/drawingml/picture";

#[derive(Debug,Clone,PartialEq,Eq)]
pub enum PictureGeometryState{
    Absent,
    Parsed(PictureGeometry),
    /// Keep the picture/resource even when its explicit geometry is unsupported.
    Unavailable(String),
}
static ABSENT:PictureGeometryState=PictureGeometryState::Absent;
#[derive(Debug,Clone,PartialEq,Eq)]
pub enum RunDrawingKind{
    Inline(usize),
    Floating(Box<FloatingPicture>),
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct RunDrawingAt{
    pub text_offset:usize,pub kind:RunDrawingKind,pub geometry:PictureGeometryState,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum PictureKindRef<'a>{
    Inline(&'a InlinePicture),
    Floating(&'a FloatingPicture),
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct PictureAtRef<'a>{
    pub text_offset:usize,pub kind:PictureKindRef<'a>,pub geometry:&'a PictureGeometryState,
}
impl<'a> PictureAtRef<'a>{
    pub fn payload(self)->&'a InlinePicture{
        match self.kind{PictureKindRef::Inline(p)=>p,PictureKindRef::Floating(p)=>&p.picture}
    }
    pub fn floating_placement(self,context:&AnchorLayoutContext)->Result<Option<FloatingPlacement>,Error>{
        match self.kind{PictureKindRef::Inline(_)=>Ok(None),
            PictureKindRef::Floating(p)=>crate::place_floating_picture(p,context).map(Some)}
    }
    /// Map a picture at a caller-supplied physical-page origin. Only the
    /// unambiguous zero shape offset and equal wp:extent/xfrm ext subset is
    /// reconciled here; do not silently choose between conflicting frames.
    pub fn mapping_at(self,x:i64,y:i64)->Result<Option<PictureMapping>,Error>{
        let mut g=match self.geometry{
            PictureGeometryState::Absent=>return Ok(None),
            PictureGeometryState::Unavailable(_)=>return Err(Error::Unsupported("picture geometry unavailable")),
            PictureGeometryState::Parsed(g)=>*g,
        };
        let e=self.payload().extent;
        if g.transform.x!=0||g.transform.y!=0||g.transform.width!=e.cx||g.transform.height!=e.cy{
            return Err(Error::Unsupported("picture shape and drawing frame mismatch"));
        }
        g.transform.x=x;g.transform.y=y;
        g.mapping().map(Some)
    }
}
fn one<'a,'i>(n:Node<'a,'i>,ns:&str,key:&str)->Result<Node<'a,'i>,Error>{
    let mut it=n.children().filter(|c|named(*c,ns,key));
    let first=it.next().ok_or(Error::Invalid("missing integrated drawing element"))?;
    if it.next().is_some(){return Err(Error::Invalid("duplicate integrated drawing element"));}
    Ok(first)
}
fn geometry(node:Node<'_, '_>,a:&str,pic:&str)->PictureGeometryState{
    let result=(||{
        let graphic=one(node,a,"graphic")?;
        let data=one(graphic,a,"graphicData")?;
        let picture=one(data,pic,"pic")?;
        // Empty spPr in older metadata-only fixtures is not an explicit transform.
        let has_geometry=picture.children().filter(|n|named(*n,pic,"blipFill")||named(*n,pic,"spPr"))
            .any(|n|n.children().any(|c|c.is_element()&&
                matches!(c.tag_name().name(),"srcRect"|"xfrm"|"tile"|"stretch")));
        if !has_geometry{return Ok(PictureGeometryState::Absent);}
        crate::picture_geometry::parse_picture_geometry_node(picture).map(PictureGeometryState::Parsed)
    })();
    result.unwrap_or_else(|e:Error|PictureGeometryState::Unavailable(e.to_string()))
}
pub(crate) fn append_run_drawing(run:&mut Run,node:Node<'_, '_>)->Result<(),Error>{
    let (wp,a,pic)=if named(node,W,"drawing"){(WP,A,PIC)}
        else if named(node,WS,"drawing"){(WPS,AS,PICS)}
        else{return Err(Error::Invalid("drawing namespace"));};
    let mut children=node.children().filter(|n|n.is_element());
    let child=children.next().ok_or(Error::Invalid("missing drawing child"))?;
    if children.next().is_some(){return Err(Error::Unsupported("multiple drawing children"));}
    let text_offset=run.text.len();
    let kind=if named(child,wp,"inline"){
        let picture=crate::drawing_inline::parse_inline_picture_node(child)?;
        let index=run.pictures.len();
        run.pictures.push(InlinePictureAt{text_offset,picture});
        RunDrawingKind::Inline(index)
    }else if named(child,wp,"anchor"){
        RunDrawingKind::Floating(Box::new(crate::drawing_anchor::parse_floating_picture_node(child)?))
    }else{return Err(Error::Unsupported("drawing child"));};
    run.drawing_order.push(RunDrawingAt{text_offset,kind,geometry:geometry(child,a,pic)});
    Ok(())
}
impl Run{
    /// Combined ordered drawing view. Legacy manually constructed runs with
    /// only pictures still work. Mutations must maintain the index, offset and
    /// source order; reject stale/duplicate/missing inline indices explicitly.
    pub fn drawings(&self)->Result<Vec<PictureAtRef<'_>>,Error>{
        let mut out=Vec::new();
        let mut cursor=0;
        let mut next_inline=0;
        if self.drawing_order.is_empty(){
            for p in &self.pictures{
                if p.text_offset<cursor||!self.text.is_char_boundary(p.text_offset){
                    return Err(Error::Invalid("picture text offset"));
                }
                out.push(PictureAtRef{text_offset:p.text_offset,kind:PictureKindRef::Inline(&p.picture),geometry:&ABSENT});
                cursor=p.text_offset;
            }
            return Ok(out);
        }
        for d in &self.drawing_order{
            if d.text_offset<cursor||!self.text.is_char_boundary(d.text_offset){
                return Err(Error::Invalid("drawing text offset"));
            }
            let kind=match &d.kind{
                RunDrawingKind::Inline(i)=>{
                    if *i!=next_inline{return Err(Error::Invalid("drawing inline index order"));}
                    let p=self.pictures.get(*i).ok_or(Error::Invalid("drawing inline index"))?;
                    if p.text_offset!=d.text_offset{return Err(Error::Invalid("drawing inline offset mismatch"));}
                    next_inline+=1;
                    PictureKindRef::Inline(&p.picture)
                }
                RunDrawingKind::Floating(p)=>PictureKindRef::Floating(p),
            };
            out.push(PictureAtRef{text_offset:d.text_offset,kind,geometry:&d.geometry});
            cursor=d.text_offset;
        }
        if next_inline!=self.pictures.len(){return Err(Error::Invalid("unindexed inline picture"));}
        Ok(out)
    }
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct PictureOccurrence<'a>{
    pub source_part:&'a str,pub drawing:PictureAtRef<'a>,
}
impl PictureOccurrence<'_>{
    pub fn bind<'m>(self,media:&'m MediaCatalog)->Result<InlineMediaBindings<'m>,Error>{
        self.drawing.payload().bind(media,self.source_part)
    }
    /// Embedded header dimensions only. Linked-only resources stay inert.
    pub fn embedded_dimensions(self,media:&MediaCatalog,limits:ImageLimits)->Result<Option<ImageDimensions>,Error>{
        match self.bind(media)?.embedded{
            Some(MediaBinding::Embedded{resource,..})=>resource.dimensions(limits).map(Some),
            _=>Ok(None),
        }
    }
    pub fn pixel_window(self,media:&MediaCatalog,limits:ImageLimits)->Result<Option<(u32,u32,u32,u32)>,Error>{
        let g=match self.drawing.geometry{
            PictureGeometryState::Parsed(g)=>g,
            PictureGeometryState::Absent=>return Ok(None),
            PictureGeometryState::Unavailable(_)=>return Err(Error::Unsupported("picture geometry unavailable")),
        };
        match self.embedded_dimensions(media,limits)?{
            Some(d)=>g.crop.pixel_window(d.width,d.height).map(Some),
            None=>Ok(None),
        }
    }
}
#[derive(Debug,PartialEq,Eq)]
pub struct DocumentWithMedia{pub document:Document,pub media:MediaCatalog}
/// Explicit opt-in: read document (including referenced stories) and load the
/// bounded media catalog from the same immutable archive. Two archive passes
/// for now; raw binary storage remains unique per canonical media part.
pub fn read_docx_with_media(bytes:&[u8],limits:Limits,media_limits:MediaLimits)->Result<DocumentWithMedia,Error>{
    let document=crate::read_docx_with_stories(bytes,limits)?;
    let media=crate::load_media(bytes,limits,media_limits)?;
    Ok(DocumentWithMedia{document,media})
}
fn collect<'a>(blocks:&'a [Block],source:&'a str,max:usize,depth:usize,out:&mut Vec<PictureOccurrence<'a>>)->Result<(),Error>{
    if depth>64{return Err(Error::Limit("picture traversal table depth"));}
    for block in blocks{
        match block{
            Block::Paragraph(p)=>for r in &p.runs{
                for drawing in r.drawings()?{
                    if out.len()>=max{return Err(Error::Limit("picture occurrences"));}
                    out.push(PictureOccurrence{source_part:source,drawing});
                }
            },
            Block::Table(t)=>for row in &t.rows{for cell in &row.cells{
                collect(&cell.blocks,source,max,depth+1,out)?;
            }},
            Block::Unsupported{..}=>{},
        }
    }
    Ok(())
}
impl DocumentWithMedia{
    /// Main body (including nested tables) in source order, then loaded story
    /// parts in canonical name order. Occurrences borrow resource-independent
    /// document nodes; binding uses their actual OPC owner, not a global ID.
    pub fn pictures(&self,max_occurrences:usize)->Result<Vec<PictureOccurrence<'_>>,Error>{
        let mut out=Vec::new();
        collect(&self.document.blocks,&self.media.main_part,max_occurrences,0,&mut out)?;
        if let Some(stories)=&self.document.header_footer{
            for (source,story) in &stories.parts{
                collect(&story.blocks,source,max_occurrences,0,&mut out)?;
            }
        }
        Ok(out)
    }
}
