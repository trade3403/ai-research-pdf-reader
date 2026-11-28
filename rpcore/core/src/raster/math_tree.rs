//! Bounded, conservative formula candidate tree and allowlisted LaTeX output.
use super::{PageGlyphGeometry,math_geometry::{MathRegion,ScriptKind}};
use crate::{Error,Result};
use std::collections::{BTreeMap,BTreeSet};

const MAX_GLYPHS:usize=512;
const MAX_DEPTH:usize=16;
const MAX_LATEX:usize=32*1024;

#[derive(Debug,Clone,PartialEq)]
pub enum MathNode{
    Glyph{index:usize},
    Row(Vec<MathNode>),
    Scripts{base:Box<MathNode>,sub:Option<Box<MathNode>>,sup:Option<Box<MathNode>>},
    Fraction{rule_command_index:usize,numerator:Box<MathNode>,denominator:Box<MathNode>},
    Radical{symbol:usize,rule_command_index:usize,radicand:Box<MathNode>},
}
#[derive(Debug,Clone,PartialEq)]
pub struct FormulaTree{pub root:MathNode}
type RowId=Option<(usize,bool)>;
type Children=BTreeMap<usize,(Vec<usize>,Vec<usize>)>;

fn glyph_node(i:usize,children:&Children,depth:usize)->Result<MathNode>{
    if depth>MAX_DEPTH{return Err(Error::Limit("math tree depth"));}
    let base=MathNode::Glyph{index:i};
    let Some((sub,sup))=children.get(&i)else{return Ok(base);};
    let child=|v:&[usize]|->Result<Option<Box<MathNode>>>{
        if v.is_empty(){return Ok(None);}
        if v.len()==1{return Ok(Some(Box::new(glyph_node(v[0],children,depth+1)?)));}
        let nodes=v.iter().map(|&j|glyph_node(j,children,depth+1)).collect::<Result<Vec<_>>>()?;
        Ok(Some(Box::new(MathNode::Row(nodes))))
    };
    Ok(MathNode::Scripts{base:Box::new(base),sub:child(sub)?,sup:child(sup)?})
}
fn row_nodes(ids:&[usize],g:&PageGlyphGeometry,parents:&BTreeSet<usize>,children:&Children)->Result<Vec<(f64,MathNode)>>{
    let mut roots:Vec<usize>=ids.iter().copied().filter(|i|!parents.contains(i)).collect();
    roots.sort_by(|&a,&b|g.glyphs[a].origin[0].total_cmp(&g.glyphs[b].origin[0]).then(a.cmp(&b)));
    // A row must share an approximate baseline and proceed horizontally.
    // Reject rather than flatten stacked or coincident unclassified glyphs.
    if let Some(&first)=roots.first(){
        let f=&g.glyphs[first];
        let mut previous=None;
        for &i in &roots{
            let a=&g.glyphs[i];
            let em=a.em_matrix.0[3].min(f.em_matrix.0[3]);
            if (a.origin[1]-f.origin[1]).abs()>em*0.3{return Err(Error::Unsupported("ambiguous math row baseline"));}
            if let Some(x)=previous{if a.origin[0]<=x{return Err(Error::Unsupported("overlapping math row origins"));}}
            previous=Some(a.origin[0]);
        }
    }
    roots.into_iter().map(|i|Ok((g.glyphs[i].origin[0],glyph_node(i,children,0)?))).collect()
}
fn sorted_row(mut entries:Vec<(f64,MathNode)>)->MathNode{
    entries.sort_by(|a,b|a.0.total_cmp(&b.0));
    MathNode::Row(entries.into_iter().map(|(_,n)|n).collect())
}
/// Convert local candidate relationships to a tree. Whole-side nested fractions
/// and nested scripts are supported, as is a radical occupying a complete
/// side of a non-nested fraction. Ambiguous ownership and incompatible rows are rejected.
pub fn build_tree(g:&PageGlyphGeometry,m:&MathRegion)->Result<FormulaTree>{
    if m.glyph_indices.is_empty(){return Err(Error::Invalid("empty formula tree"));}
    if m.glyph_indices.len()>MAX_GLYPHS||m.scripts.len()>MAX_GLYPHS||m.fractions.len()>256||m.radicals.len()>256{return Err(Error::Limit("math tree inputs"));}
    let selected:BTreeSet<usize>=m.glyph_indices.iter().copied().collect();
    if selected.len()!=m.glyph_indices.len(){return Err(Error::Invalid("duplicate formula glyph"));}
    for &i in &selected{
        let a=g.glyphs.get(i).ok_or(Error::Invalid("formula glyph index"))?;
        let [x,b,c,y,_,_]=a.em_matrix.0;
        if a.outline_bounds.is_none()||!x.is_finite()||!y.is_finite()||x<=0.||y<=0.||b!=0.||c!=0.||
            a.origin.iter().any(|v|!v.is_finite()){return Err(Error::Unsupported("non-upright formula glyph"));}
    }
    let mut owner:BTreeMap<usize,RowId>=selected.iter().map(|&i|(i,None)).collect();
    let mut rules=BTreeSet::new();
    let mut by_rule=BTreeMap::new();
    for (fi,f) in m.fractions.iter().enumerate(){
        if !rules.insert(f.rule_command_index)||f.numerator.is_empty()||f.denominator.is_empty(){
            return Err(Error::Invalid("invalid formula fraction"));
        }
        if f.rule_bounds.iter().any(|v|!v.is_finite())||f.rule_bounds[0]>=f.rule_bounds[2]||f.rule_bounds[1]>=f.rule_bounds[3]{
            return Err(Error::Invalid("formula rule bounds"));
        }
        by_rule.insert(f.rule_command_index,fi);
        let mut seen=BTreeSet::new();
        for ids in [&f.numerator,&f.denominator]{
            if ids.len()>MAX_GLYPHS{return Err(Error::Limit("formula row glyphs"));}
            for &i in ids{
                if !selected.contains(&i)||!seen.insert(i){return Err(Error::Invalid("duplicate or external fraction glyph"));}
            }
        }
    }
    let mut nested=BTreeSet::new();
    for (fi,f) in m.fractions.iter().enumerate(){
        for (above,ids,child) in [(true,&f.numerator,f.numerator_fraction),(false,&f.denominator,f.denominator_fraction)]{
            if let Some(rule)=child{
                let ci=*by_rule.get(&rule).ok_or(Error::Invalid("nested fraction rule"))?;
                let c=&m.fractions[ci];
                let w=f.rule_bounds[2]-f.rule_bounds[0];let cw=c.rule_bounds[2]-c.rule_bounds[0];
                if cw>=w||!nested.insert(ci){return Err(Error::Unsupported("cyclic or shared fraction child"));}
                let expected:BTreeSet<usize>=c.numerator.iter().chain(&c.denominator).copied().collect();
                if expected!=ids.iter().copied().collect(){return Err(Error::Invalid("nested fraction glyph ownership"));}
                if (above&&c.rule_bounds[1]<=f.rule_bounds[3])||(!above&&c.rule_bounds[3]>=f.rule_bounds[1]){
                    return Err(Error::Unsupported("nested fraction on wrong side"));
                }
                for &i in ids{
                    let a=g.glyphs[i].outline_bounds.ok_or(Error::Invalid("nested glyph bounds"))?;
                    if (above&&a[1]<f.rule_bounds[3])||(!above&&a[3]>f.rule_bounds[1]){
                        return Err(Error::Unsupported("nested fraction crosses rule"));
                    }
                }
                continue;
            }
            for &i in ids{
                let o=owner.get_mut(&i).ok_or(Error::Invalid("fraction glyph outside region"))?;
                if o.is_some(){return Err(Error::Unsupported("overlapping fraction candidates"));}
                *o=Some((fi,above));
            }
        }
    }
    let mut radical_symbols=BTreeSet::new();
    let mut radical_sides:BTreeMap<(usize,bool),usize>=BTreeMap::new();
    let mut nested_radicals=BTreeSet::new();
    for (ri,r) in m.radicals.iter().enumerate(){
        if !rules.insert(r.rule_command_index)||r.radicand.is_empty()||!radical_symbols.insert(r.symbol)
            ||g.glyphs.get(r.symbol).and_then(|a|a.unicode.as_deref())!=Some("√"){
            return Err(Error::Invalid("invalid radical candidate"));
        }
        if r.radicand.len()>MAX_GLYPHS||r.rule_bounds.iter().any(|v|!v.is_finite())||
            r.rule_bounds[0]>=r.rule_bounds[2]||r.rule_bounds[1]>=r.rule_bounds[3]{
            return Err(Error::Invalid("radical rule or row bounds"));
        }
        let mut members=BTreeSet::new();
        for i in std::iter::once(r.symbol).chain(r.radicand.iter().copied()){
            if !selected.contains(&i)||!members.insert(i){return Err(Error::Invalid("duplicate or external radical glyph"));}
        }
        let previous=owner[&r.symbol];
        if let Some((fi,above))=previous{
            // The radical symbol must be explicitly selected by the fraction
            // row detector. Only an entire side of a non-nested fraction can
            // be replaced; no partially intersecting or competing structures.
            let f=m.fractions.get(fi).ok_or(Error::Unsupported("overlapping radical candidates"))?;
            if nested.contains(&fi)||f.numerator_fraction.is_some()||f.denominator_fraction.is_some(){
                return Err(Error::Unsupported("radical in nested fraction"));
            }
            let ids=if above{&f.numerator}else{&f.denominator};
            if !ids.iter().all(|i|members.contains(i))||members.iter().any(|i|owner[i].is_some()&&owner[i]!=previous){
                return Err(Error::Unsupported("partial radical fraction ownership"));
            }
            let b=f.rule_bounds;
            let em=g.glyphs[r.symbol].em_matrix.0[3];
            let mut extent=r.rule_bounds;
            for &i in &members{
                let a=g.glyphs[i].outline_bounds.ok_or(Error::Invalid("radical glyph bounds"))?;
                if a.iter().any(|v|!v.is_finite())||a[0]>a[2]||a[1]>a[3]{return Err(Error::Invalid("radical glyph bounds"));}
                extent=[extent[0].min(a[0]),extent[1].min(a[1]),extent[2].max(a[2]),extent[3].max(a[3])];
            }
            let width=b[2]-b[0];
            let gap=if above{extent[1]-b[3]}else{b[1]-extent[3]};
            if gap<0.||gap>em*0.8||extent[0]<b[0]-em*0.1||extent[2]>b[2]+em*0.1
                ||((extent[0]+extent[2])-(b[0]+b[2])).abs()>width*0.4
                ||extent[2]-extent[0]<width*0.15{
                return Err(Error::Unsupported("radical outside fraction side"));
            }
            if radical_sides.insert((fi,above),ri).is_some(){return Err(Error::Unsupported("shared radical fraction side"));}
            nested_radicals.insert(ri);
        }else if members.iter().any(|i|owner[i].is_some()){
            return Err(Error::Unsupported("overlapping radical or fraction candidates"));
        }
        // Assign all members to the innermost row; their fraction parent is
        // represented by radical_sides instead of emitting duplicate glyphs.
        for i in members{
            *owner.get_mut(&i).ok_or(Error::Invalid("radical glyph outside region"))?=Some((m.fractions.len()+ri,true));
        }
    }
    let mut children:Children=BTreeMap::new();let mut parents=BTreeSet::new();
    for s in &m.scripts{
        let a=owner.get(&s.base).ok_or(Error::Invalid("script base outside region"))?;
        let b=owner.get(&s.script).ok_or(Error::Invalid("script glyph outside region"))?;
        if a!=b||radical_symbols.contains(&s.base)||radical_symbols.contains(&s.script){return Err(Error::Unsupported("cross-row or radical symbol script candidate"));}
        // Strictly descending em size makes the graph acyclic.
        if g.glyphs[s.script].em_matrix.0[3]>=g.glyphs[s.base].em_matrix.0[3]||!parents.insert(s.script){
            return Err(Error::Unsupported("cyclic or shared math script"));
        }
        let slots=children.entry(s.base).or_default();
        let slot=match s.kind{ScriptKind::Subscript=>&mut slots.0,ScriptKind::Superscript=>&mut slots.1};
        slot.push(s.script);
    }
    for (sub,sup) in children.values_mut(){
        for slot in [sub,sup]{
            slot.sort_by(|&a,&b|g.glyphs[a].origin[0].total_cmp(&g.glyphs[b].origin[0]).then(a.cmp(&b)));
            for pair in slot.windows(2){
                let a=&g.glyphs[pair[0]];let b=&g.glyphs[pair[1]];
                let em=a.em_matrix.0[3];
                if pair[1]!=pair[0]+1||a.command_index!=b.command_index||a.form_scope!=b.form_scope
                    ||a.font_resource!=b.font_resource||a.font_resource_owner!=b.font_resource_owner
                    ||b.origin[0]<=a.origin[0]||!(0.9..=1.1).contains(&(b.em_matrix.0[3]/em))
                    ||(b.origin[1]-a.origin[1]).abs()>em*0.15
                    ||!(-0.15..=0.3).contains(&((b.origin[0]-a.advance_end[0])/em)){
                    return Err(Error::Unsupported("incompatible multi-glyph script row"));
                }
            }
        }
    }
    let top:Vec<usize>=selected.iter().copied().filter(|i|owner[i].is_none()).collect();
    let mut entries=row_nodes(&top,g,&parents,&children)?;
    for (fi,f) in m.fractions.iter().enumerate(){
        if nested.contains(&fi){continue;}
        entries.push((f.rule_bounds[0],fraction_node(fi,m,g,&by_rule,&radical_sides,&parents,&children,0)?));
    }
    for (ri,r) in m.radicals.iter().enumerate(){
        if nested_radicals.contains(&ri){continue;}
        entries.push((g.glyphs[r.symbol].origin[0],radical_node(ri,m,g,&parents,&children)?));
    }
    Ok(FormulaTree{root:sorted_row(entries)})
}
fn radical_node(ri:usize,m:&MathRegion,g:&PageGlyphGeometry,parents:&BTreeSet<usize>,children:&Children)->Result<MathNode>{
    let r=&m.radicals[ri];
    let nodes=row_nodes(&r.radicand,g,parents,children)?;
    if nodes.is_empty(){return Err(Error::Invalid("empty radicand"));}
    Ok(MathNode::Radical{symbol:r.symbol,rule_command_index:r.rule_command_index,radicand:Box::new(sorted_row(nodes))})
}
fn fraction_node(fi:usize,m:&MathRegion,g:&PageGlyphGeometry,by_rule:&BTreeMap<usize,usize>,
    radical_sides:&BTreeMap<(usize,bool),usize>,parents:&BTreeSet<usize>,children:&Children,depth:usize)->Result<MathNode>{
    if depth>MAX_DEPTH{return Err(Error::Limit("nested fraction depth"));}
    let f=&m.fractions[fi];
    let side=|ids:&[usize],child:Option<usize>,above:bool|->Result<MathNode>{
        if let Some(&ri)=radical_sides.get(&(fi,above)){return radical_node(ri,m,g,parents,children);}
        if let Some(rule)=child{
            let ci=*by_rule.get(&rule).ok_or(Error::Invalid("nested fraction rule"))?;
            return fraction_node(ci,m,g,by_rule,radical_sides,parents,children,depth+1);
        }
        let n=row_nodes(ids,g,parents,children)?;
        if n.is_empty(){return Err(Error::Invalid("empty fraction row"));}
        Ok(sorted_row(n))
    };
    Ok(MathNode::Fraction{rule_command_index:f.rule_command_index,
        numerator:Box::new(side(&f.numerator,f.numerator_fraction,true)?),
        denominator:Box::new(side(&f.denominator,f.denominator_fraction,false)?)})
}
// Every command is authored here, never taken from PDF text. Empty/multiple
// scalars and unknown characters fail; no raw TeX control sequence injection.
fn token(s:&str)->Result<&str>{
    let mut chars=s.chars();
    let c=chars.next().ok_or(Error::Unsupported("empty formula Unicode"))?;
    if chars.next().is_some(){return Err(Error::Unsupported("multi-scalar formula glyph"));}
    if c.is_ascii_alphanumeric()||"+-=*/.,:;()[]!<>|".contains(c){return Ok(s);}
    Ok(match c{
        '−'=>"-",'×'=>r"\times{}",'÷'=>r"\div{}",'·'=>r"\cdot{}",'±'=>r"\pm{}",'∓'=>r"\mp{}",
        '≤'=>r"\le{}",'≥'=>r"\ge{}",'≠'=>r"\ne{}",'≈'=>r"\approx{}",'∞'=>r"\infty{}",
        '∑'=>r"\sum{}",'∏'=>r"\prod{}",'∫'=>r"\int{}",'∂'=>r"\partial{}",'∇'=>r"\nabla{}",
        '∈'=>r"\in{}",'∉'=>r"\notin{}",'→'=>r"\to{}",'←'=>r"\leftarrow{}",
        'α'=>r"\alpha{}",'β'=>r"\beta{}",'γ'=>r"\gamma{}",'δ'=>r"\delta{}",'ε'=>r"\epsilon{}",
        'ϵ'=>r"\varepsilon{}",'θ'=>r"\theta{}",'λ'=>r"\lambda{}",'μ'=>r"\mu{}",'ν'=>r"\nu{}",
        'ξ'=>r"\xi{}",'π'=>r"\pi{}",'ρ'=>r"\rho{}",'σ'=>r"\sigma{}",'τ'=>r"\tau{}",
        'φ'=>r"\phi{}",'ϕ'=>r"\varphi{}",'χ'=>r"\chi{}",'ψ'=>r"\psi{}",'ω'=>r"\omega{}",
        'Γ'=>r"\Gamma{}",'Δ'=>r"\Delta{}",'Θ'=>r"\Theta{}",'Λ'=>r"\Lambda{}",'Ξ'=>r"\Xi{}",
        'Π'=>r"\Pi{}",'Σ'=>r"\Sigma{}",'Φ'=>r"\Phi{}",'Ψ'=>r"\Psi{}",'Ω'=>r"\Omega{}",
        '{'=>r"\{",'}'=>r"\}",'_'=>r"\_",'%'=>r"\%",'#'=>r"\#",'&'=>r"\&",'$'=>r"\$",
        _=>return Err(Error::Unsupported("unmapped formula symbol")),
    })
}
fn emit(n:&MathNode,g:&PageGlyphGeometry,out:&mut String,depth:usize,count:&mut usize)->Result<()>{
    if depth>MAX_DEPTH*3+4{return Err(Error::Limit("latex tree depth"));}
    *count+=1;if *count>MAX_GLYPHS*4+512{return Err(Error::Limit("latex tree nodes"));}
    match n{
        MathNode::Glyph{index}=>{
            let a=g.glyphs.get(*index).ok_or(Error::Invalid("latex glyph index"))?;
            out.push_str(token(a.unicode.as_deref().ok_or(Error::Unsupported("missing formula Unicode"))?)?);
        }
        MathNode::Row(nodes)=>for child in nodes{emit(child,g,out,depth+1,count)?;},
        MathNode::Scripts{base,sub,sup}=>{
            out.push('{');emit(base,g,out,depth+1,count)?;out.push('}');
            for (marker,child) in [('_',sub),('^',sup)]{
                if let Some(c)=child{out.push(marker);out.push('{');emit(c,g,out,depth+1,count)?;out.push('}');}
            }
        }
        MathNode::Radical{symbol,radicand,..}=>{
            if g.glyphs.get(*symbol).and_then(|a|a.unicode.as_deref())!=Some("√"){return Err(Error::Invalid("latex radical symbol"));}
            out.push_str(r"\sqrt{");emit(radicand,g,out,depth+1,count)?;out.push('}');
        }
        MathNode::Fraction{numerator,denominator,..}=>{
            out.push_str(r"\frac{");emit(numerator,g,out,depth+1,count)?;out.push_str("}{");
            emit(denominator,g,out,depth+1,count)?;out.push('}');
        }
    }
    if out.len()>MAX_LATEX{return Err(Error::Limit("latex output bytes"));}Ok(())
}
impl FormulaTree{
    /// Candidate math-mode LaTeX fragment, not a verified transcription.
    /// Pass the same glyph geometry used to build the tree.
    pub fn to_latex(&self,g:&PageGlyphGeometry)->Result<String>{
        let mut out=String::new();let mut count=0;
        emit(&self.root,g,&mut out,0,&mut count)?;Ok(out)
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn literal_tokens_are_allowlisted(){
        assert_eq!(token("α").unwrap(),r"\alpha{}");
        assert_eq!(token("%").unwrap(),r"\%");
        assert!(token(r"\input").is_err());assert!(token("\\").is_err());
        assert!(token("√").is_err());assert!(token("fi").is_err());assert!(token("").is_err());
    }
}
