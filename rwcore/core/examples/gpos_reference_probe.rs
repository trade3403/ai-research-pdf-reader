//! Test-only command-line bridge for an independent HarfBuzz oracle.
//! Input glyph IDs are supplied by the reference process; this does not
//! validate cmap, GSUB, bidi, or complete shaping. No HarfBuzz linkage.
use rwcore::*;
fn read_u16(b:&[u8],at:usize)->u16{u16::from_be_bytes([b[at],b[at+1]])}
fn main()->Result<(),Box<dyn std::error::Error>>{
    let mut args=std::env::args().skip(1);
    let path=args.next().ok_or("usage: gpos_reference_probe FONT GLYPH_IDS ...")?;
    let bytes=std::fs::read(path)?;
    let font=parse_font(&bytes,Default::default())?;
    let face=font.face(0).ok_or("font face")?;
    let b=face.table(*b"GPOS").ok_or("missing GPOS")?;
    let n=u32::from(read_u16(face.table(*b"maxp").ok_or("missing maxp")?,4));
    let gd=face.table(*b"GDEF").map(|g|parse_gdef(g,n,Default::default())).transpose()?;
    let layout=parse_gpos_layout(b,Default::default())?;
    let on=build_gpos_base_plan(&layout,*b"latn",None,&[*b"kern"],Default::default())?.ok_or("missing latn plan")?;
    let off=build_gpos_base_plan(&layout,*b"latn",None,&[],Default::default())?.ok_or("missing base plan")?;
    if on.unavailable_tags.contains(b"kern"){return Err("missing native GPOS kern".into());}
    let p=compile_gpos_plan_program(b,&on,n,gd.as_ref(),Default::default())?;
    let q=compile_gpos_plan_program(b,&off,n,gd.as_ref(),Default::default())?;
    if p.lookups().iter().chain(q.lookups()).any(|l|!matches!(l.effective_kind(),1|2)){
        return Err("reference scope requires single/pair positioning".into());
    }
    let h=face.table(*b"hhea").ok_or("missing hhea")?;
    let m=face.table(*b"hmtx").ok_or("missing hmtx")?;
    let count=usize::from(read_u16(h,34));
    if count==0{return Err("empty hmtx".into());}
    for group in args{
        let ids:Vec<u16>=group.split(',').map(str::parse).collect::<Result<_,_>>()?;
        if ids.is_empty()||ids.len()>64||ids.iter().any(|&g|u32::from(g)>=n){return Err("glyph domain".into());}
        let nominal:Vec<_>=ids.iter().map(|&g|(i32::from(read_u16(m,4*usize::from(g).min(count-1))),0)).collect();
        let prior=vec![GposRunAdjustment::default();ids.len()];
        let zero=vec![(0,0);ids.len()];
        let sources=vec![GposLigatureSource::Unmapped;ids.len()];
        let run=|program:&CompiledGposProgram<'_>|execute_gpos_mixed_joint(program,&ids,&prior,&nominal,&zero,&sources,
            GposRunMode::DesignOnly,GposHorizontalFlow::LeftToRight,Default::default(),Default::default());
        let a=run(&p)?;let z=run(&q)?;
        let mut row=Vec::new();
        for (x,y) in a.adjustments.iter().zip(&z.adjustments){
            let x=x.design;let y=y.design;
            row.push(format!("{},{},{},{}",x.x_advance-y.x_advance,x.y_advance-y.y_advance,
                x.x_placement-y.x_placement,x.y_placement-y.y_placement));
        }
        println!("{}",row.join(";"));
    }
    Ok(())
}
