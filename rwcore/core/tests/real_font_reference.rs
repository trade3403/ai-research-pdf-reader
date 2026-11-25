//! Test-only independent dense quadratic sampling and point-in-polygon oracle.
//! Shares the parsed path, not the production flattening or scanline rasterizer.
use rwcore::*;
const FONT: &[u8] = include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf");
type P = (f64,f64);
fn reference(path:&GlyphPath, v:RasterViewport, ppem:f64, ox:f64, oy:f64)->GlyphBitmap {
    assert!(v.width<=128 && v.height<=128 && path.commands().len()<=512);
    let cv=|p:PathPoint| (ox+p.x as f64/PATH_ONE as f64*ppem/1000.0,
                         oy-p.y as f64/PATH_ONE as f64*ppem/1000.0);
    let mut edges:Vec<(P,P)>=Vec::new();
    let mut cur=(0.0,0.0);
    let mut start=cur;
    for &cmd in path.commands() {
        match cmd {
            GlyphPathCommand::MoveTo(p)=>{cur=cv(p);start=cur;},
            GlyphPathCommand::LineTo(p)=>{let q=cv(p);edges.push((cur,q));cur=q;},
            GlyphPathCommand::QuadTo{control,end}=>{
                let a=cur;let b=cv(control);let c=cv(end);
                for i in 1..=128 {
                    let t=i as f64/128.0;let u=1.0-t;
                    let q=(u*u*a.0+2.0*u*t*b.0+t*t*c.0,u*u*a.1+2.0*u*t*b.1+t*t*c.1);
                    edges.push((cur,q));cur=q;
                }
                cur=c;
            },
            GlyphPathCommand::Close=>{edges.push((cur,start));cur=start;}
        }
    }
    let mut alpha=Vec::new();
    for y in 0..v.height {
        for x in 0..v.width {
            let mut count=0u32;
            for sy in 0..8 {
                for sx in 0..8 {
                    let px=v.left as f64+x as f64+(sx as f64+0.5)/8.0;
                    let py=v.top as f64+y as f64+(sy as f64+0.5)/8.0;
                    let mut winding=0i32;
                    for &(a,b) in &edges {
                        if (a.1<=py && py<b.1)||(b.1<=py && py<a.1) {
                            let cross=(b.0-a.0)*(py-a.1)-(b.1-a.1)*(px-a.0);
                            if b.1>a.1 && cross>0.0 { winding+=1; }
                            if b.1<a.1 && cross<0.0 { winding-=1; }
                        }
                    }
                    if winding!=0 {count+=1;}
                }
            }
            alpha.push(((count*255+32)/64) as u8);
        }
    }
    GlyphBitmap{viewport:v,alpha}
}
fn check(ch:char, ppem:u32, ox:i64, oy:i64) {
    let c=parse_font(FONT,FontLimits::default()).unwrap();
    let f=c.face(0).unwrap();
    let r=parse_true_type_renderer(f,RenderFontLimits::default()).unwrap();
    let o=parse_true_type_outlines(f,OutlineLimits::default()).unwrap();
    let actual=r.render_char(ch,MissingGlyphPolicy::Reject,GlyphRenderOptions{
        ppem_26_6:ppem*64,origin_x_26_6:ox,origin_y_26_6:oy,
        samples_per_axis:8,tolerance_16_16:64,..Default::default()
    }).unwrap();
    let path=true_type_glyph_path(&o,actual.glyph_id,CompositeLimits::default(),PathLimits::default()).unwrap();
    // Reference viewport deliberately extends outside the production viewport.
    let v=actual.bitmap.viewport;
    let expanded=if v.width==0||v.height==0 {v} else {
        RasterViewport{left:v.left-1,top:v.top-1,width:v.width+2,height:v.height+2}
    };
    let expected=reference(&path,expanded,ppem as f64,ox as f64/64.0,oy as f64/64.0);
    let e=compare_glyph_bitmaps(&actual.bitmap,&expected,16384).unwrap();
    println!("reference {ch:?} {ppem}px origin=({ox},{oy}): {e:?}");
    if ch==' ' {assert_eq!(e.actual_ink+e.reference_ink,0);return;}
    assert!(e.reference_ink>0);
    // Explicit conservative tolerance for sample decisions near a curve.
    // No assertion of exact analytic coverage or independent outline parsing.
    assert!(e.absolute_error<=e.reference_ink/20+32, "{ch:?}: {e:?}");
    assert!(e.max_error<=64, "{ch:?}: {e:?}");
}
#[test] fn real_reference_a(){check('A',16,0,0);}
#[test] fn real_reference_o(){check('O',24,0,0);}
#[test] fn real_reference_descender(){check('g',20,17,-23);}
#[test] fn real_reference_composite(){check('é',20,32,16);}
#[test] fn real_reference_ring(){check('Å',24,-19,31);}
#[test] fn real_reference_space(){check(' ',16,0,0);}
