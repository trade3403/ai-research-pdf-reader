//! Bounded enumeration of direct contextual LookupList references.
//! Includes unreachable rules and duplicate actions. No transitive closure,
//! cycle rejection, matching, or execution is performed here.
use crate::{CompiledGsubLookup,ContextAction,Error,ExtensionTarget};

#[derive(Debug,Clone,Copy)]
pub struct GsubDependencyLimits{
    /// Cumulative lookup, subtable, rule-set slot (including null), and rule visits.
    pub max_scan_records:usize,
    /// Cumulative emitted action references, including duplicates.
    pub max_actions:usize,
}
impl Default for GsubDependencyLimits{
    fn default()->Self{Self{max_scan_records:1048576,max_actions:1048576}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct GsubDependencySite{
    pub lookup_index:u16,
    pub subtable_index:usize,
    pub source_offset:usize,
    pub target_offset:usize,
    pub effective_kind:u16,
    pub format:u16,
    /// Coverage index (format 1), class index (format 2), or None (format 3).
    pub set_index:Option<usize>,
    /// Zero for the single format 3 rule.
    pub rule_index:usize,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct GsubDependency{
    pub site:GsubDependencySite,
    /// Index in this rule's ordered action list.
    pub action_index:usize,
    pub action:ContextAction,
}
struct Collector{
    scan:usize,
    remaining:usize,
    lookup_count:usize,
    edges:Vec<GsubDependency>,
}
impl Collector{
    fn scan(&mut self,n:usize)->Result<(),Error>{
        self.scan=self.scan.checked_sub(n).ok_or(Error::Limit("gsub dependency scan"))?;
        Ok(())
    }
    fn actions(&mut self,site:GsubDependencySite,actions:&[ContextAction])->Result<(),Error>{
        self.scan(1)?;
        self.remaining=self.remaining.checked_sub(actions.len()).ok_or(Error::Limit("gsub dependency actions"))?;
        self.edges.try_reserve(actions.len()).map_err(|_|Error::Limit("gsub dependency allocation"))?;
        for (action_index,&action) in actions.iter().enumerate(){
            if usize::from(action.lookup_index)>=self.lookup_count{
                return Err(Error::Invalid("gsub dependency target index"));
            }
            self.edges.push(GsubDependency{site,action_index,action});
        }
        Ok(())
    }
    fn sets<R>(&mut self,site:GsubDependencySite,sets:&[Option<Vec<R>>],
        actions:impl Fn(&R)->&[ContextAction])->Result<(),Error>{
        self.scan(sets.len())?;
        for (i,set) in sets.iter().enumerate(){
            let Some(rules)=set else{continue;};
            for (j,r) in rules.iter().enumerate(){
                self.actions(GsubDependencySite{set_index:Some(i),rule_index:j,..site},actions(r))?;
            }
        }
        Ok(())
    }
}
/// The input may be a partial compiled selection. Sources must be strictly
/// ascending unique indices. Targets need only be in the full LookupList,
/// not present in the selection. The caller supplies that full count.
/// Edges preserve source/subtable/set/rule/action order and aliases.
/// Self references and mutual references are deliberately retained.
pub fn discover_gsub_dependencies(lookups:&[CompiledGsubLookup<'_>],lookup_count:usize,
    l:GsubDependencyLimits)->Result<Vec<GsubDependency>,Error>{
    if lookup_count>65535{return Err(Error::Invalid("gsub dependency lookup count"));}
    if lookups.len()>lookup_count{return Err(Error::Invalid("gsub dependency source count"));}
    let mut c=Collector{scan:l.max_scan_records,remaining:l.max_actions,lookup_count,edges:Vec::new()};
    c.scan(lookups.len())?;
    let mut previous=None;
    for lookup in lookups{
        let i=lookup.index();
        if usize::from(i)>=lookup_count||previous.is_some_and(|p|p>=i){
            return Err(Error::Invalid("gsub dependency source index"));
        }
        previous=Some(i);
        c.scan(lookup.subtables().len())?;
        for (j,sub) in lookup.subtables().iter().enumerate(){
            let site=GsubDependencySite{lookup_index:i,subtable_index:j,source_offset:sub.source_offset(),
                target_offset:sub.target_offset(),effective_kind:lookup.effective_kind(),
                format:1,set_index:None,rule_index:0};
            use ExtensionTarget::*;
            match sub.target(){
                Context(x)=>c.sets(site,x.rule_sets(),|r|&r.actions)?,
                ClassContext(x)=>c.sets(GsubDependencySite{format:2,..site},x.rule_sets(),|r|&r.actions)?,
                CoverageContext(x)=>c.actions(GsubDependencySite{format:3,..site},x.actions())?,
                Chain(x)=>c.sets(site,x.rule_sets(),|r|&r.actions)?,
                ClassChain(x)=>c.sets(GsubDependencySite{format:2,..site},x.rule_sets(),|r|&r.actions)?,
                CoverageChain(x)=>c.actions(GsubDependencySite{format:3,..site},x.actions())?,
                Single(_)|Multiple(_)|Alternate(_)|Ligature(_)|Reverse(_)=>{},
            }
        }
    }
    Ok(c.edges)
}
