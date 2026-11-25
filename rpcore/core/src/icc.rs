//! Bounded ICC v2/v4 profile inspection, RGB matrix/TRC and CMYK LUTs.
//! ICC.1:2022 sections 7, 10 and Annex F; source transforms for PDF rendering.
use crate::{Error,Result};
use std::collections::BTreeMap;
pub mod mpe;

const MAX_PROFILE:usize=16*1024*1024;
const MAX_TAGS:usize=1024;
const MAX_CURVE:usize=65536;
pub type Signature=[u8;4];

/// Source of a profile decision during page interpretation/rendering.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ProfileUse{Vector,Image}
/// Native paths currently approximate PDF rendering intents. Alternate is
/// explicitly distinguishable from a native transform even for identity RGB.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ProfilePath{RgbMatrix,GrayTrc,CmykLut{tag:Signature},CmykMab{tag:Signature},Mpe{tag:Signature},Alternate{reason:&'static str}}
/// One record per profile reference and source category on a successful page.
/// Selection/decode can occur even if the affected paint is clipped away.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ProfileDiagnostic{
    pub object_id:(u32,u16),pub source:ProfileUse,pub components:usize,pub path:ProfilePath,
}


fn sig(b:&[u8],p:usize)->Result<Signature>{
    b.get(p..p+4).ok_or(Error::Invalid("ICC truncated field"))?
        .try_into().map_err(|_|Error::Invalid("ICC signature"))
}
fn u32be(b:&[u8],p:usize)->Result<u32>{Ok(u32::from_be_bytes(sig(b,p)?))}
fn u16be(b:&[u8],p:usize)->Result<u16>{
    Ok(u16::from_be_bytes(b.get(p..p+2).ok_or(Error::Invalid("ICC truncated short"))?.try_into().map_err(|_|Error::Invalid("ICC short"))?))
}
fn fixed(b:&[u8],p:usize)->Result<f64>{Ok(i32::from_be_bytes(sig(b,p)?) as f64/65536.)}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Tag{pub offset:usize,pub size:usize,pub kind:Signature}
/// Inspection of a borrowed profile, not proof of ICC conformance.
#[derive(Debug)]
pub struct Profile<'a>{
    data:&'a [u8],
    pub version_major:u8,pub class:Signature,pub color_space:Signature,pub pcs:Signature,
    pub intent:u32,pub illuminant:[f64;3],
    tags:BTreeMap<Signature,Tag>,
}
impl<'a> Profile<'a>{
    pub fn parse(bytes:&'a [u8])->Result<Self>{
        if bytes.len()>MAX_PROFILE{return Err(Error::Limit("ICC profile bytes"));}
        if bytes.len()<132{return Err(Error::Invalid("ICC profile header"));}
        let size=u32be(bytes,0)? as usize;
        if size!=bytes.len()||&bytes[36..40]!=b"acsp"{return Err(Error::Invalid("ICC size or magic"));}
        let version_major=bytes[8];
        if !matches!(version_major,2|4){return Err(Error::Unsupported("ICC version"));}
        let class=sig(bytes,12)?;let color_space=sig(bytes,16)?;let pcs=sig(bytes,20)?;
        let intent=u32be(bytes,64)?;
        if intent>3{return Err(Error::Invalid("ICC rendering intent"));}
        let illuminant=[fixed(bytes,68)?,fixed(bytes,72)?,fixed(bytes,76)?];
        let count=u32be(bytes,128)? as usize;
        if count>MAX_TAGS{return Err(Error::Limit("ICC tag count"));}
        let end=132+12*count;
        if end>size{return Err(Error::Invalid("ICC tag table"));}
        let mut tags=BTreeMap::new();let mut ranges=Vec::new();
        for i in 0..count{
            let p=132+12*i;let name=sig(bytes,p)?;
            let offset=u32be(bytes,p+4)? as usize;let len=u32be(bytes,p+8)? as usize;
            let stop=offset.checked_add(len).ok_or(Error::Invalid("ICC tag range"))?;
            if offset<end||offset%4!=0||len<8||stop>size{return Err(Error::Invalid("ICC tag bounds"));}
            let kind=sig(bytes,offset)?;
            if tags.insert(name,Tag{offset,size:len,kind}).is_some(){return Err(Error::Invalid("ICC duplicate tag"));}
            ranges.push((offset,stop));
        }
        ranges.sort_unstable();ranges.dedup();
        if ranges.windows(2).any(|w|w[0].1>w[1].0){return Err(Error::Invalid("ICC overlapping tags"));}
        Ok(Self{data:bytes,version_major,class,color_space,pcs,intent,illuminant,tags})
    }
    pub fn tags(&self)->&BTreeMap<Signature,Tag>{&self.tags}
    pub fn tag(&self,name:&Signature)->Option<&'a [u8]>{
        let t=self.tags.get(name)?;Some(&self.data[t.offset..t.offset+t.size])
    }
    fn xyz(&self,name:&Signature)->Result<[f64;3]>{
        let b=self.tag(name).ok_or(Error::Unsupported("ICC missing XYZ tag"))?;
        if b.len()!=20||sig(b,0)?!=*b"XYZ "||b[4..8]!=[0;4]{return Err(Error::Invalid("ICC XYZ tag"));}
        Ok([fixed(b,8)?,fixed(b,12)?,fixed(b,16)?])
    }
    /// Parse ICC s15Fixed16ArrayType CHAD (row-major, nine coefficients).
    /// Reject singular or ill-conditioned matrices before exposing them.
    pub fn chromatic_adaptation(&self)->Result<Option<[[f64;3];3]>>{
        let Some(b)=self.tag(b"chad") else{return Ok(None);};
        if b.len()!=44||sig(b,0)?!=*b"sf32"||b[4..8]!=[0;4]{
            return Err(Error::Invalid("ICC CHAD tag"));
        }
        let mut m=[[0.;3];3];
        for i in 0..3{for j in 0..3{m[i][j]=fixed(b,8+4*(3*i+j))?;}}
        invert_chad(m)?;
        Ok(Some(m))
    }
    /// Recover the pre-adaptation D50 white for diagnostics, not for the
    /// fully adapted absolute rendering path.
    pub fn unadapted_pcs_white(&self)->Result<Option<[f64;3]>>{
        let Some(m)=self.chromatic_adaptation()? else{return Ok(None);};
        let inv=invert_chad(m)?;
        let w=inv.map(|r|r[0]*self.illuminant[0]+r[1]*self.illuminant[1]+r[2]*self.illuminant[2]);
        if w.iter().any(|v|!v.is_finite()||*v<=0.||*v>4.){
            return Err(Error::Invalid("ICC CHAD white"));
        }
        Ok(Some(w))
    }
    /// Fully adapted (state 1) absolute matrix/TRC path. Legacy v2 display
    /// profiles use D50 media white under the reference CMM convention.
    pub fn absolute_matrix_scale(&self)->Result<[f64;3]>{
        if !matches!(&self.color_space,b"RGB "|b"GRAY")||
            !matches!(&self.class,b"mntr"|b"scnr"|b"prtr")||self.pcs!=*b"XYZ "{
            return Err(Error::Unsupported("ICC absolute matrix profile"));
        }
        if self.illuminant.iter().zip([0.9642,1.,0.8249]).any(|(a,b)|(*a-b).abs()>0.001){
            return Err(Error::Unsupported("ICC non-D50 PCS"));
        }
        // Validate CHAD if present without undoing full adaptation.
        self.chromatic_adaptation()?;
        let white=if self.version_major==2&&self.class==*b"mntr"{[0.9642,1.,0.8249]}else{self.xyz(b"wtpt")?};
        if white.iter().any(|v|!v.is_finite()||*v<=0.||*v>2.){
            return Err(Error::Invalid("ICC absolute media white"));
        }
        Ok(std::array::from_fn(|i|white[i]/[0.9642,1.,0.8249][i]))
    }
    /// Media-relative to ICC-absolute PCS scaling for CMYK output profiles.
    /// Do not apply to native D2B3, or assume display/input profile semantics.
    pub fn absolute_output_scale(&self)->Result<[f64;3]>{
        self.check_cmyk()?;
        let white=self.xyz(b"wtpt")?;
        if white.iter().any(|v|!v.is_finite()||*v<=0.||*v>2.){
            return Err(Error::Invalid("ICC absolute media white"));
        }
        Ok(std::array::from_fn(|i|white[i]/[0.9642,1.,0.8249][i]))
    }
    /// Explicit relative-colorimetric matrix path to PCS XYZ D50.
    /// Refuse LUT-bearing profiles rather than silently override their LUT.
    pub fn rgb_matrix(&self)->Result<RgbMatrix>{
        if !matches!(&self.class,b"mntr"|b"scnr")||self.color_space!=*b"RGB "||self.pcs!=*b"XYZ "{
            return Err(Error::Unsupported("ICC RGB matrix profile class or space"));
        }
        if self.illuminant.iter().zip([0.9642,1.,0.8249]).any(|(a,b)|(*a-b).abs()>0.001){
            return Err(Error::Unsupported("ICC non-D50 PCS"));
        }
        if [*b"A2B0",*b"A2B1",*b"A2B2",*b"D2B0",*b"D2B1",*b"D2B2",*b"D2B3"].iter().any(|s|self.tags.contains_key(s)){
            return Err(Error::Unsupported("ICC LUT profile"));
        }
        let columns=[self.xyz(b"rXYZ")?,self.xyz(b"gXYZ")?,self.xyz(b"bXYZ")?];
        let curve=|name:&Signature|Curve::parse(self.tag(name).ok_or(Error::Unsupported("ICC missing TRC"))?);
        Ok(RgbMatrix{columns,curves:[curve(b"rTRC")?,curve(b"gTRC")?,curve(b"bTRC")?]})
    }
    /// Native media-relative monochrome TRC to D50 XYZ. The media white
    /// is not an absolute-colorimetric scaling factor on this path.
    pub fn gray_trc(&self)->Result<GrayTrc>{
        if !matches!(&self.class,b"mntr"|b"scnr"|b"prtr")||self.color_space!=*b"GRAY"||self.pcs!=*b"XYZ "{
            return Err(Error::Unsupported("ICC gray TRC class or space"));
        }
        if self.illuminant.iter().zip([0.9642,1.,0.8249]).any(|(a,b)|(*a-b).abs()>0.001){
            return Err(Error::Unsupported("ICC non-D50 PCS"));
        }
        if [*b"A2B0",*b"A2B1",*b"A2B2",*b"D2B0",*b"D2B1",*b"D2B2",*b"D2B3"].iter().any(|s|self.tags.contains_key(s)){
            return Err(Error::Unsupported("ICC gray LUT profile"));
        }
        let white=self.xyz(b"wtpt")?;
        if white.iter().any(|v|*v<=0.){return Err(Error::Invalid("ICC gray media white"));}
        Ok(GrayTrc{curve:Curve::parse(self.tag(b"kTRC").ok_or(Error::Unsupported("ICC missing gray TRC"))?)?})
    }
    /// Inspect a v4 D2B/B2D multi-process tag, without executing it.
    /// Only known GRAY/RGB/CMYK device spaces and Lab/XYZ PCS are admitted.
    pub fn mpe(&self,tag:&Signature)->Result<mpe::Mpe<'a>>{
        if self.version_major!=4{return Err(Error::Unsupported("ICC MPE version"));}
        let device=match &self.color_space{b"GRAY"=>1,b"RGB "=>3,b"CMYK"=>4,_=>return Err(Error::Unsupported("ICC MPE device space"))};
        if !matches!(&self.pcs,b"XYZ "|b"Lab "){return Err(Error::Unsupported("ICC MPE PCS"));}
        let (p,q)=match tag{
            b"D2B0"|b"D2B1"|b"D2B2"|b"D2B3"=>(device,3),
            b"B2D0"|b"B2D1"|b"B2D2"|b"B2D3"=>(3,device),
            _=>return Err(Error::Unsupported("ICC MPE tag signature")),
        };
        let m=mpe::Mpe::parse(self.tag(tag).ok_or(Error::Unsupported("ICC missing MPE tag"))?)?;
        if m.inputs()!=p||m.outputs()!=q{return Err(Error::Invalid("ICC MPE profile channels"));}
        Ok(m)
    }
    /// Prepare the exact requested floating-point transform. Legacy fallback
    /// is deliberately left to the caller; absolute intent is not emulated.
    pub fn mpe_transform(&self, direction: MpeDirection, intent: RenderingIntent) -> Result<MpeTransform<'a>> {
        if !matches!(&self.class,b"scnr"|b"mntr"|b"prtr"|b"spac"){
            return Err(Error::Unsupported("ICC MPE profile class"));
        }
        if self.illuminant.iter().zip([0.9642,1.,0.8249]).any(|(a,b)|(*a-b).abs()>0.001){
            return Err(Error::Unsupported("ICC non-D50 PCS"));
        }
        let tag=direction.tag(intent);
        let parsed=self.mpe(&tag)?;
        let program=parsed.prepare()?;
        Ok(MpeTransform{tag, direction, intent, pcs:self.pcs, program})
    }

    /// Explicit legacy A2B0 path for CMYK output profiles. Returns encoded PCS
    /// channels, not display RGB. No automatic intent selection or LUT fallback.
    pub fn cmyk_a2b0(&self)->Result<CmykLut<'a>>{
        self.cmyk_a2b(b"A2B0")
    }
    pub(crate) fn cmyk_a2b(&self,tag:&Signature)->Result<CmykLut<'a>>{
        self.check_cmyk()?;
        let b=self.tag(tag).ok_or(Error::Unsupported("ICC missing A2B tag"))?;
        CmykLut::parse(b,self.pcs)
    }
    /// ICC v4 lutAToBType, CMYK output profile source transform.
    pub fn cmyk_mab(&self,tag:&Signature)->Result<CmykMab>{
        self.check_cmyk()?;
        if self.version_major!=4{return Err(Error::Unsupported("ICC mAB version"));}
        if !matches!(tag,b"A2B0"|b"A2B1"|b"A2B2"){return Err(Error::Unsupported("ICC mAB tag"));}
        CmykMab::parse(self.tag(tag).ok_or(Error::Unsupported("ICC missing A2B tag"))?,self.pcs)
    }
    fn check_cmyk(&self)->Result<()>{
        if self.class!=*b"prtr"||self.color_space!=*b"CMYK"||!matches!(&self.pcs,b"XYZ "|b"Lab "){
            return Err(Error::Unsupported("ICC CMYK output profile class or space"));
        }
        if self.illuminant.iter().zip([0.9642,1.,0.8249]).any(|(a,b)|(*a-b).abs()>0.001){
            return Err(Error::Unsupported("ICC non-D50 PCS"));
        }
        Ok(())
    }

}
/// Bounded Gauss-Jordan 3x3 inversion with partial pivoting.
pub fn invert_chad(m:[[f64;3];3])->Result<[[f64;3];3]>{
    if m.iter().flatten().any(|v|!v.is_finite()||v.abs()>64.){
        return Err(Error::Invalid("ICC CHAD coefficient"));
    }
    let norm=m.iter().map(|r|r.iter().map(|v|v.abs()).sum::<f64>()).fold(0.,f64::max);
    if norm==0.{return Err(Error::Invalid("ICC singular CHAD"));}
    let mut a=[[0.;6];3];
    for i in 0..3{a[i][..3].copy_from_slice(&m[i]);a[i][i+3]=1.;}
    for k in 0..3{
        let mut p=k;
        for i in k+1..3{if a[i][k].abs()>a[p][k].abs(){p=i;}}
        if a[p][k].abs()<=norm*1e-10{return Err(Error::Invalid("ICC singular CHAD"));}
        a.swap(k,p);let pivot=a[k][k];
        for v in &mut a[k]{*v/=pivot;}
        let row=a[k];
        for i in 0..3{if i!=k{let v=a[i][k];for j in 0..6{a[i][j]-=v*row[j];}}}
    }
    let inv:[[f64;3];3]=std::array::from_fn(|i|std::array::from_fn(|j|a[i][j+3]));
    let inv_norm=inv.iter().map(|r|r.iter().map(|v|v.abs()).sum::<f64>()).fold(0.,f64::max);
    if !inv_norm.is_finite()||norm*inv_norm>1e6{return Err(Error::Invalid("ICC ill-conditioned CHAD"));}
    Ok(inv)
}
#[derive(Debug)]
enum Curve{Identity,Gamma(f64),Table(Vec<u16>),Parametric{kind:u16,p:[f64;7]}}
impl Curve{
    fn parse(b:&[u8])->Result<Self>{
        if b.len()<12||b[4..8]!=[0;4]{return Err(Error::Invalid("ICC curve header"));}
        match &sig(b,0)?{
            b"curv"=>{
                let n=u32be(b,8)? as usize;
                if n>MAX_CURVE{return Err(Error::Limit("ICC curve entries"));}
                if b.len()!=12+2*n{return Err(Error::Invalid("ICC curve size"));}
                match n{
                    0=>Ok(Self::Identity),
                    1=>{let g=f64::from(u16be(b,12)?)/256.;if g<=0.{return Err(Error::Invalid("ICC gamma"));}Ok(Self::Gamma(g))}
                    _=>{
                        let v=(0..n).map(|i|u16be(b,12+2*i)).collect::<Result<Vec<_>>>()?;
                        if v.windows(2).any(|w|w[0]>w[1]){return Err(Error::Unsupported("ICC nonmonotone TRC"));}
                        Ok(Self::Table(v))
                    }
                }
            }
            b"para"=>{
                let kind=u16be(b,8)?;
                let n=match kind{0=>1,1=>3,2=>4,3=>5,4=>7,_=>return Err(Error::Unsupported("ICC parametric curve function"))};
                if b.len()!=12+4*n||u16be(b,10)?!=0{return Err(Error::Invalid("ICC parametric curve size"));}
                let mut p=[0.;7];for i in 0..n{p[i]=fixed(b,12+4*i)?;}
                if p[0]<=0.{return Err(Error::Invalid("ICC gamma"));}
                if kind==0{return Ok(Self::Gamma(p[0]));}
                if p[1]<=0.{return Err(Error::Unsupported("ICC parametric slope"));}
                if kind>=3{
                    let d=p[4];
                    if !(0. ..=1.).contains(&d)||p[3]<0.{return Err(Error::Unsupported("ICC parametric linear branch"));}
                    if p[1]*d+p[2]< -1./65536.{return Err(Error::Unsupported("ICC parametric power domain"));}
                    // Allow small fixed-point rounding discrepancies at the join,
                    // but reject larger discontinuities in this bounded subset.
                    let lower=p[3]*d+if kind==4{p[6]}else{0.};
                    let upper=(p[1]*d+p[2]).max(0.).powf(p[0])+if kind==4{p[5]}else{0.};
                    if !upper.is_finite()||(upper-lower).abs()>0.001{return Err(Error::Unsupported("ICC parametric discontinuity"));}
                }
                let curve=Self::Parametric{kind,p};
                let lo=curve.eval(0.);let hi=curve.eval(1.);
                if !lo.is_finite()||!hi.is_finite()||lo< -0.001||hi>1.001||hi<lo{
                    return Err(Error::Unsupported("ICC parametric range"));
                }
                Ok(curve)
            }
            _=>Err(Error::Unsupported("ICC curve type")),
        }
    }
    fn eval(&self,x:f64)->f64{
        let x=x.clamp(0.,1.);
        match self{
            Self::Identity=>x,Self::Gamma(g)=>x.powf(*g),
            Self::Parametric{kind,p}=>{
                let [g,a,b,c,d,e,f]=*p;
                let power=|| (a*x+b).max(0.).powf(g);
                match kind{
                    1=>if x>= -b/a{power()}else{0.},
                    2=>if x>= -b/a{power()+c}else{c},
                    3=>if x>=d{power()}else{c*x},
                    4=>if x>=d{power()+e}else{c*x+f},
                    _=>unreachable!(),
                }
            }
            Self::Table(t)=>{
                let p=x*(t.len()-1)as f64;let i=p.floor()as usize;
                let f=p-i as f64;
                (f64::from(t[i])*(1.-f)+f64::from(t[(i+1).min(t.len()-1)])*f)/65535.
            }
        }
    }
}
#[derive(Debug)]
pub struct GrayTrc{curve:Curve}
impl GrayTrc{
    pub fn to_xyz_d50(&self,gray:f64)->Result<[f64;3]>{
        if !gray.is_finite(){return Err(Error::Invalid("ICC gray input"));}
        let y=self.curve.eval(gray).clamp(0.,1.);
        Ok([0.9642*y,y,0.8249*y])
    }
}
#[derive(Debug)]
pub struct RgbMatrix{columns:[[f64;3];3],curves:[Curve;3]}
impl RgbMatrix{
    /// Return un-clipped PCS XYZ (D50), not display RGB.
    pub fn to_xyz_d50(&self,rgb:[f64;3])->Result<[f64;3]>{
        if rgb.iter().any(|v|!v.is_finite()){return Err(Error::Invalid("ICC RGB input"));}
        let v:[f64;3]=std::array::from_fn(|i|self.curves[i].eval(rgb[i]));
        Ok(std::array::from_fn(|j|(0..3).map(|i|self.columns[i][j]*v[i]).sum()))
    }
}



/// Owned, validated CMYK lutAToBType pipeline. Supports four input channels,
/// three output channels, independent 2..33 grid sizes, 8/16-bit CLUT,
/// A/CLUT/B and A/CLUT/M/matrix/B combinations. No mBA or MPE execution.
#[derive(Debug)]
pub struct CmykMab{
    pcs:Signature,a:Vec<Curve>,b:Vec<Curve>,m:Vec<Curve>,
    matrix:Option<[f64;12]>,grid:[usize;4],stride:[usize;4],clut:Vec<u8>,word:usize,
}
fn embedded_curves(data:&[u8],start:usize,count:usize)->Result<(Vec<Curve>,usize)>{
    let mut pos=start;let mut curves=Vec::with_capacity(count);
    for _ in 0..count{
        let tail=data.get(pos..).ok_or(Error::Invalid("ICC mAB curve offset"))?;
        let size=match &sig(tail,0)?{
            b"curv"=>{
                let n=u32be(tail,8)? as usize;
                if n>MAX_CURVE{return Err(Error::Limit("ICC curve entries"));}
                12+2*n
            }
            b"para"=>12+4*match u16be(tail,8)?{0=>1,1=>3,2=>4,3=>5,4=>7,_=>return Err(Error::Unsupported("ICC parametric curve function"))},
            _=>return Err(Error::Unsupported("ICC mAB curve type")),
        };
        let aligned=(size+3)&!3;
        let raw=tail.get(..size).ok_or(Error::Invalid("ICC mAB curve bounds"))?;
        // Final curve padding may be external to the tag; intermediate
        // padding must be present, zero, and aligned.
        let end=if tail.len()<aligned && curves.len()+1==count{size}else{aligned};
        let pad=tail.get(size..end).ok_or(Error::Invalid("ICC mAB curve padding"))?;
        if pad.iter().any(|&x|x!=0){return Err(Error::Invalid("ICC mAB curve padding"));}
        curves.push(Curve::parse(raw)?);pos+=end;
    }
    Ok((curves,pos))
}
impl CmykMab{
    fn parse(data:&[u8],pcs:Signature)->Result<Self>{
        if data.len()>MAX_PROFILE{return Err(Error::Limit("ICC mAB bytes"));}
        if data.len()<32||&data[..4]!=b"mAB "||data[4..8]!=[0;4]||data[10..12]!=[0;2]{
            return Err(Error::Invalid("ICC mAB header"));
        }
        if data[8..10]!=[4,3]{return Err(Error::Unsupported("ICC mAB channel count"));}
        if !matches!(&pcs,b"Lab "|b"XYZ "){return Err(Error::Unsupported("ICC mAB PCS"));}
        let mut offsets=[0usize;5];
        for (i,o) in offsets.iter_mut().enumerate(){
            *o=u32be(data,12+4*i)? as usize;
            if *o!=0&&(*o<32||*o%4!=0||*o>=data.len()){
                return Err(Error::Invalid("ICC mAB element offset"));
            }
        }
        let [bo,mo,mc,co,ao]=offsets;
        if bo==0||co==0||ao==0||(mo==0)!=(mc==0){return Err(Error::Invalid("ICC mAB stage combination"));}
        let (a,ae)=embedded_curves(data,ao,4)?;let (b,be)=embedded_curves(data,bo,3)?;
        let mut ranges=vec![(ao,ae,0usize),(bo,be,0)];
        let (m,matrix)=if mo!=0{
            let (m,me)=embedded_curves(data,mc,3)?;
            let raw=data.get(mo..mo+48).ok_or(Error::Invalid("ICC mAB matrix bounds"))?;
            let mut mat=[0.;12];for i in 0..12{mat[i]=fixed(raw,i*4)?;}
            ranges.push((mc,me,0));ranges.push((mo,mo+48,1));(m,Some(mat))
        }else{(Vec::new(),None)};
        let h=data.get(co..co+20).ok_or(Error::Invalid("ICC mAB CLUT header"))?;
        if h[4..16]!=[0;12]||h[17..20]!=[0;3]{return Err(Error::Invalid("ICC mAB CLUT reserved"));}
        let word=h[16] as usize;
        if !matches!(word,1|2){return Err(Error::Unsupported("ICC mAB CLUT precision"));}
        let grid:[usize;4]=std::array::from_fn(|i|h[i] as usize);
        if grid.iter().any(|&v|v<2){return Err(Error::Invalid("ICC mAB CLUT grid"));}
        if grid.iter().any(|&v|v>33){return Err(Error::Limit("ICC mAB CLUT grid"));}
        let stride=[grid[1]*grid[2]*grid[3],grid[2]*grid[3],grid[3],1];
        let end=co+20+grid.iter().product::<usize>()*3*word;
        let clut=data.get(co+20..end).ok_or(Error::Invalid("ICC mAB CLUT bounds"))?;
        ranges.push((co,end,2));
        // Exact shared curve sequences are allowed. Cross-kind or partial
        // overlap is rejected; no offset is followed recursively.
        ranges.sort_unstable();ranges.dedup();
        if ranges.windows(2).any(|w|w[0].1>w[1].0){return Err(Error::Invalid("ICC mAB overlapping elements"));}
        Ok(Self{pcs,a,b,m,matrix,grid,stride,clut:clut.to_vec(),word})
    }
    fn value(&self,index:usize)->f64{
        let p=index*self.word;
        if self.word==1{f64::from(self.clut[p])/255.}
        else{f64::from(u16::from_be_bytes([self.clut[p],self.clut[p+1]]))/65535.}
    }
    pub fn evaluate_encoded(&self,cmyk:[f64;4])->Result<[f64;3]>{
        if cmyk.iter().any(|v|!v.is_finite()){return Err(Error::Invalid("ICC CMYK input"));}
        let mut base=0;let mut f=[0.;4];
        for i in 0..4{
            let p=self.a[i].eval(cmyk[i]).clamp(0.,1.)*(self.grid[i]-1) as f64;
            let lo=(p.floor()as usize).min(self.grid[i]-2);
            base+=lo*self.stride[i];f[i]=p-lo as f64;
        }
        let mut out=[0.;3];
        for c in 0..16{
            let mut i=base;let mut w=1.;
            for j in 0..4{
                if c&(1<<j)!=0{i+=self.stride[j];w*=f[j];}else{w*=1.-f[j];}
            }
            if w!=0.{for k in 0..3{out[k]+=w*self.value(3*i+k);}}
        }
        if let Some(mat)=self.matrix{
            let v:[f64;3]=std::array::from_fn(|i|self.m[i].eval(out[i]).clamp(0.,1.));
            out=std::array::from_fn(|i|(mat[3*i]*v[0]+mat[3*i+1]*v[1]+mat[3*i+2]*v[2]+mat[9+i]).clamp(0.,1.));
        }
        Ok(std::array::from_fn(|i|self.b[i].eval(out[i]).clamp(0.,1.)))
    }
    /// v4 full-range PCS Lab (not legacy mft2's 0xFF00 white).
    pub fn to_lab_d50(&self,cmyk:[f64;4])->Result<[f64;3]>{
        if self.pcs!=*b"Lab "{return Err(Error::Unsupported("ICC mAB Lab PCS"));}
        let v=self.evaluate_encoded(cmyk)?;
        Ok([100.*v[0],255.*v[1]-128.,255.*v[2]-128.])
    }
    pub fn to_xyz_d50(&self,cmyk:[f64;4])->Result<[f64;3]>{
        if self.pcs==*b"XYZ "{return Ok(self.evaluate_encoded(cmyk)?.map(|v|v*65535./32768.));}
        let [l,a,b]=self.to_lab_d50(cmyk)?;
        let fy=(l+16.)/116.;
        let inv=|t:f64|if t>6./29.{t*t*t}else{3.*(6_f64/29.).powi(2)*(t-4./29.)};
        Ok([0.9642*inv(fy+a/500.),inv(fy),0.8249*inv(fy-b/200.)])
    }
}

/// Borrowed legacy lut8Type/lut16Type CMYK A2B0 evaluator.
/// Input, CLUT and output tables remain in the original bounded profile.
/// Encoded PCS values are normalized code values; Lab decoding is not implied.
#[derive(Debug)]
pub struct CmykLut<'a>{
    bytes:&'a [u8],pcs:Signature,word:usize,grid:usize,
    input_entries:usize,output_entries:usize,input:usize,clut:usize,output:usize,
}
/// Private, validated owned LUT for the PDF color path. The bytes and layout
/// cannot be changed independently; borrowing it never reparses the tag.
#[derive(Debug)]
pub(crate) struct OwnedCmykLut{
    bytes:Vec<u8>,pcs:Signature,word:usize,grid:usize,
    input_entries:usize,output_entries:usize,input:usize,clut:usize,output:usize,
}
impl OwnedCmykLut{
    fn view(&self)->CmykLut<'_>{
        CmykLut{bytes:&self.bytes,pcs:self.pcs,word:self.word,grid:self.grid,
            input_entries:self.input_entries,output_entries:self.output_entries,input:self.input,clut:self.clut,output:self.output}
    }
    pub(crate) fn to_xyz_d50(&self,cmyk:[f64;4])->Result<[f64;3]>{
        self.view().to_xyz_d50(cmyk)
    }
}
impl<'a> CmykLut<'a>{
    pub(crate) fn parse(b:&'a [u8],pcs:Signature)->Result<Self>{
        if b.len()<48||b[4..8]!=[0;4]||b[11]!=0{return Err(Error::Invalid("ICC LUT header"));}
        if b[8]!=4||b[9]!=3{return Err(Error::Unsupported("ICC LUT channel count"));}
        let grid=b[10] as usize;
        if grid<2{return Err(Error::Invalid("ICC LUT grid"));}
        if grid>33{return Err(Error::Limit("ICC LUT grid"));}
        // For non-XYZ input the legacy matrix is required to be identity.
        for i in 0..9{
            if fixed(b,12+4*i)?!=if i%4==0{1.}else{0.}{
                return Err(Error::Unsupported("ICC nonidentity CMYK LUT matrix"));
            }
        }
        let (word,input,input_entries,output_entries)=match &sig(b,0)?{
            b"mft1"=>(1usize,48usize,256usize,256usize),
            b"mft2"=>{
                let n=u16be(b,48)? as usize;let m=u16be(b,50)? as usize;
                if n<2||m<2{return Err(Error::Invalid("ICC LUT table entries"));}
                if n>4096||m>4096{return Err(Error::Limit("ICC LUT table entries"));}
                (2,52,n,m)
            }
            _=>return Err(Error::Unsupported("ICC A2B0 LUT type")),
        };
        // With bounded grid/table counts these products fit even 32-bit usize.
        let clut=input+word*4*input_entries;
        let output=clut+word*3*grid.pow(4);
        let end=output+word*3*output_entries;
        if b.len()!=end{return Err(Error::Invalid("ICC LUT size"));}
        Ok(Self{bytes:b,pcs,word,grid,input_entries,output_entries,input,clut,output})
    }
    pub(crate) fn to_owned_lut(&self)->OwnedCmykLut{
        OwnedCmykLut{bytes:self.bytes.to_vec(),pcs:self.pcs,word:self.word,grid:self.grid,
            input_entries:self.input_entries,output_entries:self.output_entries,input:self.input,clut:self.clut,output:self.output}
    }
    pub fn pcs(&self)->Signature{self.pcs}
    pub fn encoding_bits(&self)->usize{self.word*8}
    fn value(&self,base:usize,index:usize)->f64{
        let p=base+self.word*index;
        if self.word==1{f64::from(self.bytes[p])/255.}
        else{f64::from(u16::from_be_bytes([self.bytes[p],self.bytes[p+1]]))/65535.}
    }
    fn table(&self,base:usize,channel:usize,n:usize,x:f64)->f64{
        let p=x.clamp(0.,1.)*(n-1) as f64;
        let lo=(p.floor() as usize).min(n-2);let f=p-lo as f64;
        let a=self.value(base,channel*n+lo);
        let b=self.value(base,channel*n+lo+1);
        a*(1.-f)+b*f
    }
    /// Four-dimensional multilinear interpolation (16 corners).
    /// Device values are clamped to [0,1]; non-finite inputs are rejected.
    /// The last input channel varies fastest in ICC CLUT storage.
    /// Return normalized PCS code values, not physical Lab/XYZ or display RGB.
    pub fn evaluate_encoded(&self,cmyk:[f64;4])->Result<[f64;3]>{
        if cmyk.iter().any(|x|!x.is_finite()){return Err(Error::Invalid("ICC CMYK input"));}
        let mut low=[0usize;4];let mut frac=[0.;4];
        for i in 0..4{
            let p=self.table(self.input,i,self.input_entries,cmyk[i])*(self.grid-1) as f64;
            low[i]=(p.floor() as usize).min(self.grid-2);frac[i]=p-low[i] as f64;
        }
        let stride=[self.grid.pow(3),self.grid.pow(2),self.grid,1];
        let base=(0..4).map(|i|low[i]*stride[i]).sum::<usize>();
        let mut out=[0.;3];
        for corner in 0..16{
            let mut w=1.;let mut index=base;
            for i in 0..4{
                let hi=(corner>>i)&1;
                index+=hi*stride[i];
                w*=if hi==1{frac[i]}else{1.-frac[i]};
            }
            // Boundary samples need fewer nonzero corners; preserve the
            // original summation order and weights for interior samples.
            if w==0.{continue;}
            for (j,v) in out.iter_mut().enumerate(){*v+=w*self.value(self.clut,3*index+j);}
        }
        Ok(std::array::from_fn(|i|self.table(self.output,i,self.output_entries,out[i])))
    }
    /// Decode physical CIELAB using the legacy tag encoding, including mft2
    /// in v4 profiles. Do not substitute the newer full-range v4 Lab encoding.
    pub fn to_lab_d50(&self,cmyk:[f64;4])->Result<[f64;3]>{
        if self.pcs!=*b"Lab "{return Err(Error::Unsupported("ICC LUT Lab PCS"));}
        let v=self.evaluate_encoded(cmyk)?;
        let scale=if self.word==1{255.}else{65535./256.};
        Ok([v[0]*scale*100./255.,v[1]*scale-128.,v[2]*scale-128.])
    }
    /// Return physical D50 XYZ, without display gamut clipping.
    /// Legacy 8-bit XYZ remains unsupported.
    pub fn to_xyz_d50(&self,cmyk:[f64;4])->Result<[f64;3]>{
        if self.pcs==*b"Lab "{
            let [l,a,b]=self.to_lab_d50(cmyk)?;
            let fy=(l+16.)/116.;
            let inv=|t:f64|if t>6./29.{t*t*t}else{3.*(6_f64/29.).powi(2)*(t-4./29.)};
            return Ok([0.9642*inv(fy+a/500.),inv(fy),0.8249*inv(fy-b/200.)]);
        }
        if self.pcs!=*b"XYZ "||self.word!=2{return Err(Error::Unsupported("ICC LUT XYZ PCS encoding"));}
        Ok(self.evaluate_encoded(cmyk)?.map(|v|v*65535./32768.))
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn put(b:&mut [u8],p:usize,v:u32){b[p..p+4].copy_from_slice(&v.to_be_bytes());}
    fn xyz(v:[f64;3])->Vec<u8>{
        let mut b=b"XYZ \0\0\0\0".to_vec();
        for x in v{b.extend_from_slice(&((x*65536.).round()as i32).to_be_bytes());}b
    }
    fn profile()->Vec<u8>{
        // Original synthetic matrix with one shared gamma tag.
        let mut b=vec![0;132+6*12];b[8]=4;
        b[12..24].copy_from_slice(b"mntrRGB XYZ ");
        b[36..40].copy_from_slice(b"acsp");
        for (i,x) in [0.9642_f64,1.,0.8249].into_iter().enumerate(){put(&mut b,68+i*4,(x*65536.).round()as u32);}
        put(&mut b,128,6);
        let names=[*b"rXYZ",*b"gXYZ",*b"bXYZ",*b"rTRC",*b"gTRC",*b"bTRC"];
        for i in 0..3{
            let p=132+i*12;let off=b.len();let mut v=[0.;3];v[i]=1.;
            b[p..p+4].copy_from_slice(&names[i]);put(&mut b,p+4,off as u32);put(&mut b,p+8,20);
            b.extend(xyz(v));
        }
        let off=b.len();
        b.extend_from_slice(b"curv\0\0\0\0\0\0\0\x01\x02\0");
        for i in 3..6{
            let p=132+i*12;b[p..p+4].copy_from_slice(&names[i]);put(&mut b,p+4,off as u32);put(&mut b,p+8,14);
        }
        while b.len()%4!=0{b.push(0);}
        let size=b.len()as u32;put(&mut b,0,size);b
    }
    #[test]fn shared_tag_and_matrix_gamma(){
        let b=profile();let p=Profile::parse(&b).unwrap();
        assert_eq!(p.tags().len(),6);assert_eq!(p.tags()[b"rTRC"],p.tags()[b"gTRC"]);
        let t=p.rgb_matrix().unwrap();
        assert_eq!(t.to_xyz_d50([0.5,0.25,1.]).unwrap(),[0.25,0.0625,1.]);
        assert!(t.to_xyz_d50([f64::NAN,0.,0.]).is_err());
    }
    #[test]fn table_and_parametric_curves(){
        let t=Curve::parse(b"curv\0\0\0\0\0\0\0\x03\0\0\x80\0\xff\xff").unwrap();
        assert!((t.eval(0.25)-0.2500038).abs()<1e-6);
        assert_eq!(t.eval(1.),1.);
        let p=Curve::parse(b"para\0\0\0\0\0\0\0\0\0\x02\0\0").unwrap();
        assert_eq!(p.eval(0.5),0.25);
        assert!(Curve::parse(b"curv\0\0\0\0\0\0\0\x02\xff\xff\0\0").is_err());
    }
    #[test]fn rejects_corrupt_directory(){
        let b=profile();
        for n in 0..b.len(){assert!(Profile::parse(&b[..n]).is_err());}
        let mut x=b.clone();put(&mut x,128,1025);assert!(matches!(Profile::parse(&x),Err(Error::Limit(_))));
        let mut x=b.clone();x[144..148].copy_from_slice(b"rXYZ");assert!(Profile::parse(&x).is_err());
        let mut x=b.clone();let first=u32be(&x,136).unwrap();put(&mut x,148,first+4);assert!(Profile::parse(&x).is_err());
        let mut x=b.clone();put(&mut x,136,u32::MAX);assert!(Profile::parse(&x).is_err());
    }
    #[test]fn rejects_wrong_profile_path(){
        let mut b=profile();b[16..20].copy_from_slice(b"CMYK");
        assert!(Profile::parse(&b).unwrap().rgb_matrix().is_err());
        let mut b=profile();b[132..136].copy_from_slice(b"A2B0");
        assert!(Profile::parse(&b).unwrap().rgb_matrix().is_err());
    }

    // Independently specified analytic fixture: input C reversed, CLUT channels
    // (C*M, Y, K), then output Y inverted. Each axis differs, exposing stride
    // and channel-order errors without relying on evaluator-generated values.
    fn lut_profile(word:usize)->Vec<u8>{
        let n=if word==1{256}else{3};
        let mut tag=vec![0;if word==1{48}else{52}];
        tag[..4].copy_from_slice(if word==1{b"mft1"}else{b"mft2"});
        tag[8..11].copy_from_slice(&[4,3,2]);
        for i in [0,4,8]{put(&mut tag,12+4*i,65536);}
        if word==2{
            tag[48..50].copy_from_slice(&(n as u16).to_be_bytes());
            tag[50..52].copy_from_slice(&(n as u16).to_be_bytes());
        }
        let mut value=|x:f64|{
            if word==1{tag.push((x*255.).round() as u8);}
            else{tag.extend_from_slice(&((x*65535.).round() as u16).to_be_bytes());}
        };
        for ch in 0..4{for i in 0..n{
            let x=i as f64/(n-1) as f64;value(if ch==0{1.-x}else{x});
        }}
        for c in 0..2{for m in 0..2{for y in 0..2{for k in 0..2{
            value((c*m) as f64);value(y as f64);value(k as f64);
        }}}}
        for ch in 0..3{for i in 0..n{
            let x=i as f64/(n-1) as f64;value(if ch==1{1.-x}else{x});
        }}
        let mut b=vec![0;144];b[8]=2;b[12..24].copy_from_slice(b"prtrCMYKXYZ ");
        b[36..40].copy_from_slice(b"acsp");
        for (i,x) in [0.9642_f64,1.,0.8249].into_iter().enumerate(){put(&mut b,68+i*4,(x*65536.).round() as u32);}
        put(&mut b,128,1);b[132..136].copy_from_slice(b"A2B0");put(&mut b,136,144);put(&mut b,140,tag.len() as u32);
        b.extend(tag);while b.len()%4!=0{b.push(0);}
        let len=b.len() as u32;put(&mut b,0,len);b
    }
    #[test]fn legacy_lut_analytic_interpolation(){
        for word in [1,2]{
            let b=lut_profile(word);let p=Profile::parse(&b).unwrap();let l=p.cmyk_a2b0().unwrap();
            assert_eq!(l.encoding_bits(),word*8);assert_eq!(l.pcs(),*b"XYZ ");
            for i in 0..=8{for j in 0..=8{
                let c=i as f64/8.;let m=j as f64/8.;
                let actual=l.evaluate_encoded([c,m,0.375,0.625]).unwrap();
                let expected=[(1.-c)*m,0.625,0.625];
                for k in 0..3{assert!((actual[k]-expected[k]).abs()<0.0001);}
            }}
            assert_eq!(l.evaluate_encoded([-1.,2.,0.,1.]).unwrap(),[1.,1.,1.]);
            assert!(l.evaluate_encoded([0.,0.,f64::INFINITY,0.]).is_err());
            if word==2{
                assert!((l.to_xyz_d50([0.,1.,0.,1.]).unwrap()[0]-65535./32768.).abs()<1e-12);
            }else{assert!(l.to_xyz_d50([0.;4]).is_err());}
        }
    }
    #[test]fn legacy_lut_rejects_invalid_shapes(){
        let b=lut_profile(2);
        for (off,v) in [(152,3),(153,4),(154,0),(155,1),(156,1),(193,1)]{
            let mut x=b.clone();x[off]=v;
            assert!(Profile::parse(&x).unwrap().cmyk_a2b0().is_err(),"offset {off}");
        }
        let mut x=b.clone();x[154]=34;
        assert!(matches!(Profile::parse(&x).unwrap().cmyk_a2b0(),Err(Error::Limit(_))));
        let mut x=b.clone();let n=u32be(&x,140).unwrap();put(&mut x,140,n-1);
        assert!(Profile::parse(&x).unwrap().cmyk_a2b0().is_err());
        let mut x=b.clone();x[144..148].copy_from_slice(b"mAB ");
        assert!(Profile::parse(&x).unwrap().cmyk_a2b0().is_err());
        let mut x=b.clone();x[20..24].copy_from_slice(b"Lab ");
        let p=Profile::parse(&x).unwrap();let l=p.cmyk_a2b0().unwrap();
        assert!(l.evaluate_encoded([0.;4]).is_ok());assert!(l.to_xyz_d50([0.;4]).is_ok());
    }
    #[test]fn legacy_lab_decode_both_versions(){
        for word in [1,2]{for version in [2,4]{
            let mut b=lut_profile(word);b[8]=version;b[20..24].copy_from_slice(b"Lab ");
            // Constant output tables: nominal white L=100, a=b=0.
            let (start,n)={
                let p=Profile::parse(&b).unwrap();let l=p.cmyk_a2b0().unwrap();(144+l.output,l.output_entries)
            };
            for ch in 0..3{for i in 0..n{
                let p=start+word*(ch*n+i);
                if word==1{b[p]=if ch==0{255}else{128};}
                else{b[p..p+2].copy_from_slice(&(if ch==0{65280_u16}else{32768}).to_be_bytes());}
            }}
            let p=Profile::parse(&b).unwrap();let l=p.cmyk_a2b0().unwrap();
            let x=[0.25,0.5,0.75,1.];
            let lab=l.to_lab_d50(x).unwrap();
            for (a,e) in lab.into_iter().zip([100.,0.,0.]){assert!((a-e).abs()<1e-10);}
            for (a,e) in l.to_xyz_d50(x).unwrap().into_iter().zip([0.9642,1.,0.8249]){assert!((a-e).abs()<1e-10);}
            // Low-L linear branch: L=0 with neutral chroma.
            for i in 0..word*n{b[start+i]=0;}
            let p=Profile::parse(&b).unwrap();let l=p.cmyk_a2b0().unwrap();
            assert!(l.to_xyz_d50(x).unwrap().iter().all(|v|v.abs()<1e-10));
        }}
        let b=lut_profile(2);assert!(Profile::parse(&b).unwrap().cmyk_a2b0().unwrap().to_lab_d50([0.;4]).is_err());
    }
    #[test]fn parametric_types_and_branch_boundaries(){
        let make=|kind:u16,p:&[f64]|{
            let mut b=b"para\0\0\0\0".to_vec();b.extend_from_slice(&kind.to_be_bytes());b.extend_from_slice(&[0,0]);
            for &v in p{b.extend_from_slice(&((v*65536.).round()as i32).to_be_bytes());}b
        };
        let near=|a:f64,b:f64|assert!((a-b).abs()<0.0001,"{a} != {b}");
        let t=Curve::parse(&make(1,&[2.,1.,-0.25])).unwrap();
        near(t.eval(0.1),0.);near(t.eval(0.25),0.);near(t.eval(0.75),0.25);
        let t=Curve::parse(&make(2,&[2.,1.,-0.25,0.125])).unwrap();
        near(t.eval(0.1),0.125);near(t.eval(0.75),0.375);
        let t=Curve::parse(&make(3,&[2.,1.,0.,0.5,0.5])).unwrap();
        near(t.eval(0.25),0.125);near(t.eval(0.5),0.25);near(t.eval(0.75),0.5625);
        let t=Curve::parse(&make(4,&[2.,1.,0.,0.5,0.5,0.,0.])).unwrap();
        near(t.eval(0.25),0.125);near(t.eval(0.75),0.5625);
        let t=Curve::parse(&make(4,&[2.,0.5,0.,0.125,0.5,0.125,0.125])).unwrap();
        near(t.eval(0.25),0.15625);near(t.eval(0.75),0.265625);
        let t=Curve::parse(&make(3,&[2.4,1./1.055,0.055/1.055,1./12.92,0.04045])).unwrap();
        near(t.eval(0.02),0.02/12.92);near(t.eval(0.5),((0.5_f64+0.055)/1.055).powf(2.4));
        assert!(Curve::parse(&make(1,&[2.,0.,0.])).is_err());
        assert!(Curve::parse(&make(3,&[2.,1.,-0.75,0.5,0.5])).is_err());
        assert!(Curve::parse(&make(3,&[2.,1.,0.,0.1,0.5])).is_err());
        assert!(Curve::parse(&make(4,&[2.,1.,0.,-0.5,0.5,0.,0.])).is_err());
        assert!(Curve::parse(&make(4,&[2.,1.,0.,0.5,0.5])).is_err());
        assert!(Curve::parse(&make(5,&[])).is_err());
    }
    #[test]fn owned_lut_matches_borrowed_and_survives_source_drop(){
        for word in [1,2]{
            let b=lut_profile(word);
            let mut lab=b.clone();lab[20..24].copy_from_slice(b"Lab ");
            let p=Profile::parse(&lab).unwrap();let lut=p.cmyk_a2b0().unwrap();
            let owned=lut.to_owned_lut();
            for i in 0..=16{for j in 0..=16{
                let x=[i as f64/16.,j as f64/16.,0.37,0.83];
                assert_eq!(owned.to_xyz_d50(x).unwrap(),lut.to_xyz_d50(x).unwrap());
            }}
            drop(lut);drop(p);drop(lab);
            assert!(owned.to_xyz_d50([0.1,0.2,0.3,0.4]).is_ok());
            assert!(owned.to_xyz_d50([0.,0.,f64::NAN,0.]).is_err());
        }
    }
    #[test]fn mab_nonuniform_clut_and_validation(){
        let mut b=vec![0;32];b[..4].copy_from_slice(b"mAB ");b[8..10].copy_from_slice(&[4,3]);
        let put=|v:&mut[u8],i:usize,n:usize|v[i..i+4].copy_from_slice(&(n as u32).to_be_bytes());
        let identity=b"curv\0\0\0\0\0\0\0\0";
        put(&mut b,12,32);for _ in 0..3{b.extend_from_slice(identity);}
        put(&mut b,28,68);for _ in 0..4{b.extend_from_slice(identity);}
        put(&mut b,24,116);b.extend_from_slice(&[2,3,2,2,0,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0]);
        for c in 0..2{for m in 0..3{for y in 0..2{for k in 0..2{
            b.extend_from_slice(&[180-40*c-20*k,100+20*m,105+40*y]);
        }}}}
        let l=CmykMab::parse(&b,*b"Lab ").unwrap();
        let v=l.evaluate_encoded([0.5,0.25,0.5,0.5]).unwrap();
        for (a,e) in v.into_iter().zip([150.,110.,125.]){assert!((a-e/255.).abs()<1e-12);}
        let lab=l.to_lab_d50([0.,0.,0.,0.]).unwrap();assert!((lab[0]-180.*100./255.).abs()<1e-12);
        assert!(l.evaluate_encoded([f64::NAN,0.,0.,0.]).is_err());
        let xyz=CmykMab::parse(&b,*b"XYZ ").unwrap().to_xyz_d50([0.;4]).unwrap();
        assert!((xyz[0]-180./255.*65535./32768.).abs()<1e-12);
        for (pos,val) in [(12,0),(24,28),(28,69),(16,116),(12,116)]{
            let mut bad=b.clone();put(&mut bad,pos,val);assert!(CmykMab::parse(&bad,*b"Lab ").is_err());
        }
        let mut bad=b.clone();bad[116]=34;assert!(matches!(CmykMab::parse(&bad,*b"Lab "),Err(Error::Limit(_))));
        assert!(CmykMab::parse(&b[..b.len()-1],*b"Lab ").is_err());
    }
}

/// ICC rendering intent, independent of the profile's preferred intent.
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub enum RenderingIntent{Perceptual,RelativeColorimetric,Saturation,AbsoluteColorimetric}
impl RenderingIntent{
    pub fn from_pdf_name(name:&[u8])->Result<Self>{
        match name{
            b"Perceptual"=>Ok(Self::Perceptual),
            b"RelativeColorimetric"=>Ok(Self::RelativeColorimetric),
            b"Saturation"=>Ok(Self::Saturation),
            b"AbsoluteColorimetric"=>Ok(Self::AbsoluteColorimetric),
            _=>Err(Error::Unsupported("PDF rendering intent")),
        }
    }
    pub fn from_icc(value:u32)->Result<Self>{
        match value{0=>Ok(Self::Perceptual),1=>Ok(Self::RelativeColorimetric),2=>Ok(Self::Saturation),3=>Ok(Self::AbsoluteColorimetric),_=>Err(Error::Invalid("ICC rendering intent"))}
    }
    fn digit(self)->u8{match self{Self::Perceptual=>b'0',Self::RelativeColorimetric=>b'1',Self::Saturation=>b'2',Self::AbsoluteColorimetric=>b'3'}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum MpeDirection{DeviceToPcs,PcsToDevice}
impl MpeDirection{
    pub fn tag(self,intent:RenderingIntent)->Signature{
        match self{Self::DeviceToPcs=>[b'D',b'2',b'B',intent.digit()],Self::PcsToDevice=>[b'B',b'2',b'D',intent.digit()]}
    }
}
/// Ordered candidates for ICC.1:2022 section 8.10.2. Candidate presence
/// does not imply support; callers must validate each transform. A resource
/// limit must never be treated as a missing tag.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum TransformCandidate{
    Mpe{tag:Signature},
    Legacy{tag:Signature,absolute_adjustment:bool},
    MatrixTrc{absolute_adjustment:bool},
}
impl MpeDirection{
    pub fn candidates(self,intent:RenderingIntent)->Vec<TransformCandidate>{
        let absolute=intent==RenderingIntent::AbsoluteColorimetric;
        let digit=if absolute{b'1'}else{intent.digit()};
        let legacy=|d|match self{
            Self::DeviceToPcs=>[b'A',b'2',b'B',d],
            Self::PcsToDevice=>[b'B',b'2',b'A',d],
        };
        let mut out=vec![TransformCandidate::Mpe{tag:self.tag(intent)},
            TransformCandidate::Legacy{tag:legacy(digit),absolute_adjustment:absolute}];
        if digit!=b'0'{out.push(TransformCandidate::Legacy{tag:legacy(b'0'),absolute_adjustment:absolute});}
        out.push(TransformCandidate::MatrixTrc{absolute_adjustment:absolute});
        out
    }
}
impl Profile<'_>{
    /// Select the first supported candidate. Unsupported transforms may fall
    /// through; malformed data and resource limits remain explicit errors.
    /// The callback owns preparation and must apply any requested PCS adjustment.
    pub fn select_transform<T>(&self,direction:MpeDirection,intent:RenderingIntent,
        mut prepare:impl FnMut(TransformCandidate)->Result<T>)->Result<(TransformCandidate,T)>{
        for candidate in direction.candidates(intent){
            let tag=match candidate{
                TransformCandidate::Mpe{tag}|TransformCandidate::Legacy{tag,..}=>Some(tag),
                TransformCandidate::MatrixTrc{..}=>None,
            };
            if tag.is_some_and(|t|self.tag(&t).is_none()){continue;}
            match prepare(candidate){
                Ok(value)=>return Ok((candidate,value)),
                Err(Error::Unsupported(_))=>continue,
                Err(err)=>return Err(err),
            }
        }
        Err(Error::Unsupported("ICC no supported intent transform"))
    }
}

/// Exact MPE tag execution in physical PCS units (not integer LUT encoding).
#[derive(Debug)]
pub struct MpeTransform<'a>{
    pub tag:Signature,pub direction:MpeDirection,pub intent:RenderingIntent,pub pcs:Signature,
    program:mpe::Program<'a>,
}
impl MpeTransform<'_>{
    pub fn work(&self)->usize{self.program.work()}
    pub fn into_owned(self)->MpeTransform<'static>{
        MpeTransform{tag:self.tag,direction:self.direction,intent:self.intent,pcs:self.pcs,program:self.program.into_owned()}
    }

    pub fn evaluate(&self,input:&[f64])->Result<Vec<f64>>{self.program.evaluate(input)}
    pub fn evaluate_into(&self,input:&[f64],output:&mut[f64])->Result<()>{self.program.evaluate_into(input,output)}
    /// D50 PCS XYZ, retaining the selected intent's colorimetry.
    /// Absolute colorimetry is NOT scaled by the media white again.
    pub fn to_xyz_d50(&self,device:&[f64])->Result<[f64;3]>{
        if self.direction!=MpeDirection::DeviceToPcs{return Err(Error::Invalid("ICC MPE direction"));}
        let mut pcs=[0.;3];self.evaluate_into(device,&mut pcs)?;
        if self.pcs==*b"XYZ "{return Ok(pcs);}
        let [l,a,b]=pcs;
        let y=(l+16.)/116.;
        let f=|t:f64|if t>6./29.{t*t*t}else{(108./841.)*(t-4./29.)};
        let xyz=[0.9642*f(y+a/500.),f(y),0.8249*f(y-b/200.)];
        if xyz.iter().any(|v|!v.is_finite()){return Err(Error::Invalid("ICC MPE PCS range"));}
        Ok(xyz)
    }
}
#[cfg(test)]
mod mpe_intent_tests{
    use super::*;
    #[test]
    fn exact_tags_and_pdf_names(){
        for (i,name) in [b"Perceptual".as_slice(),b"RelativeColorimetric",b"Saturation",b"AbsoluteColorimetric"].iter().enumerate(){
            let intent=RenderingIntent::from_icc(i as u32).unwrap();
            assert_eq!(RenderingIntent::from_pdf_name(name).unwrap(),intent);
            assert_eq!(MpeDirection::DeviceToPcs.tag(intent),[b'D',b'2',b'B',b'0'+i as u8]);
            assert_eq!(MpeDirection::PcsToDevice.tag(intent),[b'B',b'2',b'D',b'0'+i as u8]);
        }
        assert!(RenderingIntent::from_icc(4).is_err());
        assert!(RenderingIntent::from_pdf_name(b"unknown").is_err());
    }
}

#[cfg(test)]
mod intent_selection_tests{
    use super::*;
    fn profile()->Vec<u8>{
        let mut b=vec![0;192];
        b[..4].copy_from_slice(&192u32.to_be_bytes());
        b[8]=4;b[12..24].copy_from_slice(b"prtrCMYKXYZ ");
        b[36..40].copy_from_slice(b"acsp");
        b[128..132].copy_from_slice(&3u32.to_be_bytes());
        for (i,tag) in [b"D2B1",b"A2B1",b"A2B0"].iter().enumerate(){
            let p=132+12*i;let offset=168+8*i;
            b[p..p+4].copy_from_slice(*tag);
            b[p+4..p+8].copy_from_slice(&(offset as u32).to_be_bytes());
            b[p+8..p+12].copy_from_slice(&8u32.to_be_bytes());
        }
        b
    }
    #[test]
    fn candidate_order_both_directions(){
        use TransformCandidate::*;
        for (dir,mpe,legacy,zero) in [(MpeDirection::DeviceToPcs,*b"D2B3",*b"A2B1",*b"A2B0"),
            (MpeDirection::PcsToDevice,*b"B2D3",*b"B2A1",*b"B2A0")]{
            assert_eq!(dir.candidates(RenderingIntent::AbsoluteColorimetric),vec![
                Mpe{tag:mpe},Legacy{tag:legacy,absolute_adjustment:true},
                Legacy{tag:zero,absolute_adjustment:true},MatrixTrc{absolute_adjustment:true}]);
        }
        assert_eq!(MpeDirection::DeviceToPcs.candidates(RenderingIntent::Perceptual).len(),3);
    }
    #[test]
    fn selection_fallback_and_fatal_errors(){
        use TransformCandidate::*;
        let bytes=profile();let p=Profile::parse(&bytes).unwrap();
        let dir=MpeDirection::DeviceToPcs;let intent=RenderingIntent::RelativeColorimetric;
        let (chosen,_) = p.select_transform(dir,intent,|c|match c{
            Mpe{..}=>Err(Error::Unsupported("test unsupported")),
            _=>Ok(()),
        }).unwrap();
        assert_eq!(chosen,Legacy{tag:*b"A2B1",absolute_adjustment:false});
        assert!(matches!(p.select_transform(dir,intent,|_|Err::<(),_>(Error::Limit("test"))),Err(Error::Limit("test"))));
        assert!(matches!(p.select_transform(dir,intent,|_|Err::<(),_>(Error::Invalid("test"))),Err(Error::Invalid("test"))));
        let (chosen,_) = p.select_transform(dir,RenderingIntent::Saturation,|_|Ok(())).unwrap();
        assert_eq!(chosen,Legacy{tag:*b"A2B0",absolute_adjustment:false});
    }
}

#[cfg(test)]
mod chad_tests{
    use super::*;
    #[test]fn inverse_identity_and_pivot(){
        let m=[[0.,2.,0.],[1.,0.,0.],[0.,0.,4.]];
        assert_eq!(invert_chad(m).unwrap(),[[0.,1.,0.],[0.5,0.,0.],[0.,0.,0.25]]);
    }
    #[test]fn inverse_roundtrip(){
        let m=[[1.0479,0.0229,-0.0502],[0.0296,0.9904,-0.0171],[-0.0092,0.0151,0.7519]];
        let inv=invert_chad(m).unwrap();
        for i in 0..3{for j in 0..3{
            let v=(0..3).map(|k|m[i][k]*inv[k][j]).sum::<f64>();
            assert!((v-if i==j{1.}else{0.}).abs()<1e-12);
        }}
    }
    #[test]fn rejects_bad_matrices(){
        assert!(invert_chad([[0.;3];3]).is_err());
        assert!(invert_chad([[1.,0.,0.],[1.,0.,0.],[0.,0.,1.]]).is_err());
        assert!(invert_chad([[1.,0.,0.],[0.,1e-8,0.],[0.,0.,1.]]).is_err());
        assert!(invert_chad([[f64::NAN,0.,0.],[0.,1.,0.],[0.,0.,1.]]).is_err());
    }
}
