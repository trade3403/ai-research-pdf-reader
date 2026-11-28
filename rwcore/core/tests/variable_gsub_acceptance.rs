
use rwcore::*;
const FONTS: [&[u8]; 3] = [
    include_bytes!("fixtures/archivo/Archivo[wdth,wght].ttf"),
    include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf"),
    include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf"),
];
// Independent raw fvar instance reader. The fixture is immutable.
fn raw_instances(b: &[u8]) -> Vec<Vec<i32>> {
    let u = |p| u16::from_be_bytes([b[p], b[p+1]]) as usize;
    let n = u(8);
    let start = u(4) + n*u(10);
    (0..u(12)).map(|i| {
        let p = start+i*u(14)+4;
        (0..n).map(|j| i32::from_be_bytes(b[p+4*j..p+4*j+4].try_into().unwrap())).collect()
    }).collect()
}
fn check_instances(font: &[u8]) {
    let c = parse_font(font, Default::default()).unwrap();
    let sf = c.face(0).unwrap();
    let f = parse_variable_gsub_face(sf, Default::default()).unwrap();
    let raw = raw_instances(sf.table(*b"fvar").unwrap());
    assert_eq!(raw.len(), f.fvar().instances().len());
    assert!(!raw.is_empty());
    let layout = parse_gsub_layout(sf.table(*b"GSUB").unwrap(), Default::default()).unwrap();
    let script = layout.scripts.iter().find(|s| s.default_language.is_some()).unwrap().tag;
    for (user, instance) in raw.iter().zip(f.fvar().instances()) {
        assert_eq!(*user, instance.coordinates);
        let tagged: Vec<_> = f.fvar().axes().iter().zip(user).rev().map(|(a,v)| (a.tag,*v)).collect();
        let req = VariableGsubRequest { script, language: None, enabled: &[],
            coordinates: VariationUserCoordinates::Ordered(user) };
        let other = VariableGsubRequest { coordinates: VariationUserCoordinates::Tagged(&tagged), ..req };
        let p = f.build_plan(req).unwrap().unwrap();
        assert_eq!(f.build_plan(other).unwrap().unwrap(), p);
        let input = [0u16, 0, 0];
        let a = f.execute(req, &input, &[]).unwrap().unwrap();
        assert_eq!(a, f.execute(other, &input, &[]).unwrap().unwrap());
        let program = f.compile_plan(other, &p).unwrap();
        assert_eq!(a.execution, execute_gsub_program(&program, &input, &[], Default::default()).unwrap());
        assert_eq!(input, [0,0,0]);
    }
}
#[test] fn archivo_named_instances() { check_instances(FONTS[0]); }
#[test] fn devanagari_named_instances() { check_instances(FONTS[1]); }
#[test] fn nastaliq_named_instances() { check_instances(FONTS[2]); }
#[test] fn default_coordinates_and_order_independence() {
    for font in FONTS {
        let c = parse_font(font, Default::default()).unwrap();
        let f = parse_variable_gsub_face(c.face(0).unwrap(), Default::default()).unwrap();
        let values: Vec<_> = f.fvar().axes().iter().map(|a| a.default).collect();
        assert_eq!(f.normalize(VariationUserCoordinates::Ordered(&values)).unwrap(), vec![0;values.len()]);
        assert_eq!(f.normalize(VariationUserCoordinates::Tagged(&[])).unwrap(), vec![0;values.len()]);
        for a in f.fvar().axes() {
            assert_eq!(f.normalize(VariationUserCoordinates::Tagged(&[(a.tag,a.default)])).unwrap(), vec![0;values.len()]);
        }
    }
}
#[test] fn archivo_repeat_and_feature_provenance() {
    let c = parse_font(FONTS[0], Default::default()).unwrap();
    let f = parse_variable_gsub_face(c.face(0).unwrap(), Default::default()).unwrap();
    let req = VariableGsubRequest { script: *b"latn", language: None, enabled: &[*b"rvrn"],
        coordinates: VariationUserCoordinates::Tagged(&[]) };
    let input: Vec<_> = (0..834u16).collect();
    let result = f.execute(req,&input,&[]).unwrap().unwrap();
    for _ in 0..4 { assert_eq!(result, f.execute(req,&input,&[]).unwrap().unwrap()); }
    let mut bad = result.plan.clone();
    bad.plan.unavailable_tags.push(*b"zzzz");
    assert!(f.compile_plan(req,&bad).is_err());
    assert!(f.compile_plan(VariableGsubRequest { enabled: &[*b"case",*b"rvrn"], ..req }, &result.plan).is_err());
}
