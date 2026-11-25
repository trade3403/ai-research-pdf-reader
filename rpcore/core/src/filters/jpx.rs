//! Bounded JP2 / JPEG 2000 codestream geometry inspection.
//! This is NOT an entropy decoder and does not validate the tile payload.
//! No external runtime dependencies; unsupported structures fail closed.
use crate::{Error,Result,MAX_BYTES};
pub mod codestream;
pub mod geometry;

const SIGNATURE:&[u8]=b"\x00\x00\x00\x0cjP  \x0d\x0a\x87\x0a";
const MAX_BOXES:usize=4096;
const MAX_COMPONENTS:usize=16;
const MAX_TILES:u64=65536;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Container{RawCodestream,Jp2}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Component{
    pub precision:u8,pub signed:bool,pub x_sampling:u8,pub y_sampling:u8,
    pub width:u32,pub height:u32,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Header{
    pub container:Container,pub capabilities:u16,
    pub width:u32,pub height:u32,pub x_origin:u32,pub y_origin:u32,
    pub tile_width:u32,pub tile_height:u32,pub tile_x_origin:u32,pub tile_y_origin:u32,
    pub tiles_x:u32,pub tiles_y:u32,pub components:Vec<Component>,
    pub sample_count:u64,pub decoded_bytes:u64,
    pub codestream_offset:usize,pub codestream_length:usize,
}
fn u16be(b:&[u8],p:usize)->Result<u16>{
    let a=b.get(p..p+2).ok_or(Error::Invalid("JPX truncated integer"))?;
    Ok(u16::from_be_bytes([a[0],a[1]]))
}
fn u32be(b:&[u8],p:usize)->Result<u32>{
    let a=b.get(p..p+4).ok_or(Error::Invalid("JPX truncated integer"))?;
    Ok(u32::from_be_bytes([a[0],a[1],a[2],a[3]]))
}
fn ceil(x:u32,d:u32)->u32{x/d+u32::from(x%d!=0)}
fn raw(b:&[u8],offset:usize,container:Container)->Result<Header>{
    // SOC must be followed by SIZ. Main-header/tile entropy is not inspected.
    if !b.starts_with(&[0xff,0x4f,0xff,0x51]){return Err(Error::Invalid("JPX SOC/SIZ"));}
    let ls=usize::from(u16be(b,4)?);
    if ls<41{return Err(Error::Invalid("JPX SIZ length"));}
    let s=b.get(4..4+ls).ok_or(Error::Invalid("JPX truncated SIZ"))?;
    let count=usize::from(u16be(s,36)?);
    if count==0||ls!=38+3*count{return Err(Error::Invalid("JPX SIZ components"));}
    if count>MAX_COMPONENTS{return Err(Error::Limit("JPX components"));}
    let xs=u32be(s,4)?;let ys=u32be(s,8)?;
    let xo=u32be(s,12)?;let yo=u32be(s,16)?;
    let xt=u32be(s,20)?;let yt=u32be(s,24)?;
    let xto=u32be(s,28)?;let yto=u32be(s,32)?;
    if xs<=xo||ys<=yo||xt==0||yt==0||xto>xo||yto>yo
        ||u64::from(xto)+u64::from(xt)<=u64::from(xo)
        ||u64::from(yto)+u64::from(yt)<=u64::from(yo){
        return Err(Error::Invalid("JPX image/tile grid"));
    }
    let width=xs-xo;let height=ys-yo;
    let tiles_x=ceil(xs-xto,xt);let tiles_y=ceil(ys-yto,yt);
    if u64::from(tiles_x)*u64::from(tiles_y)>MAX_TILES{return Err(Error::Limit("JPX tiles"));}
    if u64::from(width)*u64::from(height)>MAX_BYTES as u64{return Err(Error::Limit("JPX image pixels"));}
    let mut components=Vec::with_capacity(count);let mut samples=0u64;let mut bytes=0u64;
    for i in 0..count{
        let p=38+3*i;let precision=(s[p]&0x7f)+1;
        let signed=s[p]&0x80!=0;let dx=s[p+1];let dy=s[p+2];
        if dx==0||dy==0||precision>38{return Err(Error::Invalid("JPX component geometry"));}
        let w=ceil(xs,u32::from(dx))-ceil(xo,u32::from(dx));
        let h=ceil(ys,u32::from(dy))-ceil(yo,u32::from(dy));
        if w==0||h==0{return Err(Error::Invalid("JPX empty component"));}
        let n=u64::from(w)*u64::from(h);
        samples=samples.checked_add(n).ok_or(Error::Limit("JPX samples"))?;
        bytes=bytes.checked_add(n.checked_mul(u64::from((precision+7)/8)).ok_or(Error::Limit("JPX decoded bytes"))?).ok_or(Error::Limit("JPX decoded bytes"))?;
        if bytes>MAX_BYTES as u64{return Err(Error::Limit("JPX decoded bytes"));}
        components.push(Component{precision,signed,x_sampling:dx,y_sampling:dy,width:w,height:h});
    }
    Ok(Header{container,capabilities:u16be(s,2)?,width,height,x_origin:xo,y_origin:yo,
        tile_width:xt,tile_height:yt,tile_x_origin:xto,tile_y_origin:yto,
        tiles_x,tiles_y,components,sample_count:samples,decoded_bytes:bytes,
        codestream_offset:offset,codestream_length:b.len()})
}
struct BoxView<'a>{kind:[u8;4],payload:&'a [u8],offset:usize}
fn next_box<'a>(data:&'a [u8],pos:&mut usize,base:usize,budget:&mut usize)->Result<BoxView<'a>>{
    *budget=budget.checked_sub(1).ok_or(Error::Limit("JP2 box count"))?;
    let start=*pos;let tail=data.get(start..).ok_or(Error::Invalid("JP2 box offset"))?;
    let len=u32be(tail,0)?;let mut kind=[0;4];
    kind.copy_from_slice(tail.get(4..8).ok_or(Error::Invalid("JP2 box header"))?);
    let (size,head)=match len{
        0=>(tail.len(),8),
        1=>{
            let a:[u8;8]=tail.get(8..16).ok_or(Error::Invalid("JP2 extended box length"))?.try_into().map_err(|_|Error::Invalid("JP2 extended box length"))?;
            (usize::try_from(u64::from_be_bytes(a)).map_err(|_|Error::Limit("JP2 box length"))?,16)
        }
        n=>(n as usize,8),
    };
    if size<head||size>tail.len(){return Err(Error::Invalid("JP2 box length"));}
    *pos=start+size;
    Ok(BoxView{kind,payload:&tail[head..size],offset:base+start+head})
}
struct ImageHeader{width:u32,height:u32,count:usize,bpc:u8,bpcc:Option<Vec<u8>>}
fn image_header(b:&[u8],base:usize,budget:&mut usize)->Result<ImageHeader>{
    let mut pos=0;let mut ih=None;let mut bpcc=None;
    while pos<b.len(){
        let v=next_box(b,&mut pos,base,budget)?;
        match &v.kind{
            b"ihdr"=>{
                if ih.is_some()||v.payload.len()!=14||v.offset!=base+8&&v.offset!=base+16{return Err(Error::Invalid("JP2 image header"));}
                let h=u32be(v.payload,0)?;let w=u32be(v.payload,4)?;let n=usize::from(u16be(v.payload,8)?);
                if h==0||w==0||n==0||v.payload[11]!=7||v.payload[12]>1||v.payload[13]>1{return Err(Error::Invalid("JP2 image header fields"));}
                if n>MAX_COMPONENTS{return Err(Error::Limit("JP2 components"));}
                ih=Some(ImageHeader{width:w,height:h,count:n,bpc:v.payload[10],bpcc:None});
            }
            b"bpcc"=>{
                if bpcc.is_some()||ih.is_none()||v.payload.len()>MAX_COMPONENTS{return Err(Error::Invalid("JP2 component bits"));}
                bpcc=Some(v.payload.to_vec());
            }
            _=>{} // Skip bounded unknown box payloads; do not claim their support.
        }
    }
    let mut ih=ih.ok_or(Error::Invalid("JP2 missing image header"))?;
    if ih.bpc==255{
        if bpcc.as_ref().map(Vec::len)!=Some(ih.count){return Err(Error::Invalid("JP2 missing component bits"));}
    }else if bpcc.is_some(){return Err(Error::Invalid("JP2 unexpected component bits"));}
    ih.bpcc=bpcc;Ok(ih)
}
/// Inspect geometry and component precision of a bounded raw codestream or
/// single-codestream baseline JP2. Not a full conformance or decode result.
/// Color, channel mapping, coding style and compressed tiles are not inspected.
pub fn inspect(data:&[u8])->Result<Header>{
    if data.len()>MAX_BYTES{return Err(Error::Limit("JPX input bytes"));}
    if data.starts_with(&[0xff,0x4f]){return raw(data,0,Container::RawCodestream);}
    if !data.starts_with(SIGNATURE){return Err(Error::Invalid("JP2 signature"));}
    let mut pos=SIGNATURE.len();let mut budget=MAX_BOXES;
    let ft=next_box(data,&mut pos,0,&mut budget)?;
    if &ft.kind!=b"ftyp"||ft.payload.len()<12||ft.payload.len()%4!=0{return Err(Error::Invalid("JP2 file type"));}
    if &ft.payload[..4]!=b"jp2 "||!ft.payload[8..].chunks_exact(4).any(|v|v==b"jp2 "){
        return Err(Error::Unsupported("JP2 file type"));
    }
    let mut ih=None;let mut codestream=None;
    while pos<data.len(){
        let v=next_box(data,&mut pos,0,&mut budget)?;
        match &v.kind{
            b"jp2h"=>{
                if ih.is_some()||codestream.is_some(){return Err(Error::Invalid("JP2 header order"));}
                ih=Some(image_header(v.payload,v.offset,&mut budget)?);
            }
            b"jp2c"=>{
                if codestream.is_some(){return Err(Error::Unsupported("JP2 multiple codestreams"));}
                if ih.is_none(){return Err(Error::Invalid("JP2 missing header before codestream"));}
                codestream=Some(raw(v.payload,v.offset,Container::Jp2)?);
            }
            b"jP  "|b"ftyp"=>return Err(Error::Invalid("JP2 duplicate signature/type")),
            _=>{},
        }
    }
    let ih=ih.ok_or(Error::Invalid("JP2 missing header"))?;
    let h=codestream.ok_or(Error::Invalid("JP2 missing codestream"))?;
    if (ih.width,ih.height,ih.count)!=(h.width,h.height,h.components.len()){
        return Err(Error::Invalid("JP2 header/SIZ mismatch"));
    }
    for (i,c) in h.components.iter().enumerate(){
        let bpc=ih.bpcc.as_ref().map_or(ih.bpc,|v|v[i]);
        if (bpc&0x7f)+1!=c.precision||(bpc&0x80!=0)!=c.signed{return Err(Error::Invalid("JP2 component precision mismatch"));}
    }
    Ok(h)
}

#[cfg(test)]
mod tests{
    use super::*;
    fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap(),16).unwrap()).collect()}
    // Independently produced by Pillow/OpenJPEG 2.5.4, 7x5 gray/RGB.
    const GRAY:&str="0000000c6a5020200d0a870a00000014667479706a703220000000006a7032200000002d6a703268000000166968647200000005000000070001070700000000000f636f6c72010000000000110000008d6a703263ff4fff510029000000000007000000050000000000000000000000070000000500000000000000000001070101ff52000c00000001000204040001ff5c000a4040484850484850ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000150001ff93df801008908080ffd9";
    const RGB:&str="0000000c6a5020200d0a870a00000014667479706a703220000000006a7032200000002d6a703268000000166968647200000005000000070003070700000000000f636f6c7201000000000010000000a16a703263ff4fff51002f000000000007000000050000000000000000000000070000000500000000000000000003070101070101070101ff52000c00000001000204040001ff5c000a4040484850484850ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000230001ff93df80100890df80100890df80100890808080808080ffd9";
    fn siz(xs:u32,ys:u32,xo:u32,yo:u32,xt:u32,yt:u32,xto:u32,yto:u32,c:&[[u8;3]])->Vec<u8>{
        let mut v=vec![0xff,0x4f,0xff,0x51];v.extend(((38+3*c.len())as u16).to_be_bytes());
        v.extend([0,0]);for n in [xs,ys,xo,yo,xt,yt,xto,yto]{v.extend(n.to_be_bytes());}
        v.extend((c.len()as u16).to_be_bytes());for a in c{v.extend(a);}v
    }
    fn box_bytes(kind:&[u8;4],p:&[u8])->Vec<u8>{
        let mut b=((p.len()+8)as u32).to_be_bytes().to_vec();b.extend(kind);b.extend(p);b
    }
    #[test]fn independent_openjpeg_jp2_and_raw(){
        for (fixture,n) in [(GRAY,1),(RGB,3)]{
            let b=hex(fixture);let h=inspect(&b).unwrap();
            assert_eq!((h.width,h.height,h.components.len()),(7,5,n));
            assert_eq!(h.decoded_bytes,35*n as u64);assert_eq!((h.tiles_x,h.tiles_y),(1,1));
            assert_eq!(h.container,Container::Jp2);
            let raw=inspect(&b[h.codestream_offset..h.codestream_offset+h.codestream_length]).unwrap();
            assert_eq!(raw.container,Container::RawCodestream);assert_eq!(raw.components,h.components);
        }
    }
    #[test]fn nonzero_origins_subsampling_signed_precision(){
        let b=siz(21,19,3,5,10,8,0,0,&[[7,1,1],[0x8b,2,3]]);
        let h=inspect(&b).unwrap();assert_eq!((h.width,h.height),(18,14));assert_eq!((h.tiles_x,h.tiles_y),(3,3));
        assert_eq!(h.components[1],Component{precision:12,signed:true,x_sampling:2,y_sampling:3,width:9,height:5});
        assert_eq!((h.sample_count,h.decoded_bytes),(297,342));
    }
    #[test]fn variable_component_precision(){
        let mut b=SIGNATURE.to_vec();b.extend(box_bytes(b"ftyp",b"jp2 \0\0\0\0jp2 "));
        let mut ih=vec![];ih.extend(5u32.to_be_bytes());ih.extend(7u32.to_be_bytes());ih.extend(2u16.to_be_bytes());ih.extend([255,7,0,0]);
        let mut header=box_bytes(b"ihdr",&ih);header.extend(box_bytes(b"bpcc",&[7,0x8b]));
        b.extend(box_bytes(b"jp2h",&header));b.extend(box_bytes(b"jp2c",&siz(7,5,0,0,7,5,0,0,&[[7,1,1],[0x8b,1,1]])));
        let h=inspect(&b).unwrap();assert_eq!(h.decoded_bytes,105);assert_eq!(h.components[1].precision,12);
    }
    #[test]fn extended_and_to_end_boxes(){
        let b=hex(GRAY);let h=inspect(&b).unwrap();let start=h.codestream_offset-8;
        let mut long=b[..start].to_vec();long.extend(1u32.to_be_bytes());long.extend(b"jp2c");long.extend((16+h.codestream_length as u64).to_be_bytes());long.extend(&b[h.codestream_offset..]);
        assert_eq!(inspect(&long).unwrap().codestream_offset,h.codestream_offset+8);
        let mut end=b.clone();end[start..start+4].fill(0);assert_eq!(inspect(&end).unwrap(),h);
    }
    #[test]fn malformed_and_mismatch(){
        let b=hex(RGB);
        for n in [0,4,11,12,20,32,45,b.len()-1]{assert!(inspect(&b[..n]).is_err(),"prefix {n}");}
        let mut x=b.clone();x[55]^=1;assert!(inspect(&x).is_err());
        let h=inspect(&b).unwrap();let mut raw=b[h.codestream_offset..].to_vec();raw[5]=0;
        assert!(inspect(&raw).is_err());
        let mut bad=b.clone();let start=h.codestream_offset-8;bad[start..start+4].copy_from_slice(&7u32.to_be_bytes());assert!(inspect(&bad).is_err());
        let mut duplicate=b.clone();duplicate.extend(&b[start..]);assert!(inspect(&duplicate).is_err());
    }
    #[test]fn invalid_grid_and_resource_limits(){
        for b in [
            siz(0,5,0,0,7,5,0,0,&[[7,1,1]]),
            siz(7,5,0,0,0,5,0,0,&[[7,1,1]]),
            siz(7,5,0,0,7,5,1,0,&[[7,1,1]]),
            siz(7,5,0,0,7,5,0,0,&[[7,0,1]]),
            siz(7,5,0,0,7,5,0,0,&[[38,1,1]]),
            siz(100000,100000,0,0,100000,100000,0,0,&[[7,1,1]]),
            siz(1000,1000,0,0,1,1,0,0,&[[7,1,1]]),
            siz(7,5,0,0,7,5,0,0,&[[7,1,1];17]),
            siz(10000,10000,0,0,10000,10000,0,0,&[[15,1,1]]),
        ]{assert!(inspect(&b).is_err());}
    }
    #[test]fn still_not_a_pixel_decoder(){
        use crate::Object;
        let mut dict=std::collections::BTreeMap::new();dict.insert(b"Filter".to_vec(),Object::Name(b"JPXDecode".to_vec()));
        let obj=Object::Stream{dict,data:hex(GRAY)};
        assert!(matches!(super::super::decode(&obj,1024),Err(Error::Unsupported("stream filter"))));
    }
}
