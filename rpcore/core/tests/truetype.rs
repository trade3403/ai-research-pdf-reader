#[path="support/tt_fixture.rs"] mod fixture;
use rpcore::{Document,Error,fonts::truetype::TrueType,graphics::Segment,raster::{Options,Bitmap}};
fn pixel(b:&Bitmap,x:usize,y:usize)->&[u8]{let i=(y*b.width+x)*4;&b.rgba[i..i+4]}
#[test]
fn font_metrics_and_contours(){
    let f=TrueType::parse(fixture::font(false)).unwrap();
    assert_eq!(f.units_per_em(),1000);assert_eq!(f.glyph_count(),4);assert_eq!(f.horizontal_metrics(3).unwrap(),(650,100));
    assert!(f.outline(0).unwrap().is_empty());
    let p=f.outline(1).unwrap();assert_eq!(p.len(),10);assert_eq!(p[0],Segment::Move([0.,0.]));
    let p=f.outline(2).unwrap();assert_eq!(p[1],Segment::Cubic([0.,400.],[100.,600.],[300.,600.]));
    assert_eq!(p[2],Segment::Cubic([500.,600.],[600.,400.],[600.,0.]));
    let p=f.outline(3).unwrap();assert_eq!(p[0],Segment::Move([100.,50.]));assert_eq!(p[1],Segment::Line([400.,50.]));
    assert!(f.outline(4).is_err());
}
#[test]
fn malformed_tables_and_compound_cycle(){
    let f=TrueType::parse(fixture::font(true)).unwrap();assert!(matches!(f.outline(3),Err(Error::Invalid(_))));
    let mut b=fixture::font(false);b[20..24].copy_from_slice(&u32::MAX.to_be_bytes());assert!(TrueType::parse(b).is_err());
    let b=fixture::font(false);assert!(TrueType::parse(b[..b.len()-8].to_vec()).is_err());
}
#[test]
fn embedded_glyph_pixels_and_search(){
    let p=fixture::pdf(b"BT /F1 20 Tf 10 20 Td [<0001> -100 <0002> <0003>] TJ ET");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    let b=d.render_page(&pages[0],Options::default(),10000).unwrap();
    assert!(!b.missing_text);assert_eq!((b.width,b.height),(140,90));
    assert_eq!(pixel(&b,11,65),[0,0,0,255]);assert_eq!(pixel(&b,15,64),[255,255,255,255]);
    assert_eq!(pixel(&b,23,65),[255,255,255,255]);assert_eq!(pixel(&b,30,65),[0,0,0,255]);
    let t=d.page_text(&pages[0],10000).unwrap();assert_eq!(t.plain_text(),"ODo");assert_eq!(t.glyphs[1].origin,[25.,20.]);
}
#[test]
fn text_color_order_invisible_and_preview(){
    let p=fixture::pdf(b"1 0 0 rg BT /F1 20 Tf 10 20 Td <0001> Tj 3 Tr <0001> Tj ET 0 0 1 rg 10 20 2 2 re f");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    let b=d.render_page(&pages[0],Options::default(),10000).unwrap();
    assert_eq!(pixel(&b,10,69),[0,0,255,255]);assert_eq!(pixel(&b,11,65),[255,0,0,255]);assert_eq!(pixel(&b,24,65),[255,255,255,255]);
    let preview=d.render_page(&pages[0],Options{scale:1.,allow_missing_text:true},10000).unwrap();assert!(preview.missing_text);assert_eq!(pixel(&preview,11,65),[255,255,255,255]);
}

#[test]fn native_cmap_formats_and_bounds(){
    use rpcore::fonts::truetype::CmapKind;
    let mut f0=vec![0,0,1,6,0,0];f0.resize(262,0);f0[6+65]=1;f0[6+66]=2;
    let f6=fixture::cmap6(65,&[1,2]);
    // Two format-4 segments, direct delta for A/B and sentinel FFFF.
    let f4=[0,4,0,32,0,0,0,4,0,4,0,1,0,0,0,66,255,255,0,0,0,65,255,255,255,192,0,1,0,0,0,0];
    // Format 12 groups map A/B and supplementary scalar to glyph 3.
    let mut f12=vec![0,12,0,0,0,0,0,40,0,0,0,0,0,0,0,2];
    for x in [65u32,66,1,0x1f600,0x1f600,3]{f12.extend(x.to_be_bytes());}
    for (platform,enc,sub) in [(0,3,f0),(3,1,f6),(3,1,f4.to_vec()),(3,10,f12.clone())]{
        let t=TrueType::parse(fixture::font_with_cmap(false,Some(fixture::cmap_table(platform,enc,&sub)))).unwrap();
        let c=t.character_map().unwrap();assert_eq!(c.glyph(CmapKind::Unicode,65),Some(1));assert_eq!(c.glyph(CmapKind::Unicode,66),Some(2));assert_eq!(c.glyph(CmapKind::Unicode,67),None);
        if enc==10{assert_eq!(c.glyph(CmapKind::Unicode,0x1f600),Some(3));}
        let mut bad=sub.clone();bad.pop();let t=TrueType::parse(fixture::font_with_cmap(false,Some(fixture::cmap_table(platform,enc,&bad)))).unwrap();assert!(t.character_map().is_err());
    }
    let bad=fixture::cmap6(65,&[4]);
    let t=TrueType::parse(fixture::font_with_cmap(false,Some(fixture::cmap_table(3,1,&bad)))).unwrap();assert!(t.character_map().is_err());
}
#[test]fn simple_truetype_encoding_and_pixels(){
    for (encoding,flags,platform,enc,start,first) in [
        ("/Encoding /WinAnsiEncoding",32,3,1,65,1),
        ("/Encoding /MacRomanEncoding",32,1,0,65,1),
        ("/Encoding << /BaseEncoding /WinAnsiEncoding /Differences [65 /B] >>",32,3,1,65,2),
        ("",4,3,0,0xf041,1),
    ]{
        let f=fixture::font_with_cmap(false,Some(fixture::cmap_table(platform,enc,&fixture::cmap6(start,&[1,2]))));
        let p=fixture::simple_pdf(b"BT /F1 20 Tf 10 20 Td (A) Tj ET",f,encoding,flags);
        let d=Document::open(&p).unwrap();let b=d.render_page(&d.pages().unwrap()[0],Options::default(),10000).unwrap();
        assert_eq!(pixel(&b,15,64),if first==1{&[255,255,255,255]}else{&[0,0,0,255]},"{encoding}");
        assert!(!b.missing_text);
    }
}
#[test]fn simple_truetype_does_not_guess_glyph_id(){
    let f=fixture::font_with_cmap(false,Some(fixture::cmap_table(3,1,&fixture::cmap6(66,&[2]))));
    let p=fixture::simple_pdf(b"BT /F1 20 Tf 10 20 Td (A) Tj ET",f,"/Encoding /WinAnsiEncoding",32);
    let d=Document::open(&p).unwrap();assert!(matches!(d.render_page(&d.pages().unwrap()[0],Options::default(),10000),Err(Error::Unsupported("missing simple TrueType glyph mapping"))));
}

#[test]fn fine_glyph_coverage_recovers_subpixel_outline(){
    use rpcore::raster::TextQuality;
    let p=fixture::pdf(b"BT /F1 0.25 Tf 10 20 Td <0001> Tj ET");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    let a=d.render_page(&pages[0],Options::default(),10000).unwrap();
    let b=d.render_page_with_text_quality(&pages[0],Options::default(),10000,None,TextQuality::Fine).unwrap().bitmap;
    assert_eq!(pixel(&a,10,69),[255,255,255,255]);
    assert_eq!(pixel(&b,10,69),[239,239,239,255]);
    assert_eq!(pixel(&b,11,69),[255,255,255,255]);
    assert_eq!(d.render_page_with_text_quality(&pages[0],Options::default(),10000,None,TextQuality::Standard).unwrap().bitmap,a);
}
#[test]fn fine_glyph_respects_clip_and_opaque_interiors(){
    use rpcore::raster::TextQuality;
    let p=fixture::pdf(b"q 10 20 2 14 re W n BT /F1 20 Tf 10 20 Td <0001> Tj ET Q");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    let b=d.render_page_with_text_quality(&pages[0],Options::default(),10000,None,TextQuality::Fine).unwrap().bitmap;
    assert_eq!(pixel(&b,11,65),[0,0,0,255]);assert_eq!(pixel(&b,12,65),[255,255,255,255]);
}

#[test]fn experimental_grid_aligns_baseline_without_changing_advances(){
    use rpcore::raster::TextQuality;
    let p=fixture::pdf(b"BT /F1 8 Tf 10 20.4 Td [<0001> -100 <0002>] TJ ET");
    let aligned=fixture::pdf(b"BT /F1 8 Tf 10 20 Td [<0001> -100 <0002>] TJ ET");
    let d=Document::open(&p).unwrap();let e=Document::open(&aligned).unwrap();
    let pages=d.pages().unwrap();let ep=e.pages().unwrap();
    let render=|doc:&Document<'_>,page:&rpcore::PageInfo,q|doc.render_page_with_text_quality(page,Options::default(),10000,None,q).unwrap().bitmap;
    let raw=render(&d,&pages[0],TextQuality::Fine);
    let grid=render(&d,&pages[0],TextQuality::FineGrid);
    assert_ne!(raw,grid);
    assert_eq!(grid,render(&e,&ep[0],TextQuality::Fine));
    let text=d.page_text(&pages[0],10000).unwrap();
    assert_eq!(text.glyphs[0].origin,[10.,20.4]);
    assert!((text.glyphs[1].origin[0]-16.).abs()<1e-9);
    assert_eq!(text.glyphs[1].origin[1],20.4);
}
#[test]fn grid_skips_rise_rotation_large_text_and_stroke(){
    use rpcore::raster::TextQuality;
    for content in [
        b"BT /F1 8 Tf 10 20.4 Td 1 Ts <0001> Tj ET".as_slice(),
        b"BT /F1 8 Tf 0 1 -1 0 20.4 20.4 Tm <0001> Tj ET",
        b"BT /F1 20 Tf 10 20.4 Td <0001> Tj ET",
        b"BT /F1 8 Tf 10 20.4 Td 1 Tr <0001> Tj ET",
        b"BT /F1 8 Tf -1 0 0 1 20 20.4 Tm <0001> Tj ET",
        b"BT /F1 2 Tf 10 20.4 Td <0001> Tj ET",
    ]{
        let p=fixture::pdf(content);let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
        let render=|q|d.render_page_with_text_quality(&pages[0],Options::default(),10000,None,q).unwrap().bitmap;
        assert_eq!(render(TextQuality::Fine),render(TextQuality::FineGrid),"{content:?}");
    }
}

#[test]fn area_glyph_recovers_narrow_vertical_stroke(){
    use rpcore::raster::TextQuality;
    // 20 pt x 1% horizontal scale x 0.6 em = 0.12 px.
    // Fine's first horizontal sample is x=0.125, so misses this stem.
    let p=fixture::pdf(b"BT /F1 20 Tf 1 Tz 10 20 Td <0001> Tj ET");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    let render=|q|d.render_page_with_text_quality(&pages[0],Options::default(),10000,None,q).unwrap().bitmap;
    let a=render(TextQuality::Fine);let b=render(TextQuality::Area);
    assert_eq!(pixel(&a,10,68),[255,255,255,255]);
    assert!((i32::from(pixel(&b,10,68)[0])-224).abs()<=1);
    assert_eq!(pixel(&b,11,68),[255,255,255,255]);
}
#[test]fn area_glyph_preserves_hole_and_clip(){
    use rpcore::raster::TextQuality;
    let p=fixture::pdf(b"q 10 20 8 14 re W n BT /F1 20 Tf 10 20 Td <0001> Tj ET Q");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    let b=d.render_page_with_text_quality(&pages[0],Options::default(),10000,None,TextQuality::Area).unwrap().bitmap;
    assert_eq!(pixel(&b,11,65),[0,0,0,255]);assert_eq!(pixel(&b,15,64),[255,255,255,255]);assert_eq!(pixel(&b,20,65),[255,255,255,255]);
}

#[test]fn math_style_rise_and_spacing_match_explicit_positions(){
    use rpcore::raster::TextQuality;
    let relative=fixture::pdf(b"BT /F1 10 Tf 80 Tz 1 Tc 10 20 Td <0001> Tj 5 Ts /F1 5 Tf <0002> Tj 0 Ts /F1 10 Tf [120 <0001>] TJ ET");
    let absolute=fixture::pdf(b"BT /F1 10 Tf 80 Tz 1 0 0 1 10 20 Tm <0001> Tj /F1 5 Tf 1 0 0 1 16 25 Tm <0002> Tj /F1 10 Tf 1 0 0 1 18.44 20 Tm <0001> Tj ET");
    let a=Document::open(&relative).unwrap();let b=Document::open(&absolute).unwrap();
    let ap=a.pages().unwrap();let bp=b.pages().unwrap();
    let text=a.page_text(&ap[0],10000).unwrap();
    assert_eq!(text.glyphs.len(),3);
    for (g,expected) in text.glyphs.iter().zip([[10.,20.],[16.,25.],[18.44,20.]]){
        for axis in 0..2{assert!((g.origin[axis]-expected[axis]).abs()<1e-9);}
    }
    for q in [TextQuality::Standard,TextQuality::Fine,TextQuality::Area]{
        let render=|d:&Document<'_>,p:&rpcore::PageInfo|d.render_page_with_text_quality(p,Options::default(),10000,None,q).unwrap().bitmap;
        assert_eq!(render(&a,&ap[0]),render(&b,&bp[0]),"{q:?}");
    }
}

#[test]fn outline_geometry_curves_rise_and_empty_glyphs(){
    let p=fixture::pdf(b"BT /F1 20 Tf 10 20 Td <0001> Tj 5 Ts <0002> Tj 0 Ts <0000> Tj 3 Tr <0001> Tj ET");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    let g=d.page_glyph_geometry(&pages[0],10000,None).unwrap();
    assert_eq!(g.glyphs.len(),4);assert!(g.substituted_fonts.is_empty());
    let close=|a:[f64;4],b:[f64;4]|{for i in 0..4{assert!((a[i]-b[i]).abs()<1e-8,"{a:?} != {b:?}");}};
    close(g.glyphs[0].outline_bounds.unwrap(),[10.,20.,22.,34.]);
    close(g.glyphs[1].outline_bounds.unwrap(),[23.,25.,35.,37.]);
    assert_eq!(g.glyphs[1].origin,[23.,25.]);assert_eq!(g.glyphs[1].advance_end,[36.,25.]);
    assert_eq!(g.glyphs[1].unicode.as_deref(),Some("D"));assert_eq!(g.glyphs[1].font_resource,b"F1");
    assert_eq!(g.glyphs[2].outline_bounds,None);assert_eq!(g.glyphs[3].outline_bounds,None);
    assert_eq!(g.glyphs[3].rendering_mode,3);
    assert_eq!(g.glyphs[0].origin,d.page_text(&pages[0],10000).unwrap().glyphs[0].origin);
}
#[test]fn geometry_rotates_outline_not_em_box(){
    let p=fixture::pdf(b"BT /F1 20 Tf 0 1 -1 0 40 20 Tm <0002> Tj ET");
    let d=Document::open(&p).unwrap();let g=d.page_glyph_geometry(&d.pages().unwrap()[0],10000,None).unwrap();
    let b=g.glyphs[0].outline_bounds.unwrap();
    for (v,e) in b.into_iter().zip([28.,20.,40.,32.]){assert!((v-e).abs()<1e-8);}
}

#[test]fn viewport_selection_matches_rendered_glyph(){
    use rpcore::raster::PageViewport;
    let p=fixture::pdf(b"BT /F1 20 Tf 10 20 Td <0001> Tj <0000> Tj 3 Tr <0001> Tj ET");
    let d=Document::open(&p).unwrap();let mut pages=d.pages().unwrap();let page=&mut pages[0];
    page.crop_box=[8.,18.,30.,38.];page.rotation=90;
    let geo=d.page_glyph_geometry(page,10000,None).unwrap();
    let v=PageViewport::new(page,2.).unwrap();
    let b=d.render_page(page,Options{scale:2.,allow_missing_text:false},10000).unwrap();
    assert_eq!(v.bitmap_size(),(b.width,b.height));assert_eq!(v.bitmap_size(),(40,44));
    let rect=geo.highlight_rect(0,&v).unwrap().unwrap();
    for (a,e) in rect.into_iter().zip([4.,4.,32.,28.]){assert!((a-e).abs()<1e-8);}
    assert_eq!(geo.highlight_rect(1,&v).unwrap(),None);
    assert_eq!(geo.highlight_rect(2,&v).unwrap(),None);
    let point=v.pdf_to_view([11.,25.]).unwrap();
    assert_eq!(geo.hit_test(&v,point).unwrap(),Some(0));
    assert_eq!(pixel(&b,point[0]as usize,point[1]as usize),[0,0,0,255]);
    assert_eq!(geo.select_view_rect(&v,[0.,0.,40.,44.]).unwrap(),vec![0]);
    assert_eq!(geo.hit_test(&v,[40.,4.]).unwrap(),None);
    assert!(geo.highlight_rect(3,&v).is_err());
}
#[test]fn viewport_overlap_and_partial_crop(){
    use rpcore::raster::PageViewport;
    let p=fixture::pdf(b"BT /F1 20 Tf 10 20 Td <0001> Tj 1 0 0 1 10 20 Tm <0001> Tj ET");
    let d=Document::open(&p).unwrap();let mut pages=d.pages().unwrap();let page=&mut pages[0];
    page.crop_box=[15.,25.,25.,35.];
    let g=d.page_glyph_geometry(page,10000,None).unwrap();
    let v=PageViewport::new(page,1.).unwrap();
    assert_eq!(g.highlight_rect(0,&v).unwrap(),Some([0.,1.,7.,10.]));
    assert_eq!(g.select_view_rect(&v,[2.,2.,3.,3.]).unwrap(),vec![0,1]);
    assert_eq!(g.hit_test(&v,[2.,2.]).unwrap(),Some(1));
    assert_eq!(g.hit_test(&v,[8.,2.]).unwrap(),None);
    assert!(g.select_view_rect(&v,[3.,3.,2.,2.]).is_err());
}

#[test]fn math_region_scripts_and_fraction_candidates(){
    use rpcore::raster::math_geometry::ScriptKind;
    // Base and two smaller displaced glyphs; separate centered fraction.
    let p=fixture::pdf(b"BT /F1 10 Tf 10 40 Td <0001> Tj /F1 6 Tf 1 0 0 1 16.5 45 Tm <0002> Tj 1 0 0 1 16.5 36 Tm <0003> Tj /F1 8 Tf 1 0 0 1 43.6 48 Tm <0001> Tj 1 0 0 1 43.6 37 Tm <0002> Tj ET 40 44 12 .5 re f");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    let (g,m)=d.page_math_geometry(&pages[0],10000,None,[5.,30.,60.,60.]).unwrap();
    assert_eq!(m.glyph_indices,vec![0,1,2,3,4]);
    assert_eq!(m.scripts.len(),2);
    assert!(m.scripts.iter().any(|s|s.base==0&&s.script==1&&s.kind==ScriptKind::Superscript));
    assert!(m.scripts.iter().any(|s|s.base==0&&s.script==2&&s.kind==ScriptKind::Subscript));
    assert_eq!(m.fractions.len(),1);let f=&m.fractions[0];
    assert_eq!(f.numerator,vec![3]);assert_eq!(f.denominator,vec![4]);assert_eq!(f.rule_bounds,[40.,44.,52.,44.5]);
    assert_eq!(g.glyphs[f.numerator[0]].unicode.as_deref(),Some("O"));
}
#[test]fn math_region_rejects_same_size_scripts_and_unpaired_rules(){
    let p=fixture::pdf(b"BT /F1 10 Tf 10 40 Td <0001> Tj 1 0 0 1 16.5 45 Tm <0002> Tj ET 8 35 30 .5 re f");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    let (_,m)=d.page_math_geometry(&pages[0],10000,None,[0.,20.,60.,70.]).unwrap();
    assert!(m.scripts.is_empty());assert!(m.fractions.is_empty());
    let (_,m)=d.page_math_geometry(&pages[0],10000,None,[0.,20.,15.,70.]).unwrap();
    assert!(m.glyph_indices.is_empty());
    assert!(d.page_math_geometry(&pages[0],10000,None,[5.,0.,4.,10.]).is_err());
}
#[test]fn math_region_horizontal_stroke_fraction(){
    let p=fixture::pdf(b"BT /F1 8 Tf 43.6 48 Td <0001> Tj 1 0 0 1 43.6 37 Tm <0002> Tj ET .5 w 40 44 m 52 44 l S");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    let (_,m)=d.page_math_geometry(&pages[0],10000,None,[35.,30.,60.,60.]).unwrap();
    assert_eq!(m.fractions.len(),1);assert_eq!(m.fractions[0].rule_bounds,[40.,43.75,52.,44.25]);
}

#[test]fn math_tree_disjoint_fraction_and_scripts_export(){
    use rpcore::raster::math_tree::build_tree;
    let p=fixture::pdf(b"BT /F1 10 Tf 10 40 Td <0001> Tj /F1 6 Tf 1 0 0 1 16.5 45 Tm <0002> Tj 1 0 0 1 16.5 36 Tm <0003> Tj /F1 8 Tf 1 0 0 1 43.6 48 Tm <0001> Tj 1 0 0 1 43.6 37 Tm <0002> Tj ET 40 44 12 .5 re f");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    let (mut g,mut m)=d.page_math_geometry(&pages[0],10000,None,[5.,30.,60.,60.]).unwrap();
    let tree=build_tree(&g,&m).unwrap();
    assert_eq!(tree.to_latex(&g).unwrap(),r"{O}_{o}^{D}\frac{O}{D}");
    g.glyphs[1].unicode=None;assert!(tree.to_latex(&g).is_err());
    m.fractions.push(m.fractions[0].clone());assert!(build_tree(&g,&m).is_err());
}
#[test]fn math_tree_rejects_conflicting_or_cyclic_scripts(){
    use rpcore::raster::{math_tree::build_tree,math_geometry::{ScriptCandidate,ScriptKind}};
    let p=fixture::pdf(b"BT /F1 10 Tf 10 40 Td <0001> Tj /F1 6 Tf 1 0 0 1 16.5 45 Tm <0002> Tj 1 0 0 1 16.5 36 Tm <0003> Tj ET");
    let d=Document::open(&p).unwrap();let (g,mut m)=d.page_math_geometry(&d.pages().unwrap()[0],10000,None,[5.,30.,60.,60.]).unwrap();
    m.scripts.push(ScriptCandidate{base:1,script:0,kind:ScriptKind::Superscript});assert!(build_tree(&g,&m).is_err());
    m.scripts.clear();assert!(build_tree(&g,&m).is_err());
}
#[test]fn math_tree_nested_scripts(){
    use rpcore::raster::math_tree::build_tree;
    let p=fixture::pdf(b"BT /F1 12 Tf 10 40 Td <0001> Tj /F1 7 Tf 1 0 0 1 17.8 45 Tm <0002> Tj /F1 4 Tf 1 0 0 1 22.35 49 Tm <0003> Tj ET");
    let d=Document::open(&p).unwrap();let (g,m)=d.page_math_geometry(&d.pages().unwrap()[0],10000,None,[5.,30.,60.,60.]).unwrap();
    assert_eq!(m.scripts.len(),2);
    assert_eq!(build_tree(&g,&m).unwrap().to_latex(&g).unwrap(),"{O}^{{D}^{o}}");
}

#[test]fn math_tree_multi_glyph_script_row(){
    use rpcore::raster::math_tree::build_tree;
    let p=fixture::pdf(b"BT /F1 10 Tf 10 40 Td <0001> Tj /F1 6 Tf 1 0 0 1 16.5 45 Tm <000200030002> Tj ET");
    let d=Document::open(&p).unwrap();
    let (g,m)=d.page_math_geometry(&d.pages().unwrap()[0],10000,None,[5.,30.,60.,60.]).unwrap();
    assert_eq!(m.scripts.len(),3);
    assert!(m.scripts.iter().all(|s|s.base==0));
    assert_eq!(build_tree(&g,&m).unwrap().to_latex(&g).unwrap(),"{O}^{DoD}");
}
#[test]fn math_tree_radical_requires_unicode_and_explicit_bar(){
    use rpcore::raster::math_tree::build_tree;
    // Replace one same-length UTF-16BE CMap value in the synthetic fixture;
    // byte offsets and stream lengths remain unchanged.
    let make=|content:&[u8]|{
        let mut p=fixture::pdf(content);
        let i=p.windows(6).position(|v|v==b"<004f>").unwrap();
        p[i..i+6].copy_from_slice(b"<221a>");p
    };
    // Root fixture uses an O-shaped outline; this tests geometric association
    // with a known Unicode radical, not recognition of an actual radical shape.
    let p=make(b"BT /F1 12 Tf 10 40 Td <0001> Tj /F1 10 Tf 1 0 0 1 18 40 Tm <00020003> Tj ET 17 49 15 .4 re f");
    let d=Document::open(&p).unwrap();
    let (g,mut m)=d.page_math_geometry(&d.pages().unwrap()[0],10000,None,[5.,30.,60.,60.]).unwrap();
    assert_eq!(m.radicals.len(),1);assert_eq!(m.radicals[0].symbol,0);assert_eq!(m.radicals[0].radicand,vec![1,2]);
    assert_eq!(build_tree(&g,&m).unwrap().to_latex(&g).unwrap(),r"\sqrt{Do}");
    m.radicals.push(m.radicals[0].clone());assert!(build_tree(&g,&m).is_err());
    let p=make(b"BT /F1 12 Tf 10 40 Td <0001> Tj /F1 10 Tf 1 0 0 1 18 40 Tm <00020003> Tj ET");
    let d=Document::open(&p).unwrap();
    let (g,m)=d.page_math_geometry(&d.pages().unwrap()[0],10000,None,[5.,30.,60.,60.]).unwrap();
    assert!(m.radicals.is_empty());assert!(build_tree(&g,&m).unwrap().to_latex(&g).is_err());
}

#[test]fn math_tree_nested_fraction_numerator(){
    use rpcore::raster::math_tree::build_tree;
    let p=fixture::pdf(b"BT /F1 6 Tf 50 59 Td <0001> Tj 1 0 0 1 50 50 Tm <0002> Tj /F1 10 Tf 1 0 0 1 49 36 Tm <0003> Tj ET 47 56 10 .4 re f 42 46 20 .5 re f");
    let d=Document::open(&p).unwrap();
    let (g,mut m)=d.page_math_geometry(&d.pages().unwrap()[0],10000,None,[35.,30.,70.,70.]).unwrap();
    assert_eq!(m.fractions.len(),2);
    assert_eq!(m.fractions[1].numerator_fraction,Some(m.fractions[0].rule_command_index));
    assert_eq!(m.fractions[1].numerator,vec![0,1]);
    assert_eq!(build_tree(&g,&m).unwrap().to_latex(&g).unwrap(),r"\frac{\frac{O}{D}}{o}");
    m.fractions[1].numerator.pop();
    assert!(build_tree(&g,&m).is_err());
}
#[test]fn math_tree_nested_fraction_denominator(){
    use rpcore::raster::math_tree::build_tree;
    let p=fixture::pdf(b"BT /F1 10 Tf 49 62 Td <0003> Tj /F1 6 Tf 1 0 0 1 50 50 Tm <0001> Tj 1 0 0 1 50 41 Tm <0002> Tj ET 42 58 20 .5 re f 47 47 10 .4 re f");
    let d=Document::open(&p).unwrap();
    let (g,mut m)=d.page_math_geometry(&d.pages().unwrap()[0],10000,None,[35.,30.,70.,80.]).unwrap();
    assert_eq!(m.fractions.len(),2);
    assert_eq!(m.fractions[1].denominator_fraction,Some(m.fractions[0].rule_command_index));
    assert_eq!(build_tree(&g,&m).unwrap().to_latex(&g).unwrap(),r"\frac{o}{\frac{O}{D}}");
    m.fractions[0].numerator_fraction=Some(m.fractions[1].rule_command_index);
    assert!(build_tree(&g,&m).is_err());
}

#[test]fn math_shallow_tex_subscript_with_advance_alignment(){
    use rpcore::raster::{math_tree::build_tree,math_geometry::ScriptKind};
    let p=fixture::pdf(b"BT /F1 10 Tf 10 40 Td <0001> Tj /F1 7 Tf 1 0 0 1 16.5 38.506 Tm <0002> Tj ET");
    let d=Document::open(&p).unwrap();
    let (g,m)=d.page_math_geometry(&d.pages().unwrap()[0],10000,None,[5.,30.,60.,60.]).unwrap();
    assert_eq!(m.scripts.len(),1);assert_eq!(m.scripts[0].kind,ScriptKind::Subscript);
    assert_eq!(build_tree(&g,&m).unwrap().to_latex(&g).unwrap(),"{O}_{D}");
    // Shallow displacement with a large gap is not a script.
    let p=fixture::pdf(b"BT /F1 10 Tf 10 40 Td <0001> Tj /F1 7 Tf 1 0 0 1 19 38.506 Tm <0002> Tj ET");
    let d=Document::open(&p).unwrap();
    let (_,m)=d.page_math_geometry(&d.pages().unwrap()[0],10000,None,[5.,30.,60.,60.]).unwrap();
    assert!(m.scripts.is_empty());
}
#[test]fn math_fraction_inherits_raised_script(){
    use rpcore::raster::math_tree::build_tree;
    // Numerator base baseline 48, superscript 52. A centered rule's
    // nearest-baseline row initially includes the base but not its script.
    let p=fixture::pdf(b"BT /F1 10 Tf 43 48 Td <0001> Tj /F1 7 Tf 1 0 0 1 49.5 52 Tm <0002> Tj /F1 10 Tf 1 0 0 1 46 35 Tm <0003> Tj ET 40 44 17 .5 re f");
    let d=Document::open(&p).unwrap();
    let (g,m)=d.page_math_geometry(&d.pages().unwrap()[0],10000,None,[35.,30.,60.,70.]).unwrap();
    assert_eq!(m.fractions.len(),1);assert_eq!(m.fractions[0].numerator,vec![0,1]);
    assert_eq!(m.fractions[0].denominator,vec![2]);
    assert_eq!(build_tree(&g,&m).unwrap().to_latex(&g).unwrap(),r"\frac{{O}^{D}}{o}");
}

#[test]fn math_radical_with_script(){
    use rpcore::raster::math_tree::build_tree;
    let mut p=fixture::pdf(b"BT /F1 12 Tf 10 40 Td <0001> Tj /F1 10 Tf 1 0 0 1 18 40 Tm <0002> Tj /F1 7 Tf 1 0 0 1 24.5 38.5 Tm <0003> Tj ET 17 49 15 .4 re f");
    let i=p.windows(6).position(|v|v==b"<004f>").unwrap();
    p[i..i+6].copy_from_slice(b"<221a>");
    let d=Document::open(&p).unwrap();
    let (g,m)=d.page_math_geometry(&d.pages().unwrap()[0],10000,None,[5.,30.,60.,60.]).unwrap();
    assert_eq!(m.radicals.len(),1);
    assert_eq!(m.radicals[0].radicand,vec![1,2]);
    assert_eq!(build_tree(&g,&m).unwrap().to_latex(&g).unwrap(),r"\sqrt{{D}_{o}}");
}
