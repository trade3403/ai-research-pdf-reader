//! Static transitive dependency analysis, not a runtime execution policy.
//! Missing source coverage is explicit. Iterative DFS avoids recursion.
use crate::{Error,GsubDependency};

#[derive(Debug,Clone,Copy)]
pub struct GsubClosureLimits{
    /// Bounds full LookupList domain, including unused slots.
    pub max_lookup_count:usize,
    /// Raw roots including duplicates.
    pub max_roots:usize,
    /// Raw action references including duplicates, reachable or not.
    pub max_actions:usize,
    /// Reachable vertices including roots and missing sources.
    pub max_reachable:usize,
    /// Maximum active DFS stack, not longest path in a general graph.
    pub max_stack_depth:usize,
}
impl Default for GsubClosureLimits{
    fn default()->Self{Self{max_lookup_count:65535,max_roots:65535,max_actions:1048576,
        max_reachable:65535,max_stack_depth:65535}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GsubDependencyClosure{
    /// Sorted unique roots and all transitively referenced vertices.
    pub reachable:Vec<u16>,
    /// Sorted reachable indices without enumerated source coverage.
    pub missing_sources:Vec<u16>,
    /// First deterministic DFS back-edge witness, closed by repeated
    /// start vertex. None means no cycle found in the known reachable graph.
    pub cycle_witness:Option<Vec<u16>>,
    /// Distinct known directed edges leaving reachable vertices.
    pub reachable_edges:usize,
}
impl GsubDependencyClosure{
    pub fn is_complete(&self)->bool{self.missing_sources.is_empty()}
    pub fn is_proven_acyclic(&self)->bool{self.is_complete()&&self.cycle_witness.is_none()}
}
fn index(id:u16,n:usize)->Result<usize,Error>{
    let i=usize::from(id);
    if i>=n{return Err(Error::Invalid("gsub closure lookup index"));}
    Ok(i)
}
/// Sources must be the complete list of indices for which direct action
/// enumeration was performed, including lookups with zero outgoing actions.
/// Sources strictly ascend and must be unique. Every supplied edge must
/// originate in a covered source. Caller is responsible for supplying all
/// edges from those sources and keeping inputs from the same font.
/// Roots are sorted and deduplicated; adjacency is sorted and deduplicated
/// only for graph analysis, not as a replacement for ordered action lists.
/// Missing reachable sources remain in the closure but have unknown outgoing
/// edges. A witness is diagnostic, not a reason to reject a font outright.
pub fn analyze_gsub_dependency_closure(lookup_count:usize,sources:&[u16],
    edges:&[GsubDependency],roots:&[u16],l:GsubClosureLimits)->Result<GsubDependencyClosure,Error>{
    if lookup_count>65535{return Err(Error::Invalid("gsub closure lookup count"));}
    if lookup_count>l.max_lookup_count||roots.len()>l.max_roots||edges.len()>l.max_actions{
        return Err(Error::Limit("gsub closure input"));
    }
    if sources.len()>lookup_count||sources.windows(2).any(|w|w[0]>=w[1]){
        return Err(Error::Invalid("gsub closure source order"));
    }
    let mut covered=vec![false;lookup_count];
    for &id in sources{covered[index(id,lookup_count)?]=true;}
    let mut adj=vec![Vec::<usize>::new();lookup_count];
    for edge in edges{
        let from=index(edge.site.lookup_index,lookup_count)?;
        let to=index(edge.action.lookup_index,lookup_count)?;
        if !covered[from]{return Err(Error::Invalid("gsub closure uncovered edge source"));}
        adj[from].try_reserve(1).map_err(|_|Error::Limit("gsub closure allocation"))?;
        adj[from].push(to);
    }
    for list in &mut adj{list.sort_unstable();list.dedup();}
    let mut roots=roots.iter().map(|&id|index(id,lookup_count)).collect::<Result<Vec<_>,_>>()?;
    roots.sort_unstable();roots.dedup();
    // White/gray/black; gray vertices are exactly the active stack.
    let mut color=vec![0u8;lookup_count];
    let mut stack:Vec<(usize,usize)>=Vec::new();
    let mut reached=0usize;
    let mut reachable_edges=0usize;
    let mut cycle_witness=None;
    let mut enter=|v:usize,color:&mut [u8],stack:&mut Vec<(usize,usize)>|->Result<(),Error>{
        if reached>=l.max_reachable||stack.len()>=l.max_stack_depth{
            return Err(Error::Limit("gsub closure traversal"));
        }
        reached+=1;
        reachable_edges=reachable_edges.checked_add(adj[v].len()).ok_or(Error::Limit("gsub closure edge count"))?;
        color[v]=1;
        stack.push((v,0));
        Ok(())
    };
    for root in roots{
        if color[root]!=0{continue;}
        enter(root,&mut color,&mut stack)?;
        while let Some(&(v,next))=stack.last(){
            if next==adj[v].len(){
                color[v]=2;
                stack.pop();
                continue;
            }
            let target=adj[v][next];
            stack.last_mut().unwrap().1+=1;
            match color[target]{
                0=>enter(target,&mut color,&mut stack)?,
                1 if cycle_witness.is_none()=>{
                    let start=stack.iter().position(|&(id,_)|id==target)
                        .ok_or(Error::Invalid("gsub closure active vertex"))?;
                    let mut witness:Vec<u16>=stack[start..].iter().map(|&(id,_)|id as u16).collect();
                    witness.push(target as u16);
                    cycle_witness=Some(witness);
                }
                _=>{}
            }
        }
    }
    let reachable:Vec<u16>=color.iter().enumerate().filter_map(|(i,&c)|(c!=0).then_some(i as u16)).collect();
    let missing_sources=reachable.iter().copied().filter(|&id|!covered[id as usize]).collect();
    Ok(GsubDependencyClosure{reachable,missing_sources,cycle_witness,reachable_edges})
}
