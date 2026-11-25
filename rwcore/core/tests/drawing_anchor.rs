use rwcore::{parse_floating_picture,parse_inline_picture,Limits,RelativeFrom,PositionValue,AnchorAlignment,AnchorWrap,WrapSide};

const ANCHOR:&str=r#"<wp:anchor xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" distT="10" distL="0" simplePos="0" relativeHeight="42" behindDoc="false" locked="1" layoutInCell="true" allowOverlap="0">
<wp:simplePos x="-1" y="2"/><wp:positionH relativeFrom="column"><wp:posOffset>-635</wp:posOffset></wp:positionH><wp:positionV relativeFrom="paragraph"><wp:align>center</wp:align></wp:positionV><wp:extent cx="6350" cy="12700"/><wp:wrapSquare wrapText="bothSides" distL="20"/><wp:docPr id="1" name="figure"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="2" name="image"/></pic:nvPicPr><pic:blipFill><a:blip r:embed="rId1"/></pic:blipFill></pic:pic></a:graphicData></a:graphic></wp:anchor>"#;
fn parse(s:&str)->Result<rwcore::FloatingPicture,rwcore::Error>{parse_floating_picture(s.as_bytes(),Limits::default())}
#[test]fn parses_payload_positions_and_flags(){
    let p=parse(ANCHOR).unwrap();
    assert_eq!(p.picture.extent.twips().unwrap(),(10,20));
    assert_eq!(p.picture.embed.as_deref(),Some("rId1"));
    assert_eq!(p.horizontal.relative_from,RelativeFrom::Column);
    assert_eq!(p.horizontal.value,PositionValue::Offset(-635));
    assert_eq!(p.vertical.relative_from,RelativeFrom::Paragraph);
    assert_eq!(p.vertical.value,PositionValue::Align(AnchorAlignment::Center));
    assert_eq!(p.relative_height,42);
    assert!(p.locked&&p.layout_in_cell&&!p.behind_doc&&!p.allow_overlap&&!p.simple_position_enabled);
}
#[test]fn distances_keep_absent_and_zero_distinct(){
    let p=parse(ANCHOR).unwrap();
    assert_eq!(p.distances.top,Some(10));assert_eq!(p.distances.left,Some(0));
    assert_eq!(p.distances.right,None);assert_eq!(p.wrap_distances.left,Some(20));
}
#[test]fn simple_position_and_hidden(){
    let s=ANCHOR.replace("simplePos=\"0\"","simplePos=\"true\" hidden=\"1\"");
    let p=parse(&s).unwrap();
    assert!(p.simple_position_enabled);assert_eq!((p.simple_x,p.simple_y),(-1,2));assert_eq!(p.hidden,Some(true));
    assert!(parse(&s.replace("x=\"-1\"","x=\"27273042316901\"")).is_err());
}
#[test]fn wrap_types_and_sides(){
    for (xml,expected) in [
        ("<wp:wrapNone/>",AnchorWrap::None),("<wp:wrapTopAndBottom/>",AnchorWrap::TopAndBottom),
        ("<wp:wrapSquare wrapText=\"left\"/>",AnchorWrap::Square(WrapSide::Left)),
        ("<wp:wrapTight wrapText=\"right\"/>",AnchorWrap::Tight(WrapSide::Right)),
        ("<wp:wrapThrough wrapText=\"largest\"/>",AnchorWrap::Through(WrapSide::Largest)),
    ]{assert_eq!(parse(&ANCHOR.replace("<wp:wrapSquare wrapText=\"bothSides\" distL=\"20\"/>",xml)).unwrap().wrap,expected);}
    assert!(parse(&ANCHOR.replace("bothSides","bad")).is_err());
}
#[test]fn rejects_wrong_axis_and_duplicate_position(){
    for s in [ANCHOR.replace("relativeFrom=\"column\"","relativeFrom=\"line\""),
        ANCHOR.replace(">center<",">left<"),
        ANCHOR.replace("</wp:positionH>","<wp:align>left</wp:align></wp:positionH>"),
        ANCHOR.replace("</wp:positionH>","</wp:positionH><wp:positionH relativeFrom=\"page\"><wp:align>left</wp:align></wp:positionH>")]{
        assert!(parse(&s).is_err());
    }
}
#[test]fn rejects_bad_bounds_and_flags(){
    for s in [ANCHOR.replace("-635","2147483648"),ANCHOR.replace("distT=\"10\"","distT=\"-1\""),
        ANCHOR.replace("relativeHeight=\"42\"","relativeHeight=\"4294967296\""),
        ANCHOR.replace("locked=\"1\"","locked=\"yes\""),ANCHOR.replace(" locked=\"1\"","")]{
        assert!(parse(&s).is_err());
    }
}
#[test]fn rejects_missing_duplicate_wrap_and_payload(){
    for s in [ANCHOR.replace("<wp:wrapSquare wrapText=\"bothSides\" distL=\"20\"/>",""),
        ANCHOR.replace("<wp:docPr","<wp:wrapNone/><wp:docPr"),
        ANCHOR.replace("cx=\"6350\"","cx=\"0\""),
        ANCHOR.replace(" r:embed=\"rId1\"","")]{
        assert!(parse(&s).is_err());
    }
}
#[test]fn wrapper_and_strict_namespaces(){
    let w=format!("<w:drawing xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">{ANCHOR}</w:drawing>");
    assert!(parse(&w).is_ok());
    let strict=w.replace("http://schemas.openxmlformats.org/wordprocessingml/2006/main","http://purl.oclc.org/ooxml/wordprocessingml/main")
        .replace("http://schemas.openxmlformats.org/drawingml/2006/","http://purl.oclc.org/ooxml/drawingml/")
        .replace("http://schemas.openxmlformats.org/officeDocument/2006/relationships","http://purl.oclc.org/ooxml/officeDocument/relationships");
    assert!(parse(&strict).is_ok());
    assert!(parse(&strict.replace("http://purl.oclc.org/ooxml/drawingml/picture","http://schemas.openxmlformats.org/drawingml/2006/picture")).is_err());
}
#[test]fn inline_and_floating_are_separate(){
    assert!(parse_inline_picture(ANCHOR.as_bytes(),Limits::default()).is_err());
    let inline=ANCHOR.replace("wp:anchor","wp:inline");
    assert!(parse(&inline).is_err());
    assert!(parse_inline_picture(inline.as_bytes(),Limits::default()).is_ok());
}
#[test]fn respects_part_budget_and_single_wrapper_child(){
    let mut limits=Limits::default();limits.max_part_bytes=ANCHOR.len()-1;
    assert!(parse_floating_picture(ANCHOR.as_bytes(),limits).is_err());
    let w=format!("<w:drawing xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">{ANCHOR}{ANCHOR}</w:drawing>");
    assert!(parse(&w).is_err());
}
