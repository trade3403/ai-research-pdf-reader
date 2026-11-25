//! C2.4b: bounded raster header metadata, not full image validation.
use crate::{Error,ImageFormat,MediaResource};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ImageLimits {
    pub max_dimension:u32,
    pub max_pixels:u64,
    pub max_header_bytes:usize,
}
impl Default for ImageLimits{
    fn default()->Self{Self{max_dimension:32768,max_pixels:100_000_000,max_header_bytes:1024*1024}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ImageDimensions{pub width:u32,pub height:u32}
impl ImageDimensions{
    pub fn pixels(self)->u64{u64::from(self.width)*u64::from(self.height)}
}
struct Header<'a>{data:&'a [u8],cap:usize}
impl Header<'_>{
    fn get(&self,at:usize,n:usize)->Result<&[u8],Error>{
        let end=at.checked_add(n).ok_or(Error::Limit("image header offset"))?;
        if end>self.cap{return Err(Error::Limit("image header bytes"));}
        self.data.get(at..end).ok_or(Error::Invalid("truncated image header"))
    }
    fn u16(&self,at:usize,le:bool)->Result<u16,Error>{
        let b=self.get(at,2)?.try_into().unwrap();
        Ok(if le{u16::from_le_bytes(b)}else{u16::from_be_bytes(b)})
    }
    fn u32(&self,at:usize,le:bool)->Result<u32,Error>{
        let b=self.get(at,4)?.try_into().unwrap();
        Ok(if le{u32::from_le_bytes(b)}else{u32::from_be_bytes(b)})
    }
    fn u24(&self,at:usize)->Result<u32,Error>{
        let b=self.get(at,3)?;
        Ok(u32::from(b[0])|(u32::from(b[1])<<8)|(u32::from(b[2])<<16))
    }
}
fn png(h:&Header<'_>)->Result<(u32,u32),Error>{
    h.get(0,33)?;
    if h.u32(8,false)?!=13 || h.get(12,4)?!=b"IHDR"{
        return Err(Error::Invalid("PNG IHDR"));
    }
    Ok((h.u32(16,false)?,h.u32(20,false)?))
}
fn jpeg(h:&Header<'_>)->Result<(u32,u32),Error>{
    let mut pos=2usize;
    loop{
        if h.get(pos,1)?[0]!=0xff{return Err(Error::Invalid("JPEG marker prefix"));}
        pos+=1;
        while h.get(pos,1)?[0]==0xff{pos+=1;}
        let marker=h.get(pos,1)?[0];
        pos+=1;
        if matches!(marker,0xda|0xd9){return Err(Error::Invalid("JPEG missing SOF"));}
        if matches!(marker,0|0xd0..=0xd8|1){return Err(Error::Invalid("unexpected JPEG marker"));}
        let len=usize::from(h.u16(pos,false)?);
        if len<2{return Err(Error::Invalid("JPEG segment length"));}
        h.get(pos,len)?;
        if matches!(marker,0xc0..=0xc3|0xc5..=0xc7|0xc9..=0xcb|0xcd..=0xcf){
            if len<8{return Err(Error::Invalid("JPEG SOF length"));}
            let components=usize::from(h.get(pos+7,1)?[0]);
            if components==0||len!=8+3*components{return Err(Error::Invalid("JPEG SOF components"));}
            return Ok((u32::from(h.u16(pos+5,false)?),u32::from(h.u16(pos+3,false)?)));
        }
        pos=pos.checked_add(len).ok_or(Error::Limit("image header offset"))?;
    }
}
fn bmp(h:&Header<'_>)->Result<(u32,u32),Error>{
    let dib=h.u32(14,true)?;
    if dib==12{
        h.get(14,12)?;
        if h.u16(22,true)?!=1{return Err(Error::Invalid("BMP planes"));}
        return Ok((u32::from(h.u16(18,true)?),u32::from(h.u16(20,true)?)));
    }
    if !matches!(dib,40|52|56|108|124){return Err(Error::Unsupported("BMP DIB header"));}
    h.get(14,dib as usize)?;
    let w=h.u32(18,true)? as i32;
    let height=h.u32(22,true)? as i32;
    if w<=0||height==i32::MIN||h.u16(26,true)?!=1{
        return Err(Error::Invalid("BMP dimensions or planes"));
    }
    Ok((w as u32,height.unsigned_abs()))
}
fn tiff(h:&Header<'_>)->Result<(u32,u32),Error>{
    let le=h.get(0,2)?==b"II";
    let offset=usize::try_from(h.u32(4,le)?).map_err(|_|Error::Limit("image header offset"))?;
    if offset<8||offset%2!=0{return Err(Error::Invalid("TIFF IFD offset"));}
    let count=usize::from(h.u16(offset,le)?);
    if count>4096{return Err(Error::Limit("TIFF IFD entries"));}
    let entries=h.get(offset,2+count*12+4)?;
    let mut width=None;
    let mut height=None;
    for i in 0..count{
        let p=offset+2+i*12;
        let tag=h.u16(p,le)?;
        let slot=match tag{256=>&mut width,257=>&mut height,_=>continue};
        if slot.is_some(){return Err(Error::Invalid("duplicate TIFF dimension"));}
        if h.u32(p+4,le)?!=1{return Err(Error::Unsupported("TIFF dimension count"));}
        *slot=Some(match h.u16(p+2,le)?{
            3=>u32::from(h.u16(p+8,le)?),
            4=>h.u32(p+8,le)?,
            _=>return Err(Error::Unsupported("TIFF dimension type")),
        });
    }
    let _=entries;
    Ok((width.ok_or(Error::Invalid("missing TIFF width"))?,height.ok_or(Error::Invalid("missing TIFF height"))?))
}
fn webp(h:&Header<'_>)->Result<(u32,u32),Error>{
    let riff=u64::from(h.u32(4,true)?)+8;
    if riff!=h.data.len() as u64 || riff<20{return Err(Error::Invalid("WebP RIFF size"));}
    let tag=h.get(12,4)?;
    let n=u64::from(h.u32(16,true)?);
    if 20+n+(n%2)>riff{return Err(Error::Invalid("WebP chunk size"));}
    match tag{
        b"VP8X"=>{
            if n!=10{return Err(Error::Invalid("WebP VP8X length"));}
            h.get(20,10)?;
            Ok((h.u24(24)?+1,h.u24(27)?+1))
        }
        b"VP8L"=>{
            if n<5||h.get(20,1)?[0]!=0x2f{return Err(Error::Invalid("WebP VP8L header"));}
            let bits=h.u32(21,true)?;
            if bits>>29!=0{return Err(Error::Unsupported("WebP VP8L version"));}
            Ok(((bits&0x3fff)+1,((bits>>14)&0x3fff)+1))
        }
        b"VP8 "=>{
            if n<10||h.get(20,1)?[0]&1!=0||h.get(23,3)?!=b"\x9d\x01\x2a"{
                return Err(Error::Invalid("WebP VP8 keyframe header"));
            }
            Ok((u32::from(h.u16(26,true)?&0x3fff),u32::from(h.u16(28,true)?&0x3fff)))
        }
        _=>Err(Error::Unsupported("WebP first chunk")),
    }
}
/// Reads PNG IHDR, JPEG SOF, GIF logical screen, BMP DIB, classic TIFF first
/// IFD, or WebP first image/canvas chunk. Dimensions are stored raster sizes,
/// not EXIF-oriented sizes. No CRC, complete image structure, frame sequence,
/// compressed payload, color profile, or pixel decoding is validated.
pub fn parse_image_dimensions(format:ImageFormat,bytes:&[u8],limits:ImageLimits)
    ->Result<ImageDimensions,Error>
{
    if !format.matches_signature(bytes){return Err(Error::Invalid("image signature mismatch"));}
    let h=Header{data:bytes,cap:limits.max_header_bytes};
    let (width,height)=match format{
        ImageFormat::Png=>png(&h)?,
        ImageFormat::Jpeg=>jpeg(&h)?,
        ImageFormat::Gif=>{h.get(0,13)?;(u32::from(h.u16(6,true)?),u32::from(h.u16(8,true)?))},
        ImageFormat::Bmp=>bmp(&h)?,
        ImageFormat::Tiff=>tiff(&h)?,
        ImageFormat::WebP=>webp(&h)?,
    };
    let d=ImageDimensions{width,height};
    if width==0||height==0{return Err(Error::Invalid("zero image dimension"));}
    if width>limits.max_dimension||height>limits.max_dimension{return Err(Error::Limit("image dimension"));}
    if d.pixels()>limits.max_pixels{return Err(Error::Limit("image pixels"));}
    Ok(d)
}
impl MediaResource{
    pub fn dimensions(&self,limits:ImageLimits)->Result<ImageDimensions,Error>{
        parse_image_dimensions(self.format,&self.bytes,limits)
    }
}
