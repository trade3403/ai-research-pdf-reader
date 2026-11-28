//! Bounded exact integer interval equalities over physical glyph widths.
//! Prefix potentials P define width[i] = P[i+1] - P[i].
//! Hard anchor constraints are P[end] - P[start] = width.
//! Then greedily retain original widths in deterministic direction order.
//! This is a coordinate constraint primitive, not an OpenType shaper.
use crate::{Error,GposHorizontalFlow};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct GposIntervalConstraint{
    pub start:usize,
    pub end:usize,
    pub width:i64,
}
impl GposIntervalConstraint{
    /// Convert physical anchor separation (exit - entry + placement_a -
    /// placement_b) into the logical width interval. The caller supplies
    /// actual placements or zero for the moving child.
    pub fn from_logical_pair(a:usize,b:usize,delta:i32,placement_a:i32,placement_b:i32,flow:GposHorizontalFlow)->Result<Self,Error>{
        if a>=b{return Err(Error::Invalid("gpos interval pair"));}
        let separation=i64::from(delta)+i64::from(placement_a)-i64::from(placement_b);
        match flow{
            GposHorizontalFlow::LeftToRight=>Ok(Self{start:a,end:b,width:separation}),
            GposHorizontalFlow::RightToLeft=>Ok(Self{
                start:a.checked_add(1).ok_or(Error::Limit("gpos interval index"))?,
                end:b.checked_add(1).ok_or(Error::Limit("gpos interval index"))?,width:-separation}),
        }
    }
}

#[derive(Debug,Clone,Copy)]
pub struct GposIntervalLimits{
    pub max_glyphs:usize,
    pub max_constraints:usize,
    /// Counts each union/find invocation and each parent traversal.
    pub max_work:usize,
}
impl Default for GposIntervalLimits{
    fn default()->Self{Self{max_glyphs:262144,max_constraints:1048576,max_work:16777216}}
}
fn charge(left:&mut usize)->Result<(),Error>{
    *left=left.checked_sub(1).ok_or(Error::Limit("gpos interval work"))?;Ok(())
}
fn add(a:i64,b:i64)->Result<i64,Error>{a.checked_add(b).ok_or(Error::Limit("gpos interval potential"))}
fn sub(a:i64,b:i64)->Result<i64,Error>{a.checked_sub(b).ok_or(Error::Limit("gpos interval potential"))}
#[derive(Clone,Copy)]
struct Node{parent:usize,size:usize,delta:i64}
struct Potentials{nodes:Vec<Node>,left:usize}
impl Potentials{
    fn new(n:usize,left:usize)->Result<Self,Error>{
        let mut nodes=Vec::new();
        nodes.try_reserve_exact(n).map_err(|_|Error::Limit("gpos interval allocation"))?;
        for i in 0..n{nodes.push(Node{parent:i,size:1,delta:0});}
        Ok(Self{nodes,left})
    }
    // delta = potential(node) - potential(parent). Union by size bounds
    // traversal depth logarithmically without recursion or path compression.
    fn find(&mut self,mut i:usize)->Result<(usize,i64),Error>{
        charge(&mut self.left)?;
        let mut d=0;
        while self.nodes[i].parent!=i{
            charge(&mut self.left)?;
            d=add(d,self.nodes[i].delta)?;
            i=self.nodes[i].parent;
        }
        Ok((i,d))
    }
    /// Returns false if the requested equality contradicts an existing one.
    /// A soft equality can be rejected without mutating the forest.
    fn constrain(&mut self,a:usize,b:usize,width:i64)->Result<bool,Error>{
        charge(&mut self.left)?;
        let (ra,da)=self.find(a)?;
        let (rb,db)=self.find(b)?;
        if ra==rb{return Ok(sub(db,da)?==width);}
        // P(rb)-P(ra) = width + da - db.
        let delta=sub(add(width,da)?,db)?;
        let size=self.nodes[ra].size.checked_add(self.nodes[rb].size).ok_or(Error::Limit("gpos interval size"))?;
        if self.nodes[ra].size>=self.nodes[rb].size{
            self.nodes[rb].parent=ra;self.nodes[rb].delta=delta;self.nodes[ra].size=size;
        }else{
            let reverse=sub(0,delta)?;
            self.nodes[ra].parent=rb;self.nodes[ra].delta=reverse;self.nodes[rb].size=size;
        }
        Ok(true)
    }
}
/// Solve hard interval equalities, then retain nominal widths greedily.
/// LTR considers soft widths right-to-left, RTL left-to-right, so a single
/// constrained interval changes the conventional leading/trailing owner.
/// Greedy retention is lexicographic, not a minimum-norm or nonnegative solver.
/// Signed widths are allowed. A mathematically consistent system can still
/// fail on integer range/budget limits; no caller state is modified.
/// Call independently for design and pixel units; never mix their coordinates.
pub fn solve_gpos_interval_widths(nominal:&[i32],constraints:&[GposIntervalConstraint],flow:GposHorizontalFlow,limits:GposIntervalLimits)->Result<Vec<i32>,Error>{
    let mut work=limits.max_work;
    solve_gpos_interval_widths_with_work(nominal,constraints,flow,limits,&mut work)
}
pub(crate) fn solve_gpos_interval_widths_with_work(nominal:&[i32],constraints:&[GposIntervalConstraint],flow:GposHorizontalFlow,limits:GposIntervalLimits,work:&mut usize)->Result<Vec<i32>,Error>{
    let n=nominal.len();
    if n>limits.max_glyphs||constraints.len()>limits.max_constraints{return Err(Error::Limit("gpos interval size"));}
    for c in constraints{
        if c.start>=c.end||c.end>n{return Err(Error::Invalid("gpos interval bounds"));}
    }
    let count=n.checked_add(1).ok_or(Error::Limit("gpos interval size"))?;
    let mut p=Potentials::new(count,(*work).min(limits.max_work))?;
    for c in constraints{
        if !p.constrain(c.start,c.end,c.width)?{return Err(Error::Invalid("gpos interval inconsistent"));}
    }
    for k in 0..n{
        let i=match flow{GposHorizontalFlow::LeftToRight=>n-1-k,GposHorizontalFlow::RightToLeft=>k};
        // Retain this nominal width when consistent with higher priorities.
        p.constrain(i,i+1,i64::from(nominal[i]))?;
    }
    let mut result=Vec::new();result.try_reserve_exact(n).map_err(|_|Error::Limit("gpos interval allocation"))?;
    let (mut root,mut before)=p.find(0)?;
    for i in 1..count{
        let (next,after)=p.find(i)?;
        if next!=root{return Err(Error::Invalid("gpos interval disconnected"));}
        result.push(i32::try_from(sub(after,before)?).map_err(|_|Error::Limit("gpos interval width"))?);
        root=next;before=after;
    }
    *work=p.left;
    Ok(result)
}
