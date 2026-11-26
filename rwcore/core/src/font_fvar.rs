//! Bounded fvar 1.0 and default (pre-avar) coordinate normalization.
//! Fixed user coordinates and intermediate normalized coordinates use
//! signed 16.16 integers. Final 2.14 conversion is a separate operation.
use crate::Error;
use std::collections::BTreeSet;

pub const VARIATION_FIXED_ONE:i32=65536;
pub const VARIATION_F2DOT14_ONE:i16=16384;
#[derive(Debug,Clone,Copy)]
pub struct FvarLimits{
    pub max_bytes:usize,pub max_axes:usize,pub max_instances:usize,
    pub max_instance_coordinates:usize,
}
impl Default for FvarLimits{
    fn default()->Self{Self{max_bytes:16*1024*1024,max_axes:256,max_instances:4096,
        max_instance_coordinates:262144}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FvarAxis{
    pub tag:[u8;4],pub min:i32,pub default:i32,pub max:i32,pub flags:u16,pub name_id:u16,
}
impl FvarAxis{
    pub fn is_hidden(&self)->bool{self.flags&1!=0}
    /// Clamp user value before division. Use widened arithmetic for
    /// the entire signed Fixed domain; collapsed halves cannot divide
    /// by zero. Round the magnitude of the 16.16 quotient to nearest,
    /// ties away from zero, then apply the sign.
    pub fn normalize_fixed(&self,user:i32)->Result<i32,Error>{
        if self.min>self.default||self.default>self.max{return Err(Error::Invalid("fvar axis interval"));}
        let v=i64::from(user.clamp(self.min,self.max));
        let d=i64::from(self.default);
        if v==d{return Ok(0);}
        let (num,den,sign)=if v<d{(d-v,d-i64::from(self.min),-1i64)}
            else{(v-d,i64::from(self.max)-d,1i64)};
        let magnitude=(num*65536+den/2)/den;
        Ok((sign*magnitude).clamp(-65536,65536) as i32)
    }
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct FvarInstance{
    pub subfamily_name_id:u16,pub coordinates:Vec<i32>,
    /// None when the field is absent or has the 0xffff sentinel.
    pub postscript_name_id:Option<u16>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Fvar{
    axes:Vec<FvarAxis>,instances:Vec<FvarInstance>,
}
impl Fvar{
    pub fn axes(&self)->&[FvarAxis]{&self.axes}
    pub fn instances(&self)->&[FvarInstance]{&self.instances}
    pub fn axis(&self,tag:[u8;4])->Option<&FvarAxis>{self.axes.iter().find(|a|a.tag==tag)}
    /// Preserve fvar axis order (not tag order). Missing coordinates
    /// in the tag-based API use defaults, unknown/duplicate tags fail.
    pub fn normalize_fixed(&self,coordinates:&[i32])->Result<Vec<i32>,Error>{
        if coordinates.len()!=self.axes.len(){return Err(Error::Invalid("fvar coordinate count"));}
        let mut out=Vec::new();reserve(&mut out,self.axes.len())?;
        for (axis,&value) in self.axes.iter().zip(coordinates){out.push(axis.normalize_fixed(value)?);}
        Ok(out)
    }
    pub fn normalize_tagged_fixed(&self,coordinates:&[([u8;4],i32)])->Result<Vec<i32>,Error>{
        if coordinates.len()>self.axes.len(){return Err(Error::Invalid("fvar tagged coordinate count"));}
        let mut user=Vec::new();reserve(&mut user,self.axes.len())?;
        user.extend(self.axes.iter().map(|a|a.default));
        let mut seen=BTreeSet::new();
        for &(tag,value) in coordinates{
            if !seen.insert(tag){return Err(Error::Invalid("fvar duplicate coordinate tag"));}
            let i=self.axes.iter().position(|a|a.tag==tag).ok_or(Error::Invalid("fvar unknown coordinate tag"))?;
            user[i]=value;
        }
        self.normalize_fixed(&user)
    }
    /// Only appropriate for fonts without an avar remapping. For avar
    /// retain 16.16 intermediates and quantize after the final mapping.
    pub fn normalize_f2dot14(&self,coordinates:&[i32])->Result<Vec<i16>,Error>{
        let fixed=self.normalize_fixed(coordinates)?;
        let mut out=Vec::new();reserve(&mut out,fixed.len())?;
        for v in fixed{out.push(normalized_fixed_to_f2dot14(v)?);}
        Ok(out)
    }
}
/// OpenType final normalized conversion: add 2 then arithmetic shift
/// right by two bits. Reject out-of-domain inputs, never wrap.
pub fn normalized_fixed_to_f2dot14(value:i32)->Result<i16,Error>{
    if !(-65536..=65536).contains(&value){return Err(Error::Invalid("normalized Fixed domain"));}
    Ok(((value+2)>>2) as i16)
}
fn slice(b:&[u8],p:usize,n:usize)->Result<&[u8],Error>{
    b.get(p..p.checked_add(n).ok_or(Error::Limit("fvar offset"))?).ok_or(Error::Invalid("fvar truncated"))
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{Ok(u16::from_be_bytes(slice(b,p,2)?.try_into().unwrap()))}
fn fixed(b:&[u8],p:usize)->Result<i32,Error>{Ok(i32::from_be_bytes(slice(b,p,4)?.try_into().unwrap()))}
fn extent(p:usize,n:usize,stride:usize)->Result<usize,Error>{
    n.checked_mul(stride).and_then(|v|p.checked_add(v)).ok_or(Error::Limit("fvar extent"))
}
fn reserve<T>(v:&mut Vec<T>,n:usize)->Result<(),Error>{
    v.try_reserve_exact(n).map_err(|_|Error::Limit("fvar allocation"))
}
fn name_id(id:u16)->bool{(256..32768).contains(&id)}
/// Strict fvar 1.0 subset: exact record sizes, reserved flags, unique
/// nonempty printable tags, ordered axis bounds, valid name ID domains,
/// in-range named coordinates. Name table contents and STAT linkage
/// are not checked. Zero axes are not a functional variable font.
pub fn parse_fvar(b:&[u8],l:FvarLimits)->Result<Fvar,Error>{
    if b.len()>l.max_bytes{return Err(Error::Limit("fvar bytes"));}
    slice(b,0,16)?;
    if word(b,0)?!=1||word(b,2)?!=0{return Err(Error::Unsupported("fvar version"));}
    let at=word(b,4)? as usize;
    let n=word(b,8)? as usize;let stride=word(b,10)? as usize;
    let count=word(b,12)? as usize;let instance_size=word(b,14)? as usize;
    if n>l.max_axes||count>l.max_instances||n.checked_mul(count).ok_or(Error::Limit("fvar coordinate count"))?>l.max_instance_coordinates{
        return Err(Error::Limit("fvar records"));
    }
    if n==0{return Err(Error::Unsupported("fvar zero axes"));}
    let base_size=extent(4,n,4)?;
    if at<16||word(b,6)?!=2||stride!=20||(instance_size!=base_size&&instance_size!=base_size+2){
        return Err(Error::Invalid("fvar header"));
    }
    let instances_at=extent(at,n,stride)?;
    let end=extent(instances_at,count,instance_size)?;
    slice(b,at,end-at)?;
    let mut axes=Vec::new();reserve(&mut axes,n)?;
    let mut tags=BTreeSet::new();
    for i in 0..n{
        let p=at+i*stride;
        let tag:[u8;4]=slice(b,p,4)?.try_into().unwrap();
        let axis=FvarAxis{tag,min:fixed(b,p+4)?,default:fixed(b,p+8)?,max:fixed(b,p+12)?,
            flags:word(b,p+16)?,name_id:word(b,p+18)?};
        if tag.iter().any(|v|!(32..=126).contains(v))||tag==*b"    "||!tags.insert(tag)
            ||axis.min>axis.default||axis.default>axis.max||axis.flags&!1!=0||!name_id(axis.name_id){
            return Err(Error::Invalid("fvar axis"));
        }
        axes.push(axis);
    }
    let mut instances=Vec::new();reserve(&mut instances,count)?;
    for i in 0..count{
        let p=instances_at+i*instance_size;
        let subfamily_name_id=word(b,p)?;
        if !(name_id(subfamily_name_id)||matches!(subfamily_name_id,2|17))||word(b,p+2)?!=0{
            return Err(Error::Invalid("fvar instance header"));
        }
        let mut coordinates=Vec::new();reserve(&mut coordinates,n)?;
        for (j,axis) in axes.iter().enumerate(){
            let v=fixed(b,p+4+4*j)?;
            if v<axis.min||v>axis.max{return Err(Error::Invalid("fvar instance coordinate"));}
            coordinates.push(v);
        }
        let postscript_name_id=if instance_size==base_size{None}else{
            match word(b,p+base_size)?{
                65535=>None,
                id if id==6||name_id(id)=>Some(id),
                _=>return Err(Error::Invalid("fvar instance PostScript name")),
            }
        };
        instances.push(FvarInstance{subfamily_name_id,coordinates,postscript_name_id});
    }
    Ok(Fvar{axes,instances})
}
