//! Horizontal span integration with eight vertical rows per output pixel.
//! Coordinates are 2x device space: one coverage cell per compositor quadrant.
use super::{Budget,Coverage,Subpath,edges,Error,Result};
const ROWS:usize=4;
const MAX_CELLS:usize=4_000_000; // at most 16 MiB temporary coverage
pub(super) struct Area{x:usize,y:usize,w:usize,h:usize,data:Vec<f32>}
impl Area{
    fn empty()->Self{Self{x:0,y:0,w:0,h:0,data:Vec::new()}}
    pub(super) fn bounds(&self)->(usize,usize,usize,usize){
        (self.x/2,self.y/2,(self.x+self.w).div_ceil(2),(self.y+self.h).div_ceil(2))
    }
    pub(super) fn at(&self,x:usize,y:usize)->f64{
        if x<self.x||y<self.y||x>=self.x+self.w||y>=self.y+self.h{0.}
        else{f64::from(self.data[(y-self.y)*self.w+x-self.x]).clamp(0.,1.)}
    }
}
fn row_bound(y:f64,max:usize)->usize{(y*ROWS as f64-0.5).ceil().clamp(0.,max as f64)as usize}
pub(super) fn coverage(paths:&[Subpath],clip:&Coverage,budget:&mut Budget)->Result<Area>{
    let e=edges(paths)?;
    if e.is_empty()||clip.w==0||clip.h==0{return Ok(Area::empty());}
    budget.charge(e.len())?;
    let lo=|i:usize|e.iter().map(|(a,b)|a[i].min(b[i])).fold(f64::INFINITY,f64::min);
    let hi=|i:usize|e.iter().map(|(a,b)|a[i].max(b[i])).fold(f64::NEG_INFINITY,f64::max);
    let x0=lo(0).floor().clamp((clip.x*2)as f64,((clip.x+clip.w)*2)as f64) as usize;
    let x1=hi(0).ceil().clamp((clip.x*2)as f64,((clip.x+clip.w)*2)as f64) as usize;
    let y0=lo(1).floor().clamp((clip.y*2)as f64,((clip.y+clip.h)*2)as f64) as usize;
    let y1=hi(1).ceil().clamp((clip.y*2)as f64,((clip.y+clip.h)*2)as f64) as usize;
    if x0>=x1||y0>=y1{return Ok(Area::empty());}
    let(w,h)=(x1-x0,y1-y0);
    if w*h>MAX_CELLS{return Err(Error::Limit("glyph area coverage cells"));}
    budget.charge(w*h)?;
    let mut out=Area{x:x0,y:y0,w,h,data:vec![0.;w*h]};
    let sy0=y0*ROWS;let sy1=y1*ROWS;
    let mut events=Vec::with_capacity(e.len());
    for(a,b)in e{
        let start=row_bound(a[1].min(b[1]),sy1).max(sy0);
        let end=row_bound(a[1].max(b[1]),sy1);
        if start<end{events.push((start,end,a,b));}
    }
    let sorting=|n:usize|n.saturating_mul(1+usize::BITS as usize-n.max(1).leading_zeros()as usize);
    budget.charge(sorting(events.len()))?;
    events.sort_unstable_by_key(|e|e.0);
    let mut next=0;let mut active:Vec<usize>=Vec::new();let mut hits=Vec::new();
    for sy in sy0..sy1{
        budget.charge(active.len()+1)?;
        active.retain(|&i|events[i].1>sy);
        while next<events.len()&&events[next].0<=sy{
            if events[next].1>sy{active.push(next);}
            next+=1;
        }
        budget.charge(active.len()+sorting(active.len()))?;
        let y=(sy as f64+0.5)/ROWS as f64;
        hits.clear();
        for &i in &active{
            let(_,_,a,b)=events[i];
            hits.push((a[0]+(y-a[1])/(b[1]-a[1])*(b[0]-a[0]),if b[1]>a[1]{1}else{-1}));
        }
        hits.sort_unstable_by(|a,b|a.0.total_cmp(&b.0));
        let mut winding=0i32;let mut prev=0.;
        for &(x,delta)in &hits{
            if winding!=0{
                let l=prev.max(x0 as f64);let r=x.min(x1 as f64);
                if r>l{
                    let a=l.floor()as usize;let b=r.ceil()as usize;
                    budget.charge(b-a)?;
                    let row=(sy/ROWS-y0)*w;
                    for cell in a..b{
                        let overlap=(r.min((cell+1)as f64)-l.max(cell as f64)).max(0.);
                        out.data[row+cell-x0]+=(overlap/ROWS as f64)as f32;
                    }
                }
            }
            winding+=delta;prev=x;
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests{
    use super::*;
    fn rect(x0:f64,y0:f64,x1:f64,y1:f64,reverse:bool)->Subpath{
        let mut points=vec![[x0,y0],[x1,y0],[x1,y1],[x0,y1]];
        if reverse{points.reverse();}
        Subpath{points,closed:true}
    }
    #[test]fn thin_span_and_hole(){
        let clip=Coverage{x:0,y:0,w:4,h:4,data:vec![]};
        let mut b=Budget::default();
        let a=coverage(&[rect(0.01,0.,0.11,1.,false)],&clip,&mut b).unwrap();
        assert!((a.at(0,0)-0.1).abs()<1e-6);
        let a=coverage(&[rect(0.,0.,4.,4.,false),rect(1.,1.,3.,3.,true)],&clip,&mut b).unwrap();
        assert_eq!(a.at(0,0),1.);assert_eq!(a.at(1,1),0.);assert_eq!(a.at(3,3),1.);
    }
    #[test]fn off_clip_winding_and_empty(){
        let clip=Coverage{x:2,y:2,w:1,h:1,data:vec![]};
        let mut b=Budget::default();
        let a=coverage(&[rect(-100.,-100.,100.,100.,false)],&clip,&mut b).unwrap();
        assert_eq!(a.bounds(),(2,2,3,3));assert_eq!(a.at(4,4),1.);assert_eq!(a.at(3,4),0.);
        let a=coverage(&[rect(0.,0.,1.,1.,false)],&clip,&mut b).unwrap();
        assert_eq!(a.at(4,4),0.);
    }
}
