use rwcore::*;
use IndicProbeKind as K;
use IndicFeatureEvidence as E;
const FONT:&[u8]=include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf");
fn candidates(t:&str)->Vec<IndicFeatureCandidate>{
    devanagari_feature_candidates(t,&bundled_indic_properties().unwrap(),&bundled_script_properties().unwrap(),Default::default()).unwrap().candidates
}
#[test]fn indic_probe_candidate_ranges(){
    let c=candidates("र्क्ति क्षि");
    assert_eq!(c,vec![
        IndicFeatureCandidate{kind:K::Reph,scalar_start:0,scalar_end:2,base_candidate:4},
        IndicFeatureCandidate{kind:K::Half,scalar_start:2,scalar_end:4,base_candidate:4},
        IndicFeatureCandidate{kind:K::Half,scalar_start:7,scalar_end:9,base_candidate:9}]);
}
#[test]fn indic_probe_controls_and_terminal_deferred(){
    for t in ["र्\u{200d}क","र्\u{200c}क","क्त्","ि","कि","अ","A"]{assert!(candidates(t).is_empty(),"{t}");}
    let c=candidates("क़्त");assert_eq!(c[0].scalar_start,0);assert_eq!(c[0].scalar_end,3);assert_eq!(c[0].base_candidate,3);
}
fn fixture(tag:[u8;4],kind:u16,sub:&[u16])->Vec<u8>{
    let mut b=vec![0,1,0,0,0,0,0,0,0,0];
    fn w(b:&mut Vec<u8>,v:u16){b.extend(v.to_be_bytes());}
    fn set(b:&mut [u8],at:usize,v:usize){b[at..at+2].copy_from_slice(&(v as u16).to_be_bytes());}
    let at=b.len();set(&mut b,4,at);w(&mut b,1);b.extend(b"dev2");w(&mut b,8);
    for v in [4,0,0,0xffff,1,0]{w(&mut b,v);}
    let at=b.len();set(&mut b,6,at);w(&mut b,1);b.extend(tag);w(&mut b,8);
    for v in [0,1,0]{w(&mut b,v);}
    let at=b.len();set(&mut b,8,at);for v in [1,4,kind,0,1,8]{w(&mut b,v);}
    for &v in sub{w(&mut b,v);}b
}
fn source(t:&str)->(SourceRun,u32){
    let f=parse_font(FONT,Default::default()).unwrap();let c=parse_unicode_cmap(f.face(0).unwrap(),Default::default()).unwrap();
    (map_source_run(t,&c,false,Default::default()).unwrap(),u32::from(c.glyph_count()))
}
fn probe(b:&[u8],tag:[u8;4],r:&SourceRun,n:u32,t:&str,kind:K,i:usize,l:IndicProbeLimits)->Result<IndicProbeResult,Error>{
    let layout=parse_gsub_layout(b,Default::default())?;
    let plan=build_gsub_base_plan(&layout,*b"dev2",None,&[tag],Default::default())?.unwrap();
    let program=compile_gsub_plan_program(b,&plan,n,None,Default::default())?;
    probe_devanagari_feature(&layout,&program,r,t,&bundled_indic_properties()?,&bundled_script_properties()?,kind,i,l)
}
#[test]fn indic_probe_reph_actual_contraction_and_isolation(){
    let t="र्क र्क";let (r,n)=source(t);let a=r.glyphs()[0].glyph_id;let h=r.glyphs()[1].glyph_id;
    let b=fixture(*b"rphf",4,&[1,8,1,14,1,1,a,1,4,20,2,h]);
    let x=probe(&b,*b"rphf",&r,n,t,K::Reph,0,Default::default()).unwrap();
    assert_eq!(x.evidence,E::WholeRangeContraction);assert_eq!(x.masks.scalar_masks(),&[1,1,0,0,0,0,0]);
    assert_eq!(x.result.provenance.glyphs()[0].sources(),&[0,1]);
    assert_eq!(x.result.execution.expansion.execution.glyphs[0],20);
    assert_eq!(x.result.provenance.glyphs()[3].sources(),&[4]);
    assert_eq!(x.result.provenance.glyphs()[3].glyph_id,a);
}
#[test]fn indic_probe_half_actual_single_change_and_no_match(){
    let t="क्त क्त";let (r,n)=source(t);let a=r.glyphs()[0].glyph_id;
    let z=if a==20{21}else{20};
    let b=fixture(*b"half",1,&[2,8,1,z,1,1,a]);
    let x=probe(&b,*b"half",&r,n,t,K::Half,1,Default::default()).unwrap();
    assert_eq!(x.evidence,E::Changed);assert_eq!(x.masks.scalar_masks(),&[0,0,0,0,1,1,0]);
    assert_eq!(x.result.execution.expansion.execution.glyphs[0],a);
    assert_eq!(x.result.execution.expansion.execution.glyphs[4],z);
    let b=fixture(*b"half",1,&[2,8,1,a,1,1,a]);
    assert_eq!(probe(&b,*b"half",&r,n,t,K::Half,0,Default::default()).unwrap().evidence,E::Unchanged);
}
#[test]fn indic_probe_invalid_and_budget(){
    let t="क्त";let (r,n)=source(t);let a=r.glyphs()[0].glyph_id;
    let b=fixture(*b"half",1,&[2,8,1,20,1,1,a]);
    assert!(matches!(probe(&b,*b"half",&r,n,t,K::Reph,0,Default::default()),Err(Error::Invalid(_))));
    assert!(matches!(probe(&b,*b"half",&r,n,t,K::Half,1,Default::default()),Err(Error::Invalid(_))));
    assert!(matches!(probe(&b,*b"half",&r,n,"ख्त",K::Half,0,Default::default()),Err(Error::Invalid(_))));
    let bad=fixture(*b"liga",1,&[2,8,1,20,1,1,a]);
    assert!(matches!(probe(&bad,*b"liga",&r,n,t,K::Half,0,Default::default()),Err(Error::Unsupported(_))));
    assert!(matches!(probe(&b,*b"half",&r,n,t,K::Half,0,IndicProbeLimits{max_evidence_work:8,..Default::default()}),Err(Error::Limit(_))));
    assert!(probe(&b,*b"half",&r,n,t,K::Half,0,IndicProbeLimits{max_evidence_work:9,..Default::default()}).is_ok());
}
#[test]fn indic_probe_retained_native_dev2(){
    let f=parse_font(FONT,Default::default()).unwrap();let face=f.face(0).unwrap();
    let p=bundled_indic_properties().unwrap();let s=bundled_script_properties().unwrap();
    let mut report=String::new();
    for (t,k) in [("र्क",K::Reph),("क्त",K::Half)]{
        let x=probe_font_face_devanagari(face,t,&p,&s,*b"dev2",None,k,0,Default::default(),Default::default()).unwrap();
        assert_eq!(x.candidate.kind,k);assert_eq!(x.masks.scalar_masks(),&[1,1,0]);
        assert_eq!(x.result.provenance.glyphs().last().unwrap().sources(),&[2]);
        report.push_str(&format!("INDIC_NATIVE_PROBE {t} {k:?} {:?} ids={:?}\n",x.evidence,x.result.execution.expansion.execution.glyphs));
    }
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/indic-native-probe.txt");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(path,report).unwrap();
}

fn placement(b:&[u8],t:&str)->Result<(IndicProbeResult,IndicRephPlacement),Error>{
    let (r,n)=source(t);let layout=parse_gsub_layout(b,Default::default())?;
    let plan=build_gsub_base_plan(&layout,*b"dev2",None,&[*b"rphf"],Default::default())?.unwrap();
    let program=compile_gsub_plan_program(b,&plan,n,None,Default::default())?;
    plan_isolated_reph_after_base(&layout,&program,&r,t,&bundled_indic_properties()?,&bundled_script_properties()?,Default::default())
}
#[test]fn indic_reph_placement_actual_prefix_and_base(){
    let (r,_)=source("र्क");let a=r.glyphs()[0].glyph_id;let h=r.glyphs()[1].glyph_id;
    let b=fixture(*b"rphf",4,&[1,8,1,14,1,1,a,1,4,20,2,h]);
    let (x,p)=placement(&b,"र्क").unwrap();
    assert_eq!(p.order,vec![1,0]);assert_eq!(p.inverse,vec![1,0]);
    assert_eq!((p.reph_glyph,p.base_glyph),(0,1));
    let mut ledger=x.result.provenance;let before=ledger.glyphs().to_vec();
    ledger.reorder(&p.order).unwrap();assert_eq!(ledger.glyphs(),&[before[1].clone(),before[0].clone()]);
    assert_eq!(ledger.glyphs()[0].sources(),&[2]);assert_eq!(ledger.glyphs()[1].sources(),&[0,1]);
}
#[test]fn indic_reph_placement_reject_noncontraction_and_complex(){
    let (r,_)=source("र्क");let a=r.glyphs()[0].glyph_id;let h=r.glyphs()[1].glyph_id;
    let single=fixture(*b"rphf",1,&[2,8,1,20,1,1,a]);
    assert!(matches!(placement(&single,"र्क"),Err(Error::Unsupported(_))));
    let b=fixture(*b"rphf",4,&[1,8,1,14,1,1,a,1,4,20,2,h]);
    for t in ["र्कि","र्क्त","र्क र्क"]{
        assert!(matches!(placement(&b,t),Err(Error::Unsupported(_))),"{t}");
    }
}
#[test]fn indic_native_reph_placement_evidence_gate(){
    let f=parse_font(FONT,Default::default()).unwrap();let face=f.face(0).unwrap();
    let p=bundled_indic_properties().unwrap();let s=bundled_script_properties().unwrap();
    let x=probe_font_face_devanagari(face,"र्क",&p,&s,*b"dev2",None,K::Reph,0,Default::default(),Default::default()).unwrap();
    let y=plan_font_face_isolated_reph(face,"र्क",&p,&s,*b"dev2",None,Default::default(),Default::default());
    if x.evidence==E::WholeRangeContraction{
        let (result,placement)=y.unwrap();
        assert_eq!(placement.order,vec![1,0]);
        assert_eq!(result.result.execution,x.result.execution);
    }else{assert!(matches!(y,Err(Error::Unsupported(_))));}
}
