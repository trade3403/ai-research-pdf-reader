use rwcore::{resolve_formatting, Paragraph, Run, Error, Limits};
use rwcore::opc::{parse_styles, ParagraphAlignment, StyleCatalog};

fn catalog(body: &str) -> StyleCatalog {
    let xml = format!("<w:styles xmlns:w='http://schemas.openxmlformats.org/wordprocessingml/2006/main'>{body}</w:styles>");
    parse_styles(xml.as_bytes(), Limits::default()).unwrap()
}

#[test]
fn cascade_preserves_independent_fields_and_direct_overrides() {
    let c = catalog("<w:docDefaults><w:rPrDefault><w:rPr><w:sz w:val='20'/><w:rFonts w:ascii='Default'/></w:rPr></w:rPrDefault></w:docDefaults>
        <w:style w:type='paragraph' w:styleId='A' w:default='1'><w:pPr><w:spacing w:before='100'/></w:pPr><w:rPr><w:color w:val='112233'/><w:rFonts w:eastAsia='宋体'/></w:rPr></w:style>
        <w:style w:type='paragraph' w:styleId='B'><w:basedOn w:val='A'/><w:pPr><w:jc w:val='center'/></w:pPr><w:rPr><w:sz w:val='24'/></w:rPr></w:style>
        <w:style w:type='character' w:styleId='C'><w:rPr><w:sz w:val='28'/><w:i/></w:rPr></w:style>");
    let mut p = Paragraph::default();
    p.properties.style_id = Some("B".into());
    p.properties.after_twips = Some(200);
    let mut r = Run::default();
    r.properties.style_id = Some("C".into());
    r.properties.size_half_points = Some(32);
    let result = resolve_formatting(&c, &p, &r).unwrap();
    assert_eq!(result.paragraph.alignment, Some(ParagraphAlignment::Center));
    assert_eq!(result.paragraph.before_twips, Some(100));
    assert_eq!(result.paragraph.after_twips, Some(200));
    assert_eq!(result.run.size_half_points, Some(32));
    assert_eq!(result.run.color_rgb, Some([17,34,51]));
    assert_eq!(result.run.ascii_font.as_deref(), Some("Default"));
    assert_eq!(result.run.east_asia_font.as_deref(), Some("宋体"));
    assert_eq!(result.run.italic, Some(true));
    assert_eq!(r.properties.italic, None);
}

#[test]
fn style_toggles_and_direct_false() {
    let c = catalog("<w:style w:type='paragraph' w:styleId='A'><w:rPr><w:b/><w:i/></w:rPr></w:style>
        <w:style w:type='paragraph' w:styleId='B'><w:basedOn w:val='A'/><w:rPr><w:b/><w:i w:val='false'/></w:rPr></w:style>
        <w:style w:type='character' w:styleId='C'><w:rPr><w:b/></w:rPr></w:style>");
    let mut p = Paragraph::default(); p.properties.style_id = Some("B".into());
    let mut r = Run::default();
    let a = resolve_formatting(&c,&p,&r).unwrap();
    assert_eq!(a.run.bold, Some(false));
    assert_eq!(a.run.italic, Some(true));
    r.properties.style_id = Some("C".into());
    assert_eq!(resolve_formatting(&c,&p,&r).unwrap().run.bold, Some(true));
    r.properties.bold = Some(false);
    assert_eq!(resolve_formatting(&c,&p,&r).unwrap().run.bold, Some(false));
}

#[test]
fn default_style_and_explicit_style_selection() {
    let c = catalog("<w:style w:type='paragraph' w:styleId='Normal' w:default='1'><w:rPr><w:sz w:val='22'/></w:rPr></w:style>
        <w:style w:type='paragraph' w:styleId='Other'/>");
    let mut p = Paragraph::default(); let r = Run::default();
    assert_eq!(resolve_formatting(&c,&p,&r).unwrap().run.size_half_points, Some(22));
    p.properties.style_id = Some("Other".into());
    assert_eq!(resolve_formatting(&c,&p,&r).unwrap().run.size_half_points, None);
}

#[test]
fn mutually_exclusive_color_and_indent() {
    let c = catalog("<w:style w:type='paragraph' w:styleId='A' w:default='1'><w:pPr><w:ind w:hanging='120'/></w:pPr><w:rPr><w:color w:val='ABCDEF'/></w:rPr></w:style>");
    let mut p = Paragraph::default(); let mut r = Run::default();
    p.properties.first_line_twips = Some(0);
    r.properties.automatic_color = true;
    let a = resolve_formatting(&c,&p,&r).unwrap();
    assert_eq!(a.paragraph.first_line_twips, Some(0));
    assert_eq!(a.paragraph.hanging_twips, None);
    assert_eq!(a.run.color_rgb, None);
    assert!(a.run.automatic_color);
}

#[test]
fn invalid_styles_do_not_silently_fallback() {
    let c = catalog("<w:style w:type='paragraph' w:styleId='A'><w:basedOn w:val='A'/></w:style><w:style w:type='character' w:styleId='C'/>");
    let mut p = Paragraph::default(); let r = Run::default();
    for (id, err) in [("A", "cyclic style inheritance"), ("C", "wrong selected style kind"), ("missing", "missing selected style")] {
        p.properties.style_id = Some(id.into());
        assert_eq!(resolve_formatting(&c,&p,&r), Err(Error::Invalid(err)));
    }
}

#[test]
fn ambiguous_default_and_empty_catalog() {
    let c = catalog("<w:style w:type='paragraph' w:styleId='A' w:default='1'/><w:style w:type='paragraph' w:styleId='B' w:default='1'/>");
    assert_eq!(resolve_formatting(&c,&Paragraph::default(),&Run::default()), Err(Error::Invalid("multiple default styles of kind")));
    assert_eq!(resolve_formatting(&StyleCatalog::default(),&Paragraph::default(),&Run::default()).unwrap(), Default::default());
}


#[test]
fn document_view_preserves_empty_and_unsupported_blocks() {
    use rwcore::{Document, Block, FormattedBlock};
    let c = catalog("<w:style w:type='paragraph' w:styleId='Normal' w:default='1'><w:pPr><w:jc w:val='center'/></w:pPr></w:style>");
    let d = Document { styles: Some(c), blocks: vec![Block::Paragraph(Paragraph::default()),
        Block::Unsupported { name: "tbl".into() }], ..Default::default() };
    let mut blocks = d.formatted_blocks();
    let FormattedBlock::Paragraph(p) = blocks.next().unwrap().unwrap() else { panic!("paragraph expected") };
    assert_eq!(p.properties.alignment, Some(ParagraphAlignment::Center));
    assert_eq!(p.runs().count(), 0);
    assert!(matches!(blocks.next().unwrap().unwrap(), FormattedBlock::Unsupported { name: "tbl" }));
    assert!(blocks.next().is_none());
}

#[test]
fn document_view_keeps_run_order_and_isolates_errors() {
    use rwcore::{Document, Block, FormattedBlock};
    let c = catalog("<w:style w:type='paragraph' w:styleId='Normal' w:default='1'><w:rPr><w:b/></w:rPr></w:style>");
    let mut p = Paragraph::default();
    let first = Run { text: "科研".into(), ..Default::default() };
    let mut bad = Run { text: "bad".into(), ..Default::default() };
    bad.properties.style_id = Some("missing".into());
    let mut last = Run { text: "paper".into(), ..Default::default() };
    last.properties.bold = Some(false);
    p.runs = vec![first, bad, last];
    let d = Document { styles: Some(c), blocks: vec![Block::Paragraph(p)], ..Default::default() };
    let FormattedBlock::Paragraph(view) = d.formatted_blocks().next().unwrap().unwrap() else { panic!("paragraph expected") };
    let mut runs = view.runs();
    let first = runs.next().unwrap().unwrap();
    assert_eq!(first.source.text, "科研");
    assert_eq!(first.properties.bold, Some(true));
    assert_eq!(first.source.properties.bold, None);
    assert!(matches!(runs.next().unwrap(), Err(Error::Invalid("missing selected style"))));
    let last = runs.next().unwrap().unwrap();
    assert_eq!(last.source.text, "paper");
    assert_eq!(last.properties.bold, Some(false));
    assert!(runs.next().is_none());
}

#[test]
fn document_view_without_catalog_and_invalid_paragraph() {
    use rwcore::{Document, Block, FormattedBlock};
    let mut bad = Paragraph::default();
    bad.properties.style_id = Some("absent".into());
    let mut good = Paragraph::default();
    good.properties.keep_next = Some(true);
    good.runs.push(Run { text: "ok".into(), ..Default::default() });
    let d = Document { blocks: vec![Block::Paragraph(bad), Block::Paragraph(good)], ..Default::default() };
    let mut blocks = d.formatted_blocks();
    assert!(matches!(blocks.next().unwrap(), Err(Error::Invalid("missing selected style"))));
    let FormattedBlock::Paragraph(view) = blocks.next().unwrap().unwrap() else { panic!("paragraph expected") };
    assert_eq!(view.properties.keep_next, Some(true));
    assert_eq!(view.runs().next().unwrap().unwrap().source.text, "ok");
}
