#![no_main]
use libfuzzer_sys::fuzz_target;
use rpcore::filters::jbig2::{dictionary,page::{self,Limits}};
fn run(data:&[u8]){
    if data.len()<2||data.len()>4096{return;}
    let body=&data[2..];
    let split=usize::from(u16::from_le_bytes([data[0],data[1]])).min(body.len());
    let (globals,stream)=body.split_at(split);
    let mut l=Limits{page_pixels:8192,region_pixels:16384,regions:64,
        packed_bytes:65536,work:400000,decisions:65536,
        text_slots:256,text_instances:512,huffman_bits:65536,
        patterns:128,pattern_pixels:16384,grid_cells:512,..Limits::default()};
    l.dictionaries=dictionary::ExecutionLimits{dictionaries:32,new_symbols:256,
        symbol_slots:1024,pixels:16384,packed_bytes:32768,work:200000,
        export_runs_per_dictionary:512,decisions_per_dictionary:65536,
        ..l.dictionaries};
    let a=page::fuzz_pdf_image(globals,stream,l);
    let b=page::fuzz_pdf_image(globals,stream,l);
    assert_eq!(a,b,"non-deterministic page decode");
    if let Ok(d)=a{
        let w=d.bitmap.width();let h=d.bitmap.height();
        assert!(w>0&&h>0);
        assert_eq!(d.info.width,w);
        assert_eq!(d.info.height,Some(h));
        assert!(u64::from(w)*u64::from(h)<=l.page_pixels);
        assert_eq!(d.bitmap.packed().len(),(w as usize).div_ceil(8)*h as usize);
        assert_eq!(d.stats.retained_packed_bytes,d.bitmap.packed().len());
        assert!(d.stats.retained_packed_bytes<=l.packed_bytes);
        assert!(d.stats.work_used<=l.work&&d.stats.region_pixels<=l.region_pixels);
        assert!(d.stats.decisions<=l.decisions&&d.stats.huffman_bits<=l.huffman_bits);
        assert!(d.stats.dictionaries.work_used<=l.dictionaries.work);
        assert!(d.stats.dictionaries.pixels<=l.dictionaries.pixels);
    }
}
fuzz_target!(|data: &[u8]| { run(data); });
