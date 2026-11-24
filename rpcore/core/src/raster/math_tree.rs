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
}
#[derive(Debug,Clone,PartialEq)]
pub struct FormulaTree{pub root:MathNode}
type RowId=Option<(usize,bool)>;
type Children=BTreeMap<usize,(Option<usize>,Option<usize>)>;

fn glyph_node(i:usize,children:&Children,depth:usize)->Result<MathNode>{
    if depth>MAX_DEPTH{return Err(Error::Limit("math tree depth"));}
    let base=MathNode::Glyph{index:i};
    let Some(&(sub,sup))=children.get(&i)else{return Ok(base);};
    let child=|v:Option<usize>|->Result<Option<Box<MathNode>>>{
        v.map(|j|glyph_node(j,children,depth+1).map(Box::new)).transpose()
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
/// Convert local candidate relationships to a tree. Disjoint fractions and
/// nested scripts are supported. Ambiguous ownership, cross-row scripts,
/// nested/overlapping fractions and multiple children for one script slot
/// are explicitly rejected, not resolved by guessing.
pub fn build_tree(g:&PageGlyphGeometry,m:&MathRegion)->Result<FormulaTree>{
    if m.glyph_indices.is_empty(){return Err(Error::Invalid("empty formula tree"));}
    if m.glyph_indices.len()>MAX_GLYPHS||m.scripts.len()>MAX_GLYPHS||m.fractions.len()>256{return Err(Error::Limit("math tree inputs"));}
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
    for (fi,f) in m.fractions.iter().enumerate(){
        if !rules.insert(f.rule_command_index)||f.numerator.is_empty()||f.denominator.is_empty(){
            return Err(Error::Invalid("invalid formula fraction"));
        }
        if f.rule_bounds.iter().any(|v|!v.is_finite())||f.rule_bounds[0]>=f.rule_bounds[2]||f.rule_bounds[1]>=f.rule_bounds[3]{
            return Err(Error::Invalid("formula rule bounds"));
        }
        for (above,ids) in [(true,&f.numerator),(false,&f.denominator)]{
            if ids.len()>MAX_GLYPHS{return Err(Error::Limit("formula row glyphs"));}
            for &i in ids{
                let o=owner.get_mut(&i).ok_or(Error::Invalid("fraction glyph outside region"))?;
                if o.is_some(){return Err(Error::Unsupported("overlapping or nested fraction candidates"));}
                *o=Some((fi,above));
            }
        }
    }
    let mut children:Children=BTreeMap::new();let mut parents=BTreeSet::new();
    for s in &m.scripts{
        let a=owner.get(&s.base).ok_or(Error::Invalid("script base outside region"))?;
        let b=owner.get(&s.script).ok_or(Error::Invalid("script glyph outside region"))?;
        if a!=b{return Err(Error::Unsupported("cross-row script candidate"));}
        // Strictly descending em size makes the graph acyclic.
        if g.glyphs[s.script].em_matrix.0[3]>=g.glyphs[s.base].em_matrix.0[3]||!parents.insert(s.script){
            return Err(Error::Unsupported("cyclic or shared math script"));
        }
        let slots=children.entry(s.base).or_default();
        let slot=match s.kind{ScriptKind::Subscript=>&mut slots.0,ScriptKind::Superscript=>&mut slots.1};
        if slot.replace(s.script).is_some(){return Err(Error::Unsupported("multiple glyphs in script slot"));}
    }
    let top:Vec<usize>=selected.iter().copied().filter(|i|owner[i].is_none()).collect();
    let mut entries=row_nodes(&top,g,&parents,&children)?;
    for f in &m.fractions{
        let n=row_nodes(&f.numerator,g,&parents,&children)?;
        let d=row_nodes(&f.denominator,g,&parents,&children)?;
        if n.is_empty()||d.is_empty(){return Err(Error::Invalid("empty fraction row"));}
        entries.push((f.rule_bounds[0],MathNode::Fraction{rule_command_index:f.rule_command_index,
            numerator:Box::new(sorted_row(n)),denominator:Box::new(sorted_row(d))}));
    }
    Ok(FormulaTree{root:sorted_row(entries)})
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
