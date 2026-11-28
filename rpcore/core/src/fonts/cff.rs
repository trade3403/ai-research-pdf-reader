//! CFF1 outline reader. No PostScript execution, no CFF2 or hinting.
use crate::{graphics::{Path,Segment},text::{Matrix,Point},Error,Result};
use std::collections::BTreeMap;
type Range=(usize,usize);
type Dict=BTreeMap<u16,Vec<f64>>;
const MAX_BYTES:usize=16*1024*1024;
const MAX_OPS:usize=100_000;
const MAX_STEMS:usize=96;
const MAX_DEPTH:usize=10;
const MAX_STACK:usize=48;
const STANDARD:&str=".notdef space exclam quotedbl numbersign dollar percent ampersand quoteright parenleft parenright asterisk plus comma hyphen period slash zero one two three four five six seven eight nine colon semicolon less equal greater question at A B C D E F G H I J K L M N O P Q R S T U V W X Y Z bracketleft backslash bracketright asciicircum underscore quoteleft a b c d e f g h i j k l m n o p q r s t u v w x y z braceleft bar braceright asciitilde exclamdown cent sterling fraction yen florin section currency quotesingle quotedblleft guillemotleft guilsinglleft guilsinglright fi fl endash dagger daggerdbl periodcentered paragraph bullet quotesinglbase quotedblbase quotedblright guillemotright ellipsis perthousand questiondown grave acute circumflex tilde macron breve dotaccent dieresis ring cedilla hungarumlaut ogonek caron emdash AE ordfeminine Lslash Oslash OE ordmasculine ae dotlessi lslash oslash oe germandbls onesuperior logicalnot mu trademark Eth onehalf plusminus Thorn onequarter divide brokenbar degree thorn threequarters twosuperior registered minus eth multiply threesuperior copyright Aacute Acircumflex Adieresis Agrave Aring Atilde Ccedilla Eacute Ecircumflex Edieresis Egrave Iacute Icircumflex Idieresis Igrave Ntilde Oacute Ocircumflex Odieresis Ograve Otilde Scaron Uacute Ucircumflex Udieresis Ugrave Yacute Ydieresis Zcaron aacute acircumflex adieresis agrave aring atilde ccedilla eacute ecircumflex edieresis egrave iacute icircumflex idieresis igrave ntilde oacute ocircumflex odieresis ograve otilde scaron uacute ucircumflex udieresis ugrave yacute ydieresis zcaron exclamsmall Hungarumlautsmall dollaroldstyle dollarsuperior ampersandsmall Acutesmall parenleftsuperior parenrightsuperior twodotenleader onedotenleader zerooldstyle oneoldstyle twooldstyle threeoldstyle fouroldstyle fiveoldstyle sixoldstyle sevenoldstyle eightoldstyle nineoldstyle commasuperior threequartersemdash periodsuperior questionsmall asuperior bsuperior centsuperior dsuperior esuperior isuperior lsuperior msuperior nsuperior osuperior rsuperior ssuperior tsuperior ff ffi ffl parenleftinferior parenrightinferior Circumflexsmall hyphensuperior Gravesmall Asmall Bsmall Csmall Dsmall Esmall Fsmall Gsmall Hsmall Ismall Jsmall Ksmall Lsmall Msmall Nsmall Osmall Psmall Qsmall Rsmall Ssmall Tsmall Usmall Vsmall Wsmall Xsmall Ysmall Zsmall colonmonetary onefitted rupiah Tildesmall exclamdownsmall centoldstyle Lslashsmall Scaronsmall Zcaronsmall Dieresissmall Brevesmall Caronsmall Dotaccentsmall Macronsmall figuredash hypheninferior Ogoneksmall Ringsmall Cedillasmall questiondownsmall oneeighth threeeighths fiveeighths seveneighths onethird twothirds zerosuperior foursuperior fivesuperior sixsuperior sevensuperior eightsuperior ninesuperior zeroinferior oneinferior twoinferior threeinferior fourinferior fiveinferior sixinferior seveninferior eightinferior nineinferior centinferior dollarinferior periodinferior commainferior Agravesmall Aacutesmall Acircumflexsmall Atildesmall Adieresissmall Aringsmall AEsmall Ccedillasmall Egravesmall Eacutesmall Ecircumflexsmall Edieresissmall Igravesmall Iacutesmall Icircumflexsmall Idieresissmall Ethsmall Ntildesmall Ogravesmall Oacutesmall Ocircumflexsmall Otildesmall Odieresissmall OEsmall Oslashsmall Ugravesmall Uacutesmall Ucircumflexsmall Udieresissmall Yacutesmall Thornsmall Ydieresissmall 001.000 001.001 001.002 001.003 Black Bold Book Light Medium Regular Roman Semibold";
// Standardized Adobe CFF charset and encoding SID tables (CFF specification, Technical Note 5176).
// PDF WinAnsi, MacRoman and MacExpert glyph-name tables use the same SID vocabulary.
// SID 0 is undefined; WinAnsi code 128 (Euro) is handled by name, not a fixed SID.
const EXPERT_CHARSET:[u16;166]=[
    0,1,229,230,231,232,233,234,235,236,237,238,13,14,15,99,
    239,240,241,242,243,244,245,246,247,248,27,28,249,250,251,252,
    253,254,255,256,257,258,259,260,261,262,263,264,265,266,109,110,
    267,268,269,270,271,272,273,274,275,276,277,278,279,280,281,282,
    283,284,285,286,287,288,289,290,291,292,293,294,295,296,297,298,
    299,300,301,302,303,304,305,306,307,308,309,310,311,312,313,314,
    315,316,317,318,158,155,163,319,320,321,322,323,324,325,326,150,
    164,169,327,328,329,330,331,332,333,334,335,336,337,338,339,340,
    341,342,343,344,345,346,347,348,349,350,351,352,353,354,355,356,
    357,358,359,360,361,362,363,364,365,366,367,368,369,370,371,372,
    373,374,375,376,377,378,
];
const EXPERT_SUBSET:[u16;87]=[
    0,1,231,232,235,236,237,238,13,14,15,99,239,240,241,242,
    243,244,245,246,247,248,27,28,249,250,251,253,254,255,256,257,
    258,259,260,261,262,263,264,265,266,109,110,267,268,269,270,272,
    300,301,302,305,314,315,158,155,163,320,321,322,323,324,325,326,
    150,164,169,327,328,329,330,331,332,333,334,335,336,337,338,339,
    340,341,342,343,344,345,346,
];
const EXPERT_ENC:[u16;256]=[
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    1,229,230,0,231,232,233,234,235,236,237,238,13,14,15,99,
    239,240,241,242,243,244,245,246,247,248,27,28,249,250,251,252,
    0,253,254,255,256,257,0,0,0,258,0,0,259,260,261,262,
    0,0,263,264,265,0,266,109,110,267,268,269,0,270,271,272,
    273,274,275,276,277,278,279,280,281,282,283,284,285,286,287,288,
    289,290,291,292,293,294,295,296,297,298,299,300,301,302,303,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,304,305,306,0,0,307,308,309,310,311,0,312,0,0,313,
    0,0,314,315,0,0,316,317,318,0,0,0,158,155,163,319,
    320,321,322,323,324,325,0,0,326,150,164,169,327,328,329,330,
    331,332,333,334,335,336,337,338,339,340,341,342,343,344,345,346,
    347,348,349,350,351,352,353,354,355,356,357,358,359,360,361,362,
    363,364,365,366,367,368,369,370,371,372,373,374,375,376,377,378,
];
const WIN_ANSI:[u16;256]=[
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    1,2,3,4,5,6,7,104,9,10,11,12,13,14,15,16,
    17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,
    33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,
    49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,64,
    124,66,67,68,69,70,71,72,73,74,75,76,77,78,79,80,
    81,82,83,84,85,86,87,88,89,90,91,92,93,94,95,116,
    0,116,117,101,118,121,112,113,126,122,192,107,142,116,199,116,
    116,65,8,105,119,116,111,137,127,153,221,108,148,116,228,198,
    1,96,97,98,103,100,160,102,131,170,139,106,151,14,165,128,
    161,156,164,169,125,152,115,114,133,150,143,120,158,155,163,123,
    174,171,172,176,173,175,138,177,181,178,179,180,185,182,183,184,
    154,186,190,187,188,191,189,168,141,196,193,194,195,197,157,149,
    203,200,201,205,202,204,144,206,210,207,208,209,214,211,212,213,
    167,215,219,216,217,220,218,159,147,225,222,223,224,226,162,227,
];
const MAC_ROMAN:[u16;256]=[
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    1,2,3,4,5,6,7,104,9,10,11,12,13,14,15,16,
    17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,
    33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,
    49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,64,
    124,66,67,68,69,70,71,72,73,74,75,76,77,78,79,80,
    81,82,83,84,85,86,87,88,89,90,91,92,93,94,95,0,
    173,175,177,178,186,189,195,200,203,201,202,205,204,206,207,210,
    208,209,211,214,212,213,215,216,219,217,218,220,222,225,223,224,
    112,161,97,98,102,116,115,149,165,170,153,125,131,0,138,141,
    0,156,0,0,100,152,0,0,0,0,0,139,143,0,144,147,
    123,96,151,0,101,0,0,106,120,121,1,174,176,191,142,148,
    111,137,105,119,65,8,159,0,227,198,99,103,107,108,109,110,
    113,114,117,118,122,172,179,171,180,181,182,183,184,185,187,188,
    0,190,193,194,196,145,126,127,128,129,130,132,133,134,135,136,
];
const MAC_EXPERT:[u16;256]=[
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    1,229,230,305,231,232,233,234,235,236,237,238,13,14,15,99,
    239,240,241,242,243,244,245,246,247,248,27,28,0,250,0,252,
    0,0,0,0,363,0,0,158,155,163,320,321,322,323,324,325,
    0,0,0,0,0,0,266,109,110,267,268,269,0,270,271,315,
    273,274,275,276,277,278,279,280,281,282,283,284,285,286,287,288,
    289,290,291,292,293,294,295,296,297,298,299,300,301,302,303,0,
    0,253,255,0,0,0,0,348,347,349,351,350,352,354,356,355,
    357,358,360,359,361,362,364,366,365,367,369,368,373,372,374,375,
    0,331,337,336,339,341,340,307,0,343,335,0,309,0,311,262,
    338,0,346,345,376,0,344,0,0,377,0,342,333,308,353,371,
    319,334,306,0,0,0,0,0,0,318,0,0,0,0,0,370,
    314,272,0,0,0,0,304,0,378,0,150,164,169,327,328,329,
    330,332,326,0,257,263,265,0,0,258,264,256,0,0,0,0,
    0,259,316,310,313,254,261,260,249,251,312,317,0,0,0,0,
];
struct Read<'a>{d:&'a[u8],p:usize}
impl Read<'_>{
    fn take(&mut self,n:usize)->Result<&[u8]>{let end=self.p.checked_add(n).ok_or(Error::Limit("CFF offset"))?;let a=self.d.get(self.p..end).ok_or(Error::Invalid("truncated CFF"))?;self.p=end;Ok(a)}
    fn byte(&mut self)->Result<u8>{Ok(self.take(1)?[0])}
    fn uint(&mut self,n:usize)->Result<usize>{if !(1..=4).contains(&n){return Err(Error::Invalid("CFF offSize"));}Ok(self.take(n)?.iter().fold(0usize,|a,&b|(a<<8)|usize::from(b)))}
    fn i16(&mut self)->Result<i16>{Ok(self.uint(2)? as i16)}
    fn index(&mut self)->Result<Vec<Range>>{
        let n=self.uint(2)?;if n==0{return Ok(Vec::new());}
        let width=self.byte()? as usize;let mut offsets=Vec::with_capacity(n+1);
        for _ in 0..=n{offsets.push(self.uint(width)?);}
        if offsets[0]!=1||offsets.windows(2).any(|w|w[0]>w[1]){return Err(Error::Invalid("CFF INDEX offsets"));}
        let start=self.p;self.take(offsets[n]-1)?;
        Ok(offsets.windows(2).map(|w|(start+w[0]-1,start+w[1]-1)).collect())
    }
}
fn number(b:u8,r:&mut Read<'_>,dict:bool)->Result<f64>{
    Ok(match b{
        32..=246=>f64::from(b)-139.,
        247..=250=>(f64::from(b)-247.)*256.+f64::from(r.byte()?)+108.,
        251..=254=>-(f64::from(b)-251.)*256.-f64::from(r.byte()?)-108.,
        28=>f64::from(r.i16()?),
        29 if dict=>f64::from(r.uint(4)? as i32),
        255 if !dict=>f64::from(r.uint(4)? as i32)/65536.,
        30 if dict=>{
            let mut s=String::new();'outer:loop{
                let b=r.byte()?;for n in [b>>4,b&15]{match n{
                    0..=9=>s.push(char::from(b'0'+n)),10=>s.push('.'),11=>s.push('E'),12=>s.push_str("E-"),14=>s.push('-'),15=>break 'outer,_=>return Err(Error::Invalid("CFF real")),
                }if s.len()>64{return Err(Error::Limit("CFF real"));}}
            }
            s.parse::<f64>().ok().filter(|v|v.is_finite()).ok_or(Error::Invalid("CFF real"))?
        }
        _=>return Err(Error::Invalid("CFF number")),
    })
}
fn dict(data:&[u8])->Result<Dict>{
    let mut r=Read{d:data,p:0};let mut d=Dict::new();let mut stack=Vec::new();
    while r.p<data.len(){
        let b=r.byte()?;
        if b>=32||matches!(b,28..=30){
            if stack.len()>=MAX_STACK{return Err(Error::Limit("CFF DICT stack"));}
            stack.push(number(b,&mut r,true)?);
        }else{
            let op=if b==12{0x0c00|u16::from(r.byte()?)}else{u16::from(b)};
            if d.insert(op,std::mem::take(&mut stack)).is_some(){return Err(Error::Invalid("duplicate CFF DICT operator"));}
        }
    }
    if !stack.is_empty(){return Err(Error::Invalid("CFF DICT trailing operands"));}Ok(d)
}
fn int(v:f64,max:usize)->Result<usize>{if v>=0.&&v<=max as f64&&v.fract()==0.{Ok(v as usize)}else{Err(Error::Invalid("CFF integer"))}}
fn offset(d:&Dict,op:u16)->Result<usize>{let a=d.get(&op).ok_or(Error::Invalid("missing CFF offset"))?;if a.len()!=1{return Err(Error::Invalid("CFF offset operands"));}int(a[0],MAX_BYTES)}
fn matrix(d:&Dict,default:Matrix)->Result<Matrix>{
    if let Some(a)=d.get(&0x0c07){if a.len()!=6||a.iter().any(|x|!x.is_finite()||x.abs()>1e6){return Err(Error::Invalid("CFF FontMatrix"));}Ok(Matrix(a.as_slice().try_into().unwrap()))}else{Ok(default)}
}
#[derive(Debug,Default)]
struct Private{local:Vec<Range>,default_width:f64,nominal_width:f64}
fn private(data:&[u8],d:&Dict)->Result<Private>{
    let Some(a)=d.get(&18) else{return Ok(Private::default());};if a.len()!=2{return Err(Error::Invalid("CFF Private DICT"));}
    let (size,start)=(int(a[0],MAX_BYTES)?,int(a[1],MAX_BYTES)?);
    let p=dict(data.get(start..start.checked_add(size).ok_or(Error::Limit("CFF private offset"))?).ok_or(Error::Invalid("CFF private range"))?)?;
    let local=if p.contains_key(&19){let mut r=Read{d:data,p:start.checked_add(offset(&p,19)?).ok_or(Error::Limit("CFF Subrs"))?};r.index()?}else{Vec::new()};
    let value=|op|->Result<f64>{match p.get(&op){None=>Ok(0.),Some(a) if a.len()==1&&a[0].abs()<=1e7=>Ok(a[0]),_=>Err(Error::Invalid("CFF width"))}};
    Ok(Private{local,default_width:value(20)?,nominal_width:value(21)?})
}
/// Names from standardized encoding tables. Undefined entries stay unmapped.
pub fn predefined_name(encoding:&[u8],code:u8)->Option<&'static[u8]>{
    if encoding==b"StandardEncoding"{return super::type1::standard_name(code);}
    if encoding==b"WinAnsiEncoding"&&code==128{return Some(b"Euro");}
    let table=match encoding{
        b"ExpertEncoding"=>&EXPERT_ENC,b"WinAnsiEncoding"=>&WIN_ANSI,
        b"MacRomanEncoding"=>&MAC_ROMAN,b"MacExpertEncoding"=>&MAC_EXPERT,_=>return None,
    };
    let sid=table[code as usize];if sid==0{return None;}
    STANDARD.split_ascii_whitespace().nth(sid as usize).map(str::as_bytes)
}
fn read_encoding(data:&[u8],offset:usize,keys:&BTreeMap<u16,u16>,names:&BTreeMap<Vec<u8>,u16>,n:usize)->Result<[Option<u16>;256]>{
    let mut map=[None;256];
    if offset<=1{
        let name=if offset==0{b"StandardEncoding".as_slice()}else{b"ExpertEncoding".as_slice()};
        for code in 0..=255u8{map[code as usize]=predefined_name(name,code).and_then(|n|names.get(n).copied());}
        return Ok(map);
    }
    let mut r=Read{d:data,p:offset};
    let fmt=r.byte()?;let mut gid=1usize;
    let mut put=|code:usize,gid:usize|->Result<()>{
        if code>255||gid>=n||map[code].is_some(){return Err(Error::Invalid("CFF Encoding range or duplicate code"));}
        map[code]=Some(gid as u16);Ok(())
    };
    match fmt&127{
        0=>{let count=r.byte()? as usize;for _ in 0..count{put(r.byte()? as usize,gid)?;gid+=1;}}
        1=>{let count=r.byte()? as usize;for _ in 0..count{
            let first=r.byte()? as usize;let left=r.byte()? as usize;
            if first+left>255{return Err(Error::Invalid("CFF Encoding range"));}
            for code in first..=first+left{put(code,gid)?;gid+=1;}
        }}
        _=>return Err(Error::Invalid("CFF Encoding format")),
    }
    if fmt&128!=0{
        let count=r.byte()? as usize;
        for _ in 0..count{
            let code=r.byte()? as usize;let sid=r.uint(2)? as u16;
            let gid=*keys.get(&sid).ok_or(Error::Invalid("CFF Encoding supplement SID"))?;
            if gid==0||map[code].is_some(){return Err(Error::Invalid("CFF Encoding supplement code"));}
            map[code]=Some(gid);
        }
    }
    Ok(map)
}
#[derive(Debug)]
pub struct Cff{data:Vec<u8>,chars:Vec<Range>,global:Vec<Range>,privates:Vec<Private>,fd:Vec<usize>,keys:BTreeMap<u16,u16>,names:BTreeMap<Vec<u8>,u16>,matrix:Matrix,cid:bool,encoding:[Option<u16>;256]}
/// Extract CFF1 from an OpenType OTTO table directory. No font platform APIs.
pub fn opentype_cff(data:&[u8])->Result<&[u8]>{
    if data.len()>MAX_BYTES{return Err(Error::Limit("OpenType font bytes"));}
    if data.get(..4)!=Some(b"OTTO"){return Err(Error::Unsupported("OpenType CFF container"));}
    let u16at=|p:usize|->Result<usize>{Ok(u16::from_be_bytes(data.get(p..p+2).ok_or(Error::Invalid("OpenType directory"))?.try_into().unwrap()) as usize)};
    let u32at=|p:usize|->Result<usize>{Ok(u32::from_be_bytes(data.get(p..p+4).ok_or(Error::Invalid("OpenType directory"))?.try_into().unwrap()) as usize)};
    let n=u16at(4)?;if n==0||n>128{return Err(Error::Limit("OpenType table count"));}
    let end=12+n*16;if end>data.len(){return Err(Error::Invalid("OpenType directory"));}
    let mut result=None;let mut ranges=Vec::new();let mut tags=std::collections::BTreeSet::new();
    for i in 0..n{
        let p=12+16*i;let tag=&data[p..p+4];
        if !tags.insert(tag){return Err(Error::Invalid("duplicate OpenType table"));}
        let offset=u32at(p+8)?;let len=u32at(p+12)?;
        let last=offset.checked_add(len).ok_or(Error::Invalid("OpenType table range"))?;
        if offset<end||last>data.len()||offset%4!=0{return Err(Error::Invalid("OpenType table range"));}
        if len>0{ranges.push((offset,last));}
        if tag==b"CFF "{if len<4{return Err(Error::Invalid("OpenType CFF table"));}result=Some(&data[offset..last]);}
    }
    ranges.sort_unstable();
    if ranges.windows(2).any(|w|w[0].1>w[1].0){return Err(Error::Invalid("overlapping OpenType tables"));}
    result.ok_or(Error::Unsupported("OpenType without CFF1"))
}
impl Cff{
    pub fn parse_opentype(data:&[u8])->Result<Self>{Self::parse(opentype_cff(data)?.to_vec())}
    pub fn parse(data:Vec<u8>)->Result<Self>{
        if data.len()>MAX_BYTES{return Err(Error::Limit("CFF bytes"));}
        let mut r=Read{d:&data,p:0};if r.byte()?!=1{return Err(Error::Unsupported("CFF version"));}
        r.byte()?;let header=r.byte()? as usize;let offsize=r.byte()?;
        if header<4||header>data.len()||!(1..=4).contains(&offsize){return Err(Error::Invalid("CFF header"));}r.p=header;
        let names_index=r.index()?;let top=r.index()?;let strings=r.index()?;let global=r.index()?;
        if names_index.len()!=1||top.len()!=1{return Err(Error::Unsupported("CFF font set"));}
        let d=dict(&data[top[0].0..top[0].1])?;
        if d.get(&0x0c06).is_some_and(|v|v.as_slice()!=[2.]){return Err(Error::Unsupported("CFF CharStringType"));}
        let cid=d.contains_key(&0x0c1e);
        let m=matrix(&d,Matrix([0.001,0.,0.,0.001,0.,0.]))?;
        r.p=offset(&d,17)?;let chars=r.index()?;if chars.is_empty(){return Err(Error::Invalid("CFF CharStrings"));}
        let n=chars.len();let charset=if d.contains_key(&15){offset(&d,15)?}else{0};
        let mut values=vec![0u16];
        if charset==0&&!cid{
            if n>229{return Err(Error::Invalid("ISOAdobe charset size"));}
            values.extend((1..n).map(|i|i as u16));
        }else if !cid&&matches!(charset,1|2){
            let predefined=if charset==1{EXPERT_CHARSET.as_slice()}else{EXPERT_SUBSET.as_slice()};
            if n>predefined.len(){return Err(Error::Invalid("CFF predefined charset size"));}
            values=predefined[..n].to_vec();
        }else{
            if charset<=2{return Err(Error::Invalid("CID predefined charset"));}
            r.p=charset;let format=r.byte()?;
            while values.len()<n{
                let first=r.uint(2)?;let more=match format{0=>0,1=>r.uint(1)?,2=>r.uint(2)?,_=>return Err(Error::Invalid("CFF charset format"))};
                if more>=n-values.len()||first+more>65535{return Err(Error::Invalid("CFF charset range"));}
                values.extend((first..=first+more).map(|x|x as u16));
            }
        }
        let mut keys=BTreeMap::new();let mut names=BTreeMap::new();
        let standard:Vec<_>=STANDARD.split_ascii_whitespace().collect();
        for (gid,&key) in values.iter().enumerate(){
            if keys.insert(key,gid as u16).is_some(){return Err(Error::Invalid("duplicate CFF charset"));}
            if !cid{
                let name=if key<391{standard[usize::from(key)].as_bytes()}else{let &(a,b)=strings.get(usize::from(key)-391).ok_or(Error::Invalid("CFF SID"))?;&data[a..b]};
                if name.len()>255||names.insert(name.to_vec(),gid as u16).is_some(){return Err(Error::Invalid("CFF glyph name"));}
            }
        }
        let (privates,fd)=if cid{
            r.p=offset(&d,0x0c24)?;let idx=r.index()?;if idx.is_empty()||idx.len()>256{return Err(Error::Limit("CFF FDArray"));}
            let mut p=Vec::new();
            for (a,b) in idx{let fd=dict(&data[a..b])?;if matrix(&fd,Matrix::IDENTITY)?!=Matrix::IDENTITY{return Err(Error::Unsupported("CFF per-FD FontMatrix"));}p.push(private(&data,&fd)?);}
            r.p=offset(&d,0x0c25)?;let mut f=Vec::with_capacity(n);
            match r.byte()?{
                0=>{for _ in 0..n{f.push(r.byte()? as usize);}}
                3=>{let count=r.uint(2)?;if count==0||count>n{return Err(Error::Invalid("FDSelect range count"));}
                    let mut ranges=Vec::with_capacity(count);
                    for _ in 0..count{ranges.push((r.uint(2)?,r.byte()? as usize));}
                    let end=r.uint(2)?;if ranges[0].0!=0||end!=n{return Err(Error::Invalid("FDSelect sentinel"));}
                    for i in 0..count{let (a,k)=ranges[i];let b=ranges.get(i+1).map_or(end,|x|x.0);if a>=b||b>n{return Err(Error::Invalid("FDSelect ranges"));}f.resize(b,k);}
                }
                _=>return Err(Error::Unsupported("CFF FDSelect format")),
            }
            if f.iter().any(|&i|i>=p.len()){return Err(Error::Invalid("FDSelect index"));}(p,f)
        }else{(vec![private(&data,&d)?],vec![0;n])};
        let encoding=if cid{[None;256]}else{
            let enc=if d.contains_key(&16){offset(&d,16)?}else{0};
            read_encoding(&data,enc,&keys,&names,n)?
        };
        Ok(Self{data,chars,global,privates,fd,keys,names,matrix:m,cid,encoding})
    }
    pub fn is_cid(&self)->bool{self.cid}
    pub fn glyph_count(&self)->usize{self.chars.len()}
    pub fn font_matrix(&self)->Matrix{self.matrix}
    pub fn glyph_for_cid(&self,cid:u32)->Option<u16>{if self.cid{self.keys.get(&u16::try_from(cid).ok()?).copied()}else{None}}
    /// Names of non-CID glyphs paired with their actual glyph IDs.
    pub(crate) fn named_glyphs(&self)->impl Iterator<Item=(u16,&[u8])>+'_{
        self.names.iter().map(|(name,&gid)|(gid,name.as_slice()))
    }
    pub fn glyph_for_name(&self,name:&[u8])->Option<u16>{self.names.get(name).copied()}
    /// Resolve a simple font's built-in code; CID fonts have no byte encoding.
    pub fn glyph_for_code(&self,code:u8)->Option<u16>{self.encoding[code as usize]}
    /// Resolve a PDF predefined encoding by glyph name, not by Unicode or GID.
    pub fn glyph_for_encoding(&self,encoding:&[u8],code:u8)->Option<u16>{
        predefined_name(encoding,code).and_then(|name|self.glyph_for_name(name))
    }
    pub fn outline(&self,gid:u16)->Result<Path>{
        let i=gid as usize;let &(a,b)=self.chars.get(i).ok_or(Error::Invalid("CFF glyph ID"))?;
        let p=&self.privates[self.fd[i]];
        let mut vm=Vm{font:self,private:p,stack:Vec::new(),path:Vec::new(),point:[0.,0.],open:false,stems:0,width_seen:false,width:p.default_width,ops:0};
        if vm.run(&self.data[a..b],0)?!=Flow::End{return Err(Error::Invalid("CFF missing endchar"));}
        Ok(vm.path)
    }
}
#[derive(PartialEq,Eq)]
enum Flow{Return,End}
struct Vm<'a>{font:&'a Cff,private:&'a Private,stack:Vec<f64>,path:Path,point:Point,open:bool,stems:usize,width_seen:bool,width:f64,ops:usize}
impl Vm<'_>{
    fn push(&mut self,v:f64)->Result<()>{if self.stack.len()>=MAX_STACK{return Err(Error::Limit("Type2 stack"));}if !v.is_finite()||v.abs()>1e9{return Err(Error::Limit("Type2 number"));}self.stack.push(v);Ok(())}
    fn close(&mut self){if self.open{self.path.push(Segment::Close);self.open=false;}}
    fn relative(&mut self,x:f64,y:f64)->Result<Point>{let p=[self.point[0]+x,self.point[1]+y];if p.iter().any(|v|!v.is_finite()||v.abs()>1e9){return Err(Error::Limit("Type2 coordinate"));}self.point=p;Ok(p)}
    fn mov(&mut self,x:f64,y:f64)->Result<()>{self.close();let p=self.relative(x,y)?;self.path.push(Segment::Move(p));self.open=true;Ok(())}
    fn line(&mut self,x:f64,y:f64)->Result<()>{if !self.open{return Err(Error::Invalid("Type2 path without move"));}let p=self.relative(x,y)?;self.path.push(Segment::Line(p));Ok(())}
    fn curve(&mut self,a:&[f64])->Result<()>{if !self.open||a.len()!=6{return Err(Error::Invalid("Type2 curve"));}let p=self.relative(a[0],a[1])?;let q=self.relative(a[2],a[3])?;let r=self.relative(a[4],a[5])?;self.path.push(Segment::Cubic(p,q,r));Ok(())}
    fn width(&mut self,extra:bool)->Result<()>{
        if !self.width_seen{self.width_seen=true;if extra{if self.stack.is_empty(){return Err(Error::Invalid("Type2 width"));}self.width=self.private.nominal_width+self.stack.remove(0);}}
        else if extra{return Err(Error::Invalid("unexpected Type2 width"));}
        Ok(())
    }
    fn hint(&mut self)->Result<()>{
        self.width(self.stack.len()%2!=0)?;
        if self.stack.len()%2!=0{return Err(Error::Invalid("Type2 stem operands"));}
        self.stems+=self.stack.len()/2;if self.stems>MAX_STEMS{return Err(Error::Limit("Type2 stems"));}self.stack.clear();Ok(())
    }
    fn run(&mut self,data:&[u8],depth:usize)->Result<Flow>{
        if depth>MAX_DEPTH{return Err(Error::Limit("Type2 subroutine depth"));}
        let mut r=Read{d:data,p:0};
        while r.p<data.len(){
            self.ops+=1;if self.ops>MAX_OPS||self.path.len()>MAX_OPS{return Err(Error::Limit("Type2 operations"));}
            let b=r.byte()?;
            if b>=32||b==28{self.push(number(b,&mut r,false)?)?;continue;}
            match b{
                1|3|18|23=>self.hint()?,
                19|20=>{self.hint()?;r.take(self.stems.div_ceil(8))?;}
                4|21|22=>{
                    let n=if b==21{2}else{1};self.width(self.stack.len()==n+1)?;
                    let a=std::mem::take(&mut self.stack);if a.len()!=n{return Err(Error::Invalid("Type2 moveto operands"));}
                    match b{4=>self.mov(0.,a[0])?,22=>self.mov(a[0],0.)?,_=>self.mov(a[0],a[1])?}
                }
                10|29=>{
                    let index=self.stack.pop().ok_or(Error::Invalid("Type2 subroutine operand"))?;
                    let subrs=if b==10{&self.private.local}else{&self.font.global};
                    let bias=if subrs.len()<1240{107.}else if subrs.len()<33900{1131.}else{32768.};
                    let i=int(index+bias,subrs.len().saturating_sub(1))?;
                    let &(a,z)=subrs.get(i).ok_or(Error::Invalid("Type2 subroutine index"))?;
                    let font=self.font;
                    if self.run(&font.data[a..z],depth+1)?==Flow::End{return Ok(Flow::End);}
                }
                11=>{if depth==0{return Err(Error::Invalid("Type2 return outside subroutine"));}return Ok(Flow::Return);}
                14=>{
                    self.width(self.stack.len()==1||self.stack.len()==5)?;
                    if !self.stack.is_empty(){return Err(Error::Unsupported("Type2 endchar seac"));}
                    self.close();return Ok(Flow::End);
                }
                _=>{
                    let esc=if b==12{Some(r.byte()?)}else{None};
                    let a=std::mem::take(&mut self.stack);
                    match b{
                        5=>{if a.len()<2||a.len()%2!=0{return Err(Error::Invalid("rlineto"));}for p in a.chunks_exact(2){self.line(p[0],p[1])?;}}
                        6|7=>{if a.is_empty(){return Err(Error::Invalid("alternating lineto"));}let mut h=b==6;for x in a{if h{self.line(x,0.)?;}else{self.line(0.,x)?;}h=!h;}}
                        8=>{if a.len()<6||a.len()%6!=0{return Err(Error::Invalid("rrcurveto"));}for p in a.chunks_exact(6){self.curve(p)?;}}
                        24=>{if a.len()<8||(a.len()-2)%6!=0{return Err(Error::Invalid("rcurveline"));}let n=a.len()-2;for p in a[..n].chunks_exact(6){self.curve(p)?;}self.line(a[n],a[n+1])?;}
                        25=>{if a.len()<8||(a.len()-6)%2!=0{return Err(Error::Invalid("rlinecurve"));}let n=a.len()-6;for p in a[..n].chunks_exact(2){self.line(p[0],p[1])?;}self.curve(&a[n..])?;}
                        26|27=>{
                            if a.len()<4||!matches!(a.len()%4,0|1){return Err(Error::Invalid("hh/vvcurveto"));}
                            let extra=a.len()%4;let mut first=if extra==1{a[0]}else{0.};
                            for p in a[extra..].chunks_exact(4){if b==26{self.curve(&[first,p[0],p[1],p[2],0.,p[3]])?;}else{self.curve(&[p[0],first,p[1],p[2],p[3],0.])?;}first=0.;}
                        }
                        30|31=>{
                            if a.len()<4||!matches!(a.len()%4,0|1){return Err(Error::Invalid("hv/vhcurveto"));}
                            let mut h=b==31;let count=a.len()/4;
                            for (i,p) in a[..count*4].chunks_exact(4).enumerate(){
                                let last=if i+1==count&&a.len()%4==1{a[a.len()-1]}else{0.};
                                if h{self.curve(&[p[0],0.,p[1],p[2],last,p[3]])?;}else{self.curve(&[0.,p[0],p[1],p[2],p[3],last])?;}h=!h;
                            }
                        }
                        12=>match esc.unwrap(){
                            34 if a.len()==7=>{self.curve(&[a[0],0.,a[1],a[2],a[3],0.])?;self.curve(&[a[4],0.,a[5],-a[2],a[6],0.])?;}
                            35 if a.len()==13=>{self.curve(&a[..6])?;self.curve(&a[6..12])?;}
                            36 if a.len()==9=>{self.curve(&[a[0],a[1],a[2],a[3],a[4],0.])?;self.curve(&[a[5],0.,a[6],a[7],a[8],-a[1]-a[3]-a[7]])?;}
                            37 if a.len()==11=>{let dx=a[..10].iter().step_by(2).sum::<f64>();let dy=a[1..10].iter().step_by(2).sum::<f64>();self.curve(&a[..6])?;let (x,y)=if dx.abs()>dy.abs(){(a[10],-dy)}else{(-dx,a[10])};self.curve(&[a[6],a[7],a[8],a[9],x,y])?;}
                            _=>return Err(Error::Unsupported("Type2 escaped operator")),
                        }
                        _=>return Err(Error::Unsupported("Type2 operator")),
                    }
                }
            }
        }
        Err(Error::Invalid("truncated Type2 program"))
    }
}
