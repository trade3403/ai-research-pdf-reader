//! Conservative local geometric formula candidates, not a semantic parser.
use super::{PageGlyphGeometry,page_viewport::validate_rect};
use crate::{graphics::{Command,DisplayList,Segment,LineCap},Error,Result};
const MAX_REGION_GLYPHS:usize=512;
const MAX_RULES:usize=256;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ScriptKind{Superscript,Subscript}
#[derive(Debug,Clone,PartialEq)]
pub struct ScriptCandidate{
    /// Indices in the returned PageGlyphGeometry, not Unicode offsets.
    pub base:usize,pub script:usize,pub kind:ScriptKind,
}
#[derive(Debug,Clone,PartialEq)]
pub struct FractionCandidate{
    pub rule_command_index:usize,pub rule_bounds:[f64;4],
    pub numerator:Vec<usize>,pub denominator:Vec<usize>,
    /// Optional child fraction rule index; that entire side is the child.
    /// The corresponding glyph list includes all descendant glyphs.
    pub numerator_fraction:Option<usize>,pub denominator_fraction:Option<usize>,
}
#[derive(Debug,Clone,PartialEq)]
pub struct RadicalCandidate{
    pub symbol:usize,
    pub rule_command_index:usize,
    pub rule_bounds:[f64;4],
    pub radicand:Vec<usize>,
}
#[derive(Debug,Clone,PartialEq)]
pub struct MathRegion{
    pub region:[f64;4],
    pub glyph_indices:Vec<usize>,
    pub scripts:Vec<ScriptCandidate>,
    pub fractions:Vec<FractionCandidate>,
    pub radicals:Vec<RadicalCandidate>,
}
#[derive(Clone,Copy)]
struct Item{index:usize,bounds:[f64;4],x:f64,y:f64,advance_x:f64,em:f64}
fn contains(r:[f64;4],b:[f64;4])->bool{
    b[0]>=r[0]&&b[1]>=r[1]&&b[2]<=r[2]&&b[3]<=r[3]
}
fn rule_bounds(c:&Command)->Option<[f64;4]>{
    let Command::Paint{path,fill,stroke}=c else{return None;};
    match (path.as_slice(),fill,stroke){
        ([Segment::Move(a),Segment::Line(b),Segment::Line(c),Segment::Line(d),Segment::Close],Some(_),None)=>{
            // Only a single nondegenerate axis-aligned rectangle.
            if !((a[1]==b[1]&&b[0]==c[0]&&c[1]==d[1]&&d[0]==a[0])||
                 (a[0]==b[0]&&b[1]==c[1]&&c[0]==d[0]&&d[1]==a[1])){return None;}
            Some([a[0].min(c[0]),a[1].min(c[1]),a[0].max(c[0]),a[1].max(c[1])])
        }
        ([Segment::Move(a),Segment::Line(b)],None,Some(s))=>{
            let [x,y,z,w,_,_]=s.ctm.0;
            // No hairline, dash, rotated/sheared or nonuniform stroke.
            if a[1]!=b[1]||!s.dash.is_empty()||s.width<=0.||s.cap!=LineCap::Butt
                ||y!=0.||z!=0.||x.abs()!=w.abs()||x==0.{return None;}
            let h=s.width*x.abs()*0.5;
            Some([a[0].min(b[0]),a[1]-h,a[0].max(b[0]),a[1]+h])
        }
        _=>None,
    }
}
fn row(items:&[Item],bar:[f64;4],above:bool)->Vec<usize>{
    let cy=(bar[1]+bar[3])*0.5;
    let width=bar[2]-bar[0];
    let mut candidates:Vec<(Item,f64)>=Vec::new();
    for &g in items{
        let gap=if above{g.bounds[1]-bar[3]}else{bar[1]-g.bounds[3]};
        let margin=g.em*0.2;
        if gap<0.||gap>g.em*0.8||g.bounds[0]<bar[0]-margin||g.bounds[2]>bar[2]+margin
            ||g.bounds[2]-g.bounds[0]>width*1.25{continue;}
        if (above&&g.y<=cy)||(!above&&g.y>=cy){continue;}
        candidates.push((g,gap));
    }
    let Some(&(nearest,_))=candidates.iter().min_by(|a,b|a.1.total_cmp(&b.1))else{return Vec::new();};
    // Keep only the closest compatible baseline row.
    let mut row:Vec<Item>=candidates.into_iter().map(|(g,_)|g).filter(|g|
        (g.y-nearest.y).abs()<=nearest.em*0.25&&(0.65..=1.5).contains(&(g.em/nearest.em))).collect();
    if row.is_empty(){return Vec::new();}
    let left=row.iter().map(|g|g.bounds[0]).fold(f64::INFINITY,f64::min);
    let right=row.iter().map(|g|g.bounds[2]).fold(f64::NEG_INFINITY,f64::max);
    // A centered row, not text simply passing near a table rule.
    if ((left+right)-(bar[0]+bar[2])).abs()>width*0.5||right-left<width*0.15{return Vec::new();}
    row.sort_by(|a,b|a.x.total_cmp(&b.x).then(a.index.cmp(&b.index)));
    row.into_iter().map(|g|g.index).collect()
}
/// Analyze an explicitly selected region in unrotated PDF coordinates.
/// Geometry and display list must come from the same page and font policy.
/// The output consists only of heuristic candidates; it does not establish
/// that the selected content is mathematics. No reading-order or LaTeX claim.
pub fn analyze_region(geometry:&PageGlyphGeometry,list:&DisplayList,region:[f64;4])->Result<MathRegion>{
    validate_rect(region)?;
    if region[0]>=region[2]||region[1]>=region[3]{return Err(Error::Invalid("empty math region"));}
    let mut items=Vec::new();
    let mut indices=Vec::new();
    for (i,g) in geometry.glyphs.iter().enumerate(){
        let Some(b)=g.outline_bounds else{continue;};
        if !contains(region,b){continue;}
        if indices.len()>=MAX_REGION_GLYPHS{return Err(Error::Limit("math region glyphs"));}
        indices.push(i);
        let [a,b1,c,d,_,_]=g.em_matrix.0;
        // Analyze upright horizontal glyphs only, leave other orientations unclassified.
        if a<=0.||d<=0.||b1!=0.||c!=0.||g.advance_end[0]<g.origin[0]{continue;}
        items.push(Item{index:i,bounds:b,x:g.origin[0],y:g.origin[1],advance_x:g.advance_end[0],em:d});
    }
    let mut fractions=Vec::new();let mut radicals=Vec::new();let mut rules=Vec::new();let mut rule_count=0;
    for (i,c) in list.commands.iter().enumerate(){
        let Some(b)=rule_bounds(c)else{continue;};
        let w=b[2]-b[0];let h=b[3]-b[1];
        if !contains(region,b)||h<=0.||w<h*6.{continue;}
        rule_count+=1;if rule_count>MAX_RULES{return Err(Error::Limit("math region rules"));}
        rules.push((i,b));
        let numerator=row(&items,b,true);let denominator=row(&items,b,false);
        // Explicit Unicode radical plus a separate overbar. Do not infer
        // an arbitrary scope from the radical glyph alone.
        if numerator.is_empty()&&!denominator.is_empty(){
            for root in &items{
                if geometry.glyphs[root.index].unicode.as_deref()!=Some("√"){continue;}
                let em=root.em;
                if (b[0]-root.bounds[2]).abs()>em*0.25||(b[1]-root.bounds[3]).abs()>em*0.25
                    ||h>em*0.15||w>em*16.||b[2]<=root.bounds[2]{continue;}
                if denominator.iter().any(|i|*i==root.index){continue;}
                if !denominator.iter().all(|&i|{
                    let g=&geometry.glyphs[i];
                    g.form_scope==geometry.glyphs[root.index].form_scope
                    &&g.outline_bounds.is_some_and(|a|a[0]>=root.bounds[2]-em*0.1&&a[2]<=b[2]+em*0.1)
                    &&(g.origin[1]-root.y).abs()<=em*0.3
                    &&g.unicode.as_deref()!=Some("√")
                }){continue;}
                if radicals.len()>=MAX_RULES{return Err(Error::Limit("math radicals"));}
                radicals.push(RadicalCandidate{symbol:root.index,rule_command_index:i,rule_bounds:b,radicand:denominator.clone()});
            }
        }
    }
    // Process narrow rules first. A nested fraction can occupy an entire
    // side only, not an arbitrary expression beside another term.
    rules.sort_by(|a,b|(a.1[2]-a.1[0]).total_cmp(&(b.1[2]-b.1[0])).then(a.0.cmp(&b.0)));
    let mut nested=std::collections::BTreeSet::new();
    let mut extents:Vec<[f64;4]>=Vec::new();
    let mut sizes:Vec<f64>=Vec::new();
    for (i,b) in rules{
        if radicals.iter().any(|r|r.rule_command_index==i){continue;}
        let w=b[2]-b[0];let h=b[3]-b[1];
        let side=|above:bool|->Option<(Vec<usize>,Option<usize>)>{
            let direct=row(&items,b,above);
            let mut matching=None;
            for (fi,f) in fractions.iter().enumerate(){
                let f:&FractionCandidate=f;
                if nested.contains(&f.rule_command_index){continue;}
                let a=extents[fi];let em=sizes[fi];
                let gap=if above{a[1]-b[3]}else{b[1]-a[3]};
                if gap<0.||gap>em*0.8||(f.rule_bounds[2]-f.rule_bounds[0])>w*0.8
                    ||a[0]<b[0]-em*0.1||a[2]>b[2]+em*0.1
                    ||((a[0]+a[2])-(b[0]+b[2])).abs()>w*0.4{continue;}
                let ids:Vec<usize>=f.numerator.iter().chain(&f.denominator).copied().collect();
                if !direct.iter().all(|j|ids.contains(j)){continue;}
                if matching.is_some(){return None;}
                matching=Some((ids,Some(f.rule_command_index)));
            }
            matching.or_else(||(!direct.is_empty()).then_some((direct,None)))
        };
        let Some((numerator,numerator_fraction))=side(true)else{continue;};
        let Some((denominator,denominator_fraction))=side(false)else{continue;};
        if numerator.iter().any(|j|denominator.contains(j)){continue;}
        let em=items.iter().filter(|g|numerator.contains(&g.index)||denominator.contains(&g.index))
            .map(|g|g.em).fold(0.,f64::max);
        if h>em*0.2||w>em*16.{continue;}
        let mut bounds=b;
        for g in items.iter().filter(|g|numerator.contains(&g.index)||denominator.contains(&g.index)){
            bounds[0]=bounds[0].min(g.bounds[0]);bounds[1]=bounds[1].min(g.bounds[1]);
            bounds[2]=bounds[2].max(g.bounds[2]);bounds[3]=bounds[3].max(g.bounds[3]);
        }
        for child in [numerator_fraction,denominator_fraction].into_iter().flatten(){
            let fi=fractions.iter().position(|f|f.rule_command_index==child).ok_or(Error::Invalid("nested fraction rule"))?;
            let a=extents[fi];bounds[0]=bounds[0].min(a[0]);bounds[1]=bounds[1].min(a[1]);
            bounds[2]=bounds[2].max(a[2]);bounds[3]=bounds[3].max(a[3]);
            nested.insert(child);
        }
        extents.push(bounds);sizes.push(em);
        fractions.push(FractionCandidate{rule_command_index:i,rule_bounds:b,numerator,denominator,numerator_fraction,denominator_fraction});
    }
    // Precompute fraction membership for constant-time pair checks.
    // At most 256 accepted rules, four u64 words per side and glyph.
    let mut membership=vec![([0u64;4],[0u64;4]);items.len()];
    for (fi,f) in fractions.iter().enumerate(){
        for (ii,g) in items.iter().enumerate(){
            if f.numerator.contains(&g.index){membership[ii].0[fi/64]|=1u64<<(fi%64);}
            if f.denominator.contains(&g.index){membership[ii].1[fi/64]|=1u64<<(fi%64);}
        }
    }
    let mut scripts=Vec::new();
    for (gi,g) in items.iter().enumerate(){
        let mut best:Option<(f64,usize,ScriptKind)>=None;let mut ambiguous=false;
        for (bi,b) in items.iter().enumerate(){
            if g.index==b.index||geometry.glyphs[g.index].form_scope!=geometry.glyphs[b.index].form_scope||
                geometry.glyphs[g.index].unicode.as_deref()==Some("√")||geometry.glyphs[b.index].unicode.as_deref()==Some("√"){continue;}
            let ratio=g.em/b.em;
            if !(0.35..=0.82).contains(&ratio){continue;}
            let dy=(g.y-b.y)/b.em;let dx=(g.x-b.advance_x)/b.em;
            if !(-0.3..=0.45).contains(&dx)||g.x<=b.x{continue;}
            let kind=if (0.22..=0.95).contains(&dy){ScriptKind::Superscript}
                else if (-0.65..=-0.15).contains(&dy)
                    // Real TeX 10/7 point scripts can be only 0.14996 em
                    // below the baseline. Admit shallow displacement only
                    // with a strong size change and close advance alignment.
                    ||((-0.15..=-0.12).contains(&dy)&&(0.55..=0.78).contains(&ratio)&&dx.abs()<=0.12)
                    {ScriptKind::Subscript}else{continue;};
            // Do not reinterpret numerator/denominator pairs as scripts.
            if (0..4).any(|k|(membership[bi].0[k]&membership[gi].1[k])!=0||(membership[bi].1[k]&membership[gi].0[k])!=0){continue;}
            let score=dx.abs()+(dy.abs()-0.4).abs()*0.2;
            match best{
                None=>{best=Some((score,b.index,kind));ambiguous=false;}
                Some((s,_,_)) if score<s-0.05=>{best=Some((score,b.index,kind));ambiguous=false;}
                Some((s,_,_)) if (score-s).abs()<=0.05=>{ambiguous=true;}
                _=>{}
            }
        }
        if !ambiguous{if let Some((_,base,kind))=best{scripts.push(ScriptCandidate{base,script:g.index,kind});}}
    }
    // Extend a detected script only along consecutive glyphs of the same
    // text command, with matching baseline and em size. Do not absorb a
    // glyph already assigned to another base or with a different fraction row.
    let mut attached=std::collections::BTreeMap::new();
    for s in &scripts{attached.insert(s.script,(s.base,s.kind));}
    for pair in items.windows(2){
        let (a,b)=(pair[0],pair[1]);
        let Some(&(base,kind))=attached.get(&a.index)else{continue;};
        if attached.contains_key(&b.index)||b.index!=a.index+1||b.x<=a.x{continue;}
        let ga=&geometry.glyphs[a.index];let gb=&geometry.glyphs[b.index];
        if ga.command_index!=gb.command_index||ga.form_scope!=gb.form_scope||ga.font_resource_owner!=gb.font_resource_owner
            ||ga.font_resource!=gb.font_resource||gb.unicode.as_deref()==Some("√"){continue;}
        if !(0.9..=1.1).contains(&(b.em/a.em))||(b.y-a.y).abs()>a.em*0.15
            ||!(-0.15..=0.3).contains(&((b.x-a.advance_x)/a.em)){continue;}
        let ai=items.iter().position(|g|g.index==a.index).ok_or(Error::Invalid("script item"))?;
        let bi=ai+1;
        if membership[ai]!=membership[bi]{continue;}
        attached.insert(b.index,(base,kind));scripts.push(ScriptCandidate{base,script:b.index,kind});
    }
    // Expand fraction rows with the scripts of their existing base glyphs.
    // Work from larger bases to smaller ones so nested scripts inherit all
    // ancestor row memberships. A script may extend a row only when its
    // outline is on the same side of the rule and horizontally within it.
    // Never move a glyph out of an already assigned different row.
    let by_index:std::collections::BTreeMap<usize,usize>=items.iter().enumerate().map(|(i,g)|(g.index,i)).collect();
    scripts.sort_by(|a,b|items[by_index[&b.base]].em.total_cmp(&items[by_index[&a.base]].em).then(a.script.cmp(&b.script)));
    for s in &scripts{
        let bi=by_index[&s.base];let si=by_index[&s.script];let g=items[si];let base=items[bi];
        if membership[bi]==membership[si]{continue;}
        if membership[si]!=([0u64;4],[0u64;4]){continue;}
        // A radical is still an independent candidate. Do not steal its glyphs
        // into a fraction until radical/fraction nesting has explicit ownership.
        if radicals.iter().any(|r|r.symbol==g.index||r.radicand.contains(&g.index)){continue;}
        let mut valid=true;
        for (fi,f) in fractions.iter().enumerate(){
            let mask=1u64<<(fi%64);let above=membership[bi].0[fi/64]&mask!=0;let below=membership[bi].1[fi/64]&mask!=0;
            if !above&&!below{continue;}
            let b=f.rule_bounds;
            if (above&&g.bounds[1]<b[3])||(below&&g.bounds[3]>b[1])
                ||g.bounds[0]<b[0]-base.em*0.1||g.bounds[2]>b[2]+base.em*0.1{
                valid=false;break;
            }
        }
        if !valid{continue;}
        for (fi,f) in fractions.iter_mut().enumerate(){
            let mask=1u64<<(fi%64);
            if membership[bi].0[fi/64]&mask!=0{f.numerator.push(g.index);}
            if membership[bi].1[fi/64]&mask!=0{f.denominator.push(g.index);}
        }
        membership[si]=membership[bi];
    }
    // Preserve display-list order of relationship records for API callers.
    scripts.sort_by_key(|s|s.script);
    Ok(MathRegion{region,glyph_indices:indices,scripts,fractions,radicals})
}
