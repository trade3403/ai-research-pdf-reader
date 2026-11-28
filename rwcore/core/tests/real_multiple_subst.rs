//! Independently extracted direct GSUB type 2 reference from original SFNT bytes.
use rwcore::*;
type Pair = (u16,&'static [u16]);
type Case = (usize,usize,usize,&'static [Pair]);
const CASES:&[Case]=&[
    (0,0,860,&[(7,&[12,51]),(8,&[12,52]),(9,&[40,52]),(10,&[12,53]),(11,&[41,52]),(13,&[69,124]),(14,&[39,122]),(15,&[69,122]),(16,&[69,123]),(17,&[18,124]),(19,&[18,121]),(21,&[20,121]),(23,&[22,121]),(25,&[24,123]),(27,&[26,121]),(29,&[28,121]),(31,&[30,121]),(33,&[83,121]),(42,&[41,125]),(72,&[12,422]),(74,&[69,421]),(75,&[69,126]),(76,&[18,126]),(77,&[20,421]),(78,&[20,123]),(79,&[22,421]),(80,&[22,465]),(81,&[22,123]),(82,&[30,123]),(84,&[83,123]),(86,&[35,121]),(87,&[85,127]),(88,&[36,57]),(90,&[89,421]),(93,&[39,52]),(95,&[40,57]),(97,&[96,57]),(98,&[40,121]),(100,&[99,52]),(102,&[39]),(108,&[85,121])]),
    (1,0,1282,&[(91,&[89,123]),(94,&[39]),(96,&[41])]),
    (2,0,1326,&[(18,&[135]),(24,&[140]),(26,&[143]),(28,&[146]),(30,&[149]),(34,&[152,122]),(35,&[156]),(36,&[159]),(37,&[162]),(38,&[132,121]),(39,&[166]),(41,&[132]),(69,&[132]),(70,&[152]),(83,&[152]),(85,&[156]),(89,&[132]),(91,&[132,126]),(92,&[166]),(94,&[176]),(96,&[132,125])]),
    (3,0,1454,&[(18,&[136]),(24,&[141]),(26,&[144]),(28,&[147]),(30,&[150]),(34,&[153,122]),(35,&[157]),(36,&[160]),(37,&[163]),(38,&[133,121]),(39,&[168]),(41,&[133]),(69,&[133]),(70,&[153]),(83,&[153]),(85,&[157]),(89,&[133]),(91,&[133,126]),(92,&[167]),(94,&[168]),(96,&[133,125])]),
    (4,0,1628,&[(12,&[130]),(18,&[134]),(20,&[137]),(22,&[138]),(24,&[139]),(26,&[142]),(28,&[145]),(30,&[148]),(34,&[154]),(35,&[155]),(36,&[158]),(37,&[161]),(38,&[164]),(39,&[165]),(40,&[169]),(41,&[170]),(69,&[131]),(70,&[432]),(83,&[151]),(85,&[173]),(89,&[174]),(91,&[174,123]),(92,&[177]),(94,&[175]),(96,&[170]),(99,&[178])]),
    (69,0,5010,&[(24,&[404]),(26,&[405]),(34,&[407,122]),(35,&[428]),(36,&[408]),(38,&[409,121]),(41,&[410]),(69,&[403]),(70,&[407]),(83,&[406]),(85,&[411]),(89,&[409]),(131,&[412]),(139,&[413]),(142,&[414]),(151,&[415]),(154,&[416,122]),(155,&[429]),(158,&[417]),(164,&[409,121]),(170,&[418]),(173,&[420]),(174,&[409]),(398,&[419]),(432,&[416])]),
];
const FONT:&[u8]=include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf");
fn with_rules(mut check:impl FnMut(&MultipleSubstitution,&[Pair])){
    let c=parse_font(FONT,FontLimits::default()).unwrap();
    let bytes=c.face(0).unwrap().table(*b"GSUB").unwrap();
    let layout=parse_gsub_layout(bytes,GsubLimits::default()).unwrap();
    assert_eq!(layout.lookups.len(),86);
    let mut seen=0;
    for (i,lookup) in layout.lookups.iter().enumerate(){
        if lookup.kind!=2{continue;}
        for (j,&offset) in lookup.subtables.iter().enumerate(){
            let &(ei,ej,eo,pairs)=&CASES[seen];
            assert_eq!((i,j,offset),(ei,ej,eo));
            let r=parse_multiple_substitution(&bytes[offset..],739,MultipleSubstLimits::default()).unwrap();
            assert_eq!(r.len(),pairs.len());
            check(&r,pairs);
            seen+=1;
        }
    }
    assert_eq!(seen,CASES.len());
}
#[test]fn real_direct_type_two_full_domain(){
    with_rules(|r,pairs|{
        for id in 0..739u16{
            let expected=pairs.binary_search_by_key(&id,|p|p.0).ok().map(|i|pairs[i].1);
            assert_eq!(r.substitute(id),expected,"glyph {id}");
        }
    });
}
#[test]fn real_expansion_and_atomic_limit(){
    with_rules(|r,pairs|{
        for &(id,seq) in pairs{
            let mut run=vec![0,id,1];
            let old=run.clone();
            if seq.len()>1{
                assert!(r.replace_at(&mut run,1,2+seq.len()-1).is_err());
                assert_eq!(run,old);
            }
            assert_eq!(r.replace_at(&mut run,1,2+seq.len()).unwrap(),Some(seq.len()));
            let mut expected=vec![0];expected.extend_from_slice(seq);expected.push(1);
            assert_eq!(run,expected);
        }
    });
}
#[test]fn latin_fonts_have_no_direct_type_two(){
    for font in [include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf").as_slice(),
                 include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf").as_slice()]{
        let c=parse_font(font,FontLimits::default()).unwrap();
        let b=c.face(0).unwrap().table(*b"GSUB").unwrap();
        let g=parse_gsub_layout(b,GsubLimits::default()).unwrap();
        assert!(!g.lookups.iter().any(|l|l.kind==2));
    }
}
