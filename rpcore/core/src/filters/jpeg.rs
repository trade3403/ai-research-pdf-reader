//! Independent 8-bit sequential and progressive JPEG decoder.
use crate::{Error,Result};
const MAX_PIXELS:usize=4_000_000;
const MAX_BLOCKS:usize=250_000;
const ZIG:[usize;64]=[0,1,8,16,9,2,3,10,17,24,32,25,18,11,4,5,12,19,26,33,40,48,41,34,27,20,13,6,7,14,21,28,35,42,49,56,57,50,43,36,29,22,15,23,30,37,44,51,58,59,52,45,38,31,39,46,53,60,61,54,47,55,62,63];
#[derive(Debug)]
pub struct Image{pub width:usize,pub height:usize,pub channels:usize,pub data:Vec<u8>}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Transform{Auto,None,YCbCr}
#[derive(Clone)]
struct Huff{codes:Vec<(u8,u16,u8)>}
impl Huff{
    fn new(count:&[u8],symbols:&[u8])->Result<Self>{
        let mut code=0u32;let mut i=0;let mut codes=Vec::new();
        for (j,&n) in count.iter().enumerate(){
            let len=j+1;
            for _ in 0..n{
                if code>=(1u32<<len)-1{return Err(Error::Invalid("JPEG Huffman code"));}
                codes.push((len as u8,code as u16,symbols[i]));i+=1;code+=1;
            }
            code<<=1;
        }
        if codes.is_empty(){return Err(Error::Invalid("empty JPEG Huffman table"));}
        Ok(Self{codes})
    }
    fn symbol(&self,b:&mut Bits<'_>)->Result<u8>{
        let mut code=0u16;
        for n in 1..=16{code=(code<<1)|u16::from(b.bit()?);if let Some(&(_,_,s))=self.codes.iter().find(|&&(l,c,_)|l==n&&c==code){return Ok(s);}}
        Err(Error::Invalid("JPEG Huffman symbol"))
    }
}
struct Bits<'a>{d:&'a[u8],p:usize,byte:u8,left:u8}
impl Bits<'_>{
    fn bit(&mut self)->Result<u8>{
        if self.left==0{
            let b=*self.d.get(self.p).ok_or(Error::Invalid("truncated JPEG entropy"))?;self.p+=1;
            if b==255{if self.d.get(self.p)!=Some(&0){return Err(Error::Invalid("unexpected JPEG entropy marker"));}self.p+=1;}
            self.byte=b;self.left=8;
        }
        self.left-=1;Ok((self.byte>>self.left)&1)
    }
    fn signed(&mut self,n:u8)->Result<i32>{
        if n>11{return Err(Error::Invalid("JPEG coefficient category"));}
        if n==0{return Ok(0);}
        let mut v=0i32;for _ in 0..n{v=(v<<1)|i32::from(self.bit()?);}
        if v<(1<<(n-1)){v-= (1<<n)-1;}Ok(v)
    }
    fn marker(&mut self,want:u8)->Result<()>{
        if self.left>0&&self.byte&((1u8<<self.left)-1)!=((1u8<<self.left)-1){return Err(Error::Invalid("JPEG entropy padding"));}
        self.left=0;
        if self.d.get(self.p)!=Some(&255){return Err(Error::Invalid("JPEG marker expected"));}
        while self.d.get(self.p)==Some(&255){self.p+=1;}
        if self.d.get(self.p)!=Some(&want){return Err(Error::Invalid("JPEG marker mismatch"));}
        self.p+=1;Ok(())
    }
}
#[derive(Clone)]
struct Component{id:u8,h:usize,v:usize,q:usize,dc:usize,ac:usize,pred:i32,plane:Vec<u8>,stride:usize,coeff:Vec<[i32;64]>,approx:[i8;64]}
fn be(d:&[u8],p:usize)->Result<usize>{let a=d.get(p..p+2).ok_or(Error::Invalid("JPEG segment integer"))?;Ok(usize::from(u16::from_be_bytes([a[0],a[1]])))}
fn block(bits:&mut Bits<'_>,dc:&Huff,ac:&Huff,q:&[u16;64],pred:&mut i32,basis:&[[f64;8];8])->Result<[u8;64]>{
    let category=dc.symbol(bits)?;*pred=pred.checked_add(bits.signed(category)?).ok_or(Error::Limit("JPEG DC"))?;
    if pred.abs()>32767{return Err(Error::Invalid("JPEG DC range"));}
    let mut coeff=[0i32;64];coeff[0]=*pred*i32::from(q[0]);let mut k=1;
    while k<64{
        let s=ac.symbol(bits)?;if s==0{break;}
        if s==0xf0{k+=16;if k>64{return Err(Error::Invalid("JPEG zero run"));}continue;}
        let size=s&15;if size==0||size>10{return Err(Error::Invalid("JPEG AC category"));}
        k+=usize::from(s>>4);if k>=64{return Err(Error::Invalid("JPEG AC run"));}
        coeff[ZIG[k]]=bits.signed(size)?*i32::from(q[ZIG[k]]);k+=1;
    }
    Ok(idct(&coeff,basis))
}
fn idct(coeff:&[i32;64],basis:&[[f64;8];8])->[u8;64]{
    if coeff[1..].iter().all(|&x|x==0){return [(128.+f64::from(coeff[0])/8.).round().clamp(0.,255.) as u8;64];}
    let mut tmp=[[0.;8];8];let mut out=[0;64];
    for v in 0..8{for x in 0..8{tmp[v][x]=(0..8).map(|u|f64::from(coeff[v*8+u])*basis[x][u]).sum();}}
    for y in 0..8{for x in 0..8{let v=(0..8).map(|k|tmp[k][x]*basis[y][k]).sum::<f64>();out[y*8+x]=(v+128.).round().clamp(0.,255.) as u8;}}
    out
}
fn unsigned(bits:&mut Bits<'_>,n:u8)->Result<usize>{let mut v=0;for _ in 0..n{v=(v<<1)|usize::from(bits.bit()?);}Ok(v)}
fn refine(bits:&mut Bits<'_>,v:&mut i32,p:i32)->Result<()>{
    if bits.bit()?!=0&&(*v&p)==0{*v+=if *v>0{p}else{-p};}Ok(())
}
#[derive(Clone,Copy)]
struct Scan{ss:usize,se:usize,ah:u8,al:u8}
fn progressive_block(bits:&mut Bits<'_>,dc:Option<&Huff>,ac:Option<&Huff>,pred:&mut i32,c:&mut [i32;64],s:Scan,eob:&mut usize)->Result<()>{
    let p=1i32<<s.al;
    if s.ss==0{
        if s.ah==0{
            let cat=dc.ok_or(Error::Invalid("missing progressive DC table"))?.symbol(bits)?;
            *pred=pred.checked_add(bits.signed(cat)?).ok_or(Error::Limit("progressive DC"))?;
            let v=pred.checked_mul(p).ok_or(Error::Limit("progressive DC"))?;
            if v.abs()>32767{return Err(Error::Invalid("progressive DC range"));}c[0]=v;
        }else if bits.bit()?!=0{c[0]|=p;}
        return Ok(());
    }
    let ac=ac.ok_or(Error::Invalid("missing progressive AC table"))?;
    let mut k=s.ss;
    if s.ah==0{
        if *eob>0{*eob-=1;return Ok(());}
        while k<=s.se{
            let v=ac.symbol(bits)?;let run=v>>4;let size=v&15;
            if size==0{
                if run==15{k+=16;if k>s.se+1{return Err(Error::Invalid("progressive ZRL"));}continue;}
                *eob=(1usize<<run)+unsigned(bits,run)?-1;break;
            }
            if size>10{return Err(Error::Invalid("progressive AC category"));}
            k+=usize::from(run);if k>s.se{return Err(Error::Invalid("progressive AC run"));}
            let value=bits.signed(size)?*p;if value.abs()>32767{return Err(Error::Invalid("progressive AC range"));}c[ZIG[k]]=value;k+=1;
        }
    }else{
        if *eob==0{
            while k<=s.se{
                let v=ac.symbol(bits)?;let run=v>>4;let size=v&15;
                if size==0&&run!=15{*eob=(1usize<<run)+unsigned(bits,run)?;break;}
                if size>1{return Err(Error::Invalid("progressive AC refine category"));}
                let new=if size==1{if bits.bit()?==1{p}else{-p}}else{0};
                let mut zeros=if size==0{16}else{usize::from(run)};
                let mut placed=false;
                while k<=s.se{
                    let at=&mut c[ZIG[k]];
                    if *at!=0{refine(bits,at,p)?;k+=1;continue;}
                    if zeros>0{zeros-=1;k+=1;if size==0&&zeros==0{placed=true;break;}continue;}
                    if size==1{*at=new;k+=1;placed=true;break;}
                }
                if !placed{return Err(Error::Invalid("progressive refine run"));}
            }
        }
        if *eob>0{
            while k<=s.se{let at=&mut c[ZIG[k]];if *at!=0{refine(bits,at,p)?;}k+=1;}
            *eob-=1;
        }
    }
    Ok(())
}
fn ycc(a:u8,b:u8,c:u8)->[u8;3]{
    let(y,cb,cr)=(f64::from(a),f64::from(b)-128.,f64::from(c)-128.);
    [y+1.402*cr,y-0.344136*cb-0.714136*cr,y+1.772*cb].map(|v|v.round().clamp(0.,255.)as u8)
}
pub fn decode(data:&[u8],limit:usize,transform:Transform)->Result<Image>{
    if data.len()>crate::MAX_BYTES{return Err(Error::Limit("JPEG bytes"));}
    if !data.starts_with(&[255,216]){return Err(Error::Invalid("JPEG SOI"));}
    let mut p=2;let mut quant:[Option<[u16;64]>;4]=[None;4];let mut dc:[Option<Huff>;4]=std::array::from_fn(|_|None);let mut ac=dc.clone();
    let(mut width,mut height,mut restart)=(0usize,0usize,0usize);let mut comps=Vec::<Component>::new();let mut adobe=None;let mut jfif=false;
    let mut progressive=false;let mut scans=0usize;let mut visits=0usize;
    let mut geometry=None;
    let basis=std::array::from_fn(|x|std::array::from_fn(|u|{let k=if u==0{std::f64::consts::FRAC_1_SQRT_2}else{1.};0.5*k*(((2*x+1)*u)as f64*std::f64::consts::PI/16.).cos()}));
    loop{
        if data.get(p)!=Some(&255){return Err(Error::Invalid("JPEG marker"));}while data.get(p)==Some(&255){p+=1;}
        let m=*data.get(p).ok_or(Error::Invalid("truncated JPEG marker"))?;p+=1;
        if m==0xd9{if scans==0||p!=data.len(){return Err(Error::Invalid("JPEG EOI"));}break;}
        if matches!(m,0|1|0xd0..=0xd8){return Err(Error::Invalid("unexpected JPEG marker"));}
        let n=be(data,p)?;if n<2{return Err(Error::Invalid("JPEG segment length"));}
        let end=p.checked_add(n).ok_or(Error::Limit("JPEG segment"))?;let s=data.get(p+2..end).ok_or(Error::Invalid("truncated JPEG segment"))?;p=end;
        match m{
            0xdb=>{if scans>0{return Err(Error::Unsupported("JPEG quant table redefinition"));}let mut i=0;while i<s.len(){let t=s[i];i+=1;if t>>4!=0||t&15>3{return Err(Error::Unsupported("JPEG quant precision/table"));}let a=s.get(i..i+64).ok_or(Error::Invalid("JPEG quant table"))?;i+=64;let mut q=[0u16;64];for(k,&v)in a.iter().enumerate(){if v==0{return Err(Error::Invalid("JPEG zero quantizer"));}q[ZIG[k]]=u16::from(v);}quant[(t&15)as usize]=Some(q);}}
            0xc4=>{let mut i=0;while i<s.len(){let t=s[i];i+=1;if t>>4>1||t&15>3{return Err(Error::Invalid("JPEG Huffman table ID"));}let counts=s.get(i..i+16).ok_or(Error::Invalid("JPEG Huffman counts"))?;i+=16;let n=counts.iter().map(|&v|usize::from(v)).sum::<usize>();if n>256{return Err(Error::Invalid("JPEG Huffman count"));}let syms=s.get(i..i+n).ok_or(Error::Invalid("JPEG Huffman symbols"))?;i+=n;let h=Huff::new(counts,syms)?;if t>>4==0{dc[(t&15)as usize]=Some(h);}else{ac[(t&15)as usize]=Some(h);}}}
            0xc0|0xc2=>{
                progressive=m==0xc2;
                if !comps.is_empty()||s.len()<6||s[0]!=8{return Err(Error::Unsupported("JPEG frame"));}
                height=be(s,1)?;width=be(s,3)?;let n=s[5]as usize;
                if width==0||height==0||width.checked_mul(height).is_none_or(|n|n>MAX_PIXELS)||!matches!(n,1|3|4)||s.len()!=6+3*n{return Err(Error::Unsupported("JPEG frame dimensions/components"));}
                if width*height*n>limit{return Err(Error::Limit("JPEG output"));}
                for a in s[6..].chunks_exact(3){let(h,v,q)=((a[1]>>4)as usize,(a[1]&15)as usize,a[2]as usize);if !(1..=4).contains(&h)||!(1..=4).contains(&v)||q>3||comps.iter().any(|c|c.id==a[0]){return Err(Error::Invalid("JPEG component"));}comps.push(Component{id:a[0],h,v,q,dc:0,ac:0,pred:0,plane:Vec::new(),stride:0,coeff:Vec::new(),approx:[-1;64]});}
            }
            0xdd=>{if s.len()!=2{return Err(Error::Invalid("JPEG DRI"));}restart=be(s,0)?;}
            0xe0=>{if s.starts_with(b"JFIF\0"){jfif=true;}}
            0xee=>{if s.starts_with(b"Adobe")&&s.len()>=12{adobe=Some(s[11]);}}
            0xda=>{
                if comps.is_empty()||s.is_empty(){return Err(Error::Invalid("JPEG SOS"));}
                scans+=1;if scans>128{return Err(Error::Limit("JPEG scans"));}
                let n=s[0]as usize;
                if n==0||n>comps.len()||s.len()!=1+2*n+3{return Err(Error::Invalid("JPEG scan components"));}
                let ss=s[1+2*n]as usize;let se=s[2+2*n]as usize;let ah=s[3+2*n]>>4;let al=s[3+2*n]&15;
                if progressive{
                    if ss>se||se>63||ah>13||al>13||(ah!=0&&ah!=al+1)||(ss==0&&se!=0)||(ss>0&&n!=1){return Err(Error::Invalid("progressive scan parameters"));}
                }else if (ss,se,ah,al)!=(0,63,0,0)||n!=comps.len()||scans!=1{return Err(Error::Unsupported("JPEG sequential multiscan"));}
                let mut order=Vec::new();
                for a in s[1..1+2*n].chunks_exact(2){
                    let i=comps.iter().position(|c|c.id==a[0]).ok_or(Error::Invalid("JPEG scan component"))?;
                    if order.contains(&i)||a[1]>>4>3||a[1]&15>3{return Err(Error::Invalid("JPEG scan table"));}
                    order.push(i);comps[i].dc=(a[1]>>4)as usize;comps[i].ac=(a[1]&15)as usize;
                    if progressive{
                        if ss>0&&comps[i].approx[0]<0{return Err(Error::Invalid("progressive AC before DC"));}
                        for k in ss..=se{let prev=comps[i].approx[k];if (ah==0&&prev!=-1)||(ah!=0&&prev!=ah as i8){return Err(Error::Invalid("progressive scan order"));}comps[i].approx[k]=al as i8;}
                    }
                }
                if geometry.is_none(){
                    let maxh=comps.iter().map(|c|c.h).max().unwrap();let maxv=comps.iter().map(|c|c.v).max().unwrap();
                    if comps.len()==1&&(maxh!=1||maxv!=1){return Err(Error::Unsupported("JPEG grayscale sampling"));}
                    if comps.iter().map(|c|c.h*c.v).sum::<usize>()>10||comps.iter().any(|c|maxh%c.h!=0||maxv%c.v!=0){return Err(Error::Unsupported("JPEG sampling factors"));}
                    let mx=width.div_ceil(8*maxh);let my=height.div_ceil(8*maxv);
                    let blocks=mx*my*comps.iter().map(|c|c.h*c.v).sum::<usize>();if blocks>MAX_BLOCKS{return Err(Error::Limit("JPEG blocks"));}
                    for c in &mut comps{c.stride=mx*c.h*8;c.plane=vec![0;c.stride*my*c.v*8];if progressive{c.coeff=vec![[0;64];mx*c.h*my*c.v];}}
                    geometry=Some((maxh,maxv,mx,my));
                }
                let(maxh,maxv,mx,my)=geometry.unwrap();
                let (nx,ny)=if n==1&&progressive{let c=&comps[order[0]];((width*c.h).div_ceil(8*maxh),(height*c.v).div_ceil(8*maxv))}else{(mx,my)};
                let mut bits=Bits{d:data,p,byte:0,left:0};let mut rst=0u8;let mut eob=0usize;
                for c in &mut comps{c.pred=0;}
                let spec=Scan{ss,se,ah,al};
                for y in 0..ny{for x in 0..nx{
                    let mcu=y*nx+x;
                    if restart>0&&mcu>0&&mcu%restart==0{
                        if eob!=0{return Err(Error::Invalid("JPEG EOB across restart"));}
                        bits.marker(0xd0+rst)?;rst=(rst+1)%8;for c in &mut comps{c.pred=0;}
                    }
                    for &i in &order{
                        let c=&mut comps[i];
                        let q=quant[c.q].as_ref().ok_or(Error::Invalid("JPEG missing quant table"))?;
                        let dt=dc[c.dc].as_ref();let at=ac[c.ac].as_ref();
                        let (hh,vv)=if n==1&&progressive{(1,1)}else{(c.h,c.v)};
                        for by in 0..vv{for bx in 0..hh{
                            visits+=1;if visits>2_000_000{return Err(Error::Limit("JPEG scan work"));}
                            let px=x*hh+bx;let py=y*vv+by;
                            if progressive{
                                let ci=py*(c.stride/8)+px;
                                progressive_block(&mut bits,dt,at,&mut c.pred,&mut c.coeff[ci],spec,&mut eob)?;
                            }else{
                                let b=block(&mut bits,dt.ok_or(Error::Invalid("JPEG missing DC table"))?,at.ok_or(Error::Invalid("JPEG missing AC table"))?,q,&mut c.pred,&basis)?;
                                for row in 0..8{let pos=(py*8+row)*c.stride+px*8;c.plane[pos..pos+8].copy_from_slice(&b[row*8..row*8+8]);}
                            }
                        }}
                    }
                }}
                if eob!=0{return Err(Error::Invalid("JPEG EOB past scan"));}
                if bits.left>0&&bits.byte&((1u8<<bits.left)-1)!=((1u8<<bits.left)-1){return Err(Error::Invalid("JPEG entropy padding"));}
                p=bits.p;
            }
            0xe1..=0xed|0xef|0xfe=>{},
            _=>return Err(Error::Unsupported("JPEG marker/frame type")),
        }
    }
    let(maxh,maxv,_,_)=geometry.ok_or(Error::Invalid("JPEG missing frame"))?;
    if progressive{
        for c in &mut comps{
            if c.approx[0]<0{return Err(Error::Invalid("JPEG missing DC scan"));}
            let q=quant[c.q].as_ref().ok_or(Error::Invalid("JPEG missing quant table"))?;
            for (i,coeff) in c.coeff.iter().enumerate(){
                let dequant=std::array::from_fn(|k|coeff[k]*i32::from(q[k]));let b=idct(&dequant,&basis);
                let px=i%(c.stride/8)*8;let py=i/(c.stride/8)*8;
                for row in 0..8{let at=(py+row)*c.stride+px;c.plane[at..at+8].copy_from_slice(&b[row*8..row*8+8]);}
            }
            c.coeff.clear();
        }
    }
    let channels=comps.len();
    let converted=match channels{
        1=>false,
        3=>match transform{Transform::None=>false,Transform::YCbCr=>true,Transform::Auto=>match adobe{Some(0)=>false,Some(1)=>true,Some(_)=>return Err(Error::Unsupported("JPEG Adobe transform")),None=>jfif||comps.iter().map(|c|c.id).collect::<Vec<_>>()!=b"RGB"}},
        4=>match transform{Transform::None=>false,Transform::YCbCr=>true,Transform::Auto=>match adobe{None|Some(0)=>false,Some(2)=>true,_=>return Err(Error::Unsupported("JPEG Adobe four-component transform"))}},
        _=>unreachable!(),
    };
    let mut out=Vec::with_capacity(width*height*channels);
    for y in 0..height{for x in 0..width{
        let val=|c:&Component|c.plane[(y*c.v/maxv)*c.stride+x*c.h/maxh];
        if channels==1{out.push(val(&comps[0]));}else{
            let [a,b,c]=[val(&comps[0]),val(&comps[1]),val(&comps[2])];
            if converted{let rgb=ycc(a,b,c);if channels==4{out.extend(rgb.map(|v|255-v));}else{out.extend(rgb);}}else{out.extend([a,b,c]);}
            if channels==4{out.push(val(&comps[3]));}
        }
    }}
    Ok(Image{width,height,channels,data:out})
}
