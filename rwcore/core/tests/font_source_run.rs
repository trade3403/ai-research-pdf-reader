use rwcore::*;
const FONT:&[u8]=include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf");
fn with_cmap(f:impl FnOnce(&UnicodeCmap<'_>)){
    let c=parse_font(FONT,Default::default()).unwrap();
    let cmap=parse_unicode_cmap(c.face(0).unwrap(),Default::default()).unwrap();
    f(&cmap);
}
#[test]fn source_empty(){with_cmap(|c|{
    let r=map_source_run("",c,false,SourceLimits{max_bytes:0,max_scalars:0,max_glyphs:0}).unwrap();
    assert!(r.scalars().is_empty());assert!(r.glyphs().is_empty());
    assert_eq!(r.byte_range(0,0).unwrap(),0..0);assert_eq!(r.scalar_at_byte(0),None);
});}
#[test]fn source_utf8_boundaries(){with_cmap(|c|{
    let r=map_source_run("Aé中😀",c,true,Default::default()).unwrap();
    assert_eq!(r.byte_len(),10);
    assert_eq!(r.scalars().iter().map(|s|(s.byte_start,s.byte_end)).collect::<Vec<_>>(),vec![(0,1),(1,3),(3,6),(6,10)]);
    assert_eq!((0..11).map(|i|r.scalar_at_byte(i)).collect::<Vec<_>>(),vec![Some(0),Some(1),Some(1),Some(2),Some(2),Some(2),Some(3),Some(3),Some(3),Some(3),None]);
    assert_eq!(r.byte_range(1,3).unwrap(),1..6);
});}
#[test]fn source_missing(){with_cmap(|c|{
    let r=map_source_run("\u{10ffff}",c,true,Default::default()).unwrap();
    assert_eq!(r.glyphs(),&[GlyphSource{glyph_id:0,scalar_index:0,missing:true}]);
    assert!(map_source_run("\u{10ffff}",c,false,Default::default()).is_err());
});}
#[test]fn source_known_mapping(){with_cmap(|c|{
    let r=map_source_run("ABA",c,false,Default::default()).unwrap();
    assert_eq!(r.glyphs()[0].glyph_id,c.glyph_for_char('A').unwrap().unwrap());
    assert_eq!(r.glyphs()[0].glyph_id,r.glyphs()[2].glyph_id);
    assert_eq!(r.glyphs()[2].scalar_index,2);
});}
#[test]fn source_exact_limits(){with_cmap(|c|{
    let l=SourceLimits{max_bytes:3,max_scalars:2,max_glyphs:2};
    assert!(map_source_run("Aé",c,true,l).is_ok());
    for bad in [SourceLimits{max_bytes:2,..l},SourceLimits{max_scalars:1,..l},SourceLimits{max_glyphs:1,..l}]{
        assert!(matches!(map_source_run("Aé",c,true,bad),Err(Error::Limit(_))));
    }
});}
#[test]fn source_no_normalization_or_grapheme_claim(){with_cmap(|c|{
    let r=map_source_run("e\u{301}\u{200d}😀",c,true,Default::default()).unwrap();
    assert_eq!(r.scalars().iter().map(|s|s.character).collect::<String>(),"e\u{301}\u{200d}😀");
    assert_eq!(r.glyphs().len(),4);
    assert_eq!(r.byte_range(4,4).unwrap(),r.byte_len()..r.byte_len());
    assert!(r.byte_range(3,2).is_err());assert!(r.byte_range(0,5).is_err());
    assert_eq!(r.scalar_at_byte(usize::MAX),None);
});}
