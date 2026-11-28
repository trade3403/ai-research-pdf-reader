//! Development-only native JBIG2 image decode benchmark. No PDFium linkage.
use rpcore::filters::jbig2::page::{self,Limits};
use std::{io::Read,time::Instant,hint::black_box};
fn compose_benchmark()->Result<(),Box<dyn std::error::Error>>{
    use rpcore::filters::jbig2::bitmap::{Bitmap,ComposeBudget,ComposeOp};
    let (w,h)=(1024u32,512u32);
    let raw:Vec<u8>=(0..(w*h/8)).map(|i|i.wrapping_mul(73).wrapping_add(i/31)as u8).collect();
    let src=Bitmap::from_packed(w,h,&raw)?;
    for x in [0u32,3]{
        let mut fast=Vec::new();let mut slow=Vec::new();
        let n=u64::from(w-x)*u64::from(h);
        for i in 0..9{
            let mut a=Bitmap::new(w,h,true)?;
            let mut b=Bitmap::new(w,h,true)?;
            let mut budget=ComposeBudget::new(n)?;
            let mut packed=||->Result<u128,Box<dyn std::error::Error>>{
                let t=Instant::now();
                black_box(&mut a).compose(black_box(&src),i64::from(x),0,ComposeOp::Xor,&mut budget)?;
                Ok(t.elapsed().as_nanos())
            };
            let mut scalar=||->Result<u128,Box<dyn std::error::Error>>{
                let t=Instant::now();
                for y in 0..h{for dx in x..w{
                    let v=black_box(&b).get(dx,y).unwrap()^black_box(&src).get(dx-x,y).unwrap();
                    b.set(dx,y,v)?;
                }}
                Ok(t.elapsed().as_nanos())
            };
            let (f,s)=if i%2==0{let f=packed()?;let s=scalar()?;(f,s)}
                else{let s=scalar()?;let f=packed()?;(f,s)};
            if a!=b||budget.remaining()!=0{return Err("compose benchmark mismatch".into());}
            if i!=0{fast.push(f);slow.push(s);}
        }
        fast.sort_unstable();slow.sort_unstable();
        println!("compose_x{x}_packed_ns={}\ncompose_x{x}_scalar_ns={}",(fast[3]+fast[4])/2,(slow[3]+slow[4])/2);
    }
    Ok(())
}
fn run()->Result<(),Box<dyn std::error::Error>>{
    let a:Vec<String>=std::env::args().skip(1).collect();
    if a.len()==1&&a[0]=="--compose"{return compose_benchmark();}
    if a.len()!=2{return Err("usage: jbig2_perf_probe SEED OUTPUT or --compose".into());}
    let t=Instant::now();let mut bytes=Vec::new();
    std::fs::File::open(&a[0])?.take(4097).read_to_end(&mut bytes)?;
    if bytes.len()<2||bytes.len()>4096{return Err("invalid seed length".into());}
    let split=usize::from(u16::from_le_bytes([bytes[0],bytes[1]]));
    if split>=bytes.len()-2{return Err("invalid seed split".into());}
    let (g,p)=bytes[2..].split_at(split);
    let open_ns=t.elapsed().as_nanos();
    let strict=page::decode_pdf_page_reclaimed(g,p,Limits::default())?;
    let baseline=page::fuzz_pdf_image(g,p,Limits::default())?;
    if baseline.bitmap!=strict.bitmap{return Err("strict/compatible bitmap mismatch".into());}
    let mut times=Vec::with_capacity(20);
    for _ in 0..20{
        let t=Instant::now();
        let d=black_box(page::fuzz_pdf_image(black_box(g),black_box(p),Limits::default())?);
        times.push(t.elapsed().as_nanos());
        if d!=baseline{return Err("nonrepeatable decoded page".into());}
    }
    let s=baseline.stats;
    std::fs::write(&a[1],baseline.bitmap.packed())?;
    times.sort_unstable();
    println!("status=ok\nopen_ns={open_ns}\nelapsed_ns={}\np95_ns={}\nsamples_ns={}\nwidth={}\nheight={}\npacked_bytes={}\nwork_used={}\nregion_pixels={}\ndecisions={}\nhuffman_bits={}\ndictionary_work={}\ncomposed_pixels={}",
        (times[9]+times[10])/2,times[18],times.iter().map(ToString::to_string).collect::<Vec<_>>().join(","),
        baseline.bitmap.width(),baseline.bitmap.height(),s.retained_packed_bytes,s.work_used,s.region_pixels,s.decisions,s.huffman_bits,s.dictionaries.work_used,s.composed_pixels);
    Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("{e}");std::process::exit(1);}}
