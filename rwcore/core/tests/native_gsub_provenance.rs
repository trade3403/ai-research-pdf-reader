use rwcore::*;
const ARCHIVO:&[u8]=include_bytes!("fixtures/archivo/Archivo[wdth,wght].ttf");
fn check_native(font:&[u8],text:&str,script:[u8;4]){
    let c=parse_font(font,Default::default()).unwrap();let sf=c.face(0).unwrap();
    let cmap=parse_unicode_cmap(sf,Default::default()).unwrap();
    let run=map_source_run(text,&cmap,true,Default::default()).unwrap();
    let f=parse_variable_gsub_face(sf,Default::default()).unwrap();
    let req=VariableGsubRequest{script,language:None,enabled:&[],coordinates:VariationUserCoordinates::Tagged(&[])};
    let input:Vec<_>=run.glyphs().iter().map(|g|g.glyph_id).collect();
    let (result,l)=f.execute_with_provenance(req,&run,&[],Default::default()).unwrap().unwrap();
    assert_eq!(result,f.execute(req,&input,&[]).unwrap().unwrap());
    let mut seen=vec![false;run.scalars().len()];
    for g in l.glyphs(){
        assert!(!g.sources().is_empty());
        assert!(g.sources().windows(2).all(|p|p[0]<p[1]));
        for &i in g.sources(){seen[i]=true;}
    }
    assert!(seen.iter().all(|x|*x));
    assert_eq!(l.reference_count(),l.glyphs().iter().map(|g|g.sources().len()).sum::<usize>());
    assert_eq!((result,l),f.execute_with_provenance(req,&run,&[],Default::default()).unwrap().unwrap());
}
#[test]fn archivo_native_default_sources(){check_native(ARCHIVO,"Aé office",*b"latn");}
#[test]fn devanagari_native_default_sources(){check_native(include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf"),"नमस्ते",*b"deva");}
#[test]fn nastaliq_native_default_sources(){check_native(include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf"),"سلام",*b"arab");}
#[test]fn archivo_native_rvrn_scalar_reference(){
    let c=parse_font(ARCHIVO,Default::default()).unwrap();let sf=c.face(0).unwrap();
    let cmap=parse_unicode_cmap(sf,Default::default()).unwrap();
    let f=parse_variable_gsub_face(sf,Default::default()).unwrap();
    // Enumerate BMP mapped characters so the reference includes actual
    // rvrn-eligible glyphs when exposed by cmap. Do not infer cluster spans.
    let mut text=String::new();
    for code in 0..=0xffff{
        if let Some(ch)=char::from_u32(code){if cmap.glyph_for_char(ch).unwrap().is_some(){text.push(ch);}}
    }
    let run=map_source_run(&text,&cmap,false,Default::default()).unwrap();
    let input:Vec<_>=run.glyphs().iter().map(|g|g.glyph_id).collect();
    let axis=f.fvar().axis(*b"wght").unwrap();
    for weight in [axis.min,axis.default,axis.max]{
        let tags=[(*b"wght",weight)];
        let req=VariableGsubRequest{script:*b"latn",language:None,enabled:&[*b"rvrn"],coordinates:VariationUserCoordinates::Tagged(&tags)};
        let (result,l)=f.execute_with_provenance(req,&run,&[],Default::default()).unwrap().unwrap();
        assert_eq!(result,f.execute(req,&input,&[]).unwrap().unwrap());
        assert_eq!(l.glyphs().len(),input.len());
        for (i,g) in l.glyphs().iter().enumerate(){assert_eq!(g.sources(),&[i]);}
    }
}
#[test]fn variable_source_budget_failure(){
    let c=parse_font(ARCHIVO,Default::default()).unwrap();let sf=c.face(0).unwrap();
    let cmap=parse_unicode_cmap(sf,Default::default()).unwrap();
    let run=map_source_run("AB",&cmap,false,Default::default()).unwrap();
    let f=parse_variable_gsub_face(sf,Default::default()).unwrap();
    let req=VariableGsubRequest{script:*b"latn",language:None,enabled:&[*b"rvrn"],coordinates:VariationUserCoordinates::Tagged(&[])};
    assert!(matches!(f.execute_with_provenance(req,&run,&[],ProvenanceLimits{max_refs:1,..Default::default()}),Err(Error::Limit(_))));
}
