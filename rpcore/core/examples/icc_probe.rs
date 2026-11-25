//! Test-only ICC differential probe. No external engine linkage.
use rpcore::icc::Profile;
use std::io::Read;
fn main()->Result<(),Box<dyn std::error::Error>>{
    let path=std::env::args_os().nth(1).ok_or("usage: icc_probe PROFILE")?;
    let mut bytes=Vec::new();std::fs::File::open(path)?.take(16*1024*1024+1).read_to_end(&mut bytes)?;
    if std::env::args_os().nth(2).is_some_and(|s|s=="perf-mab"){
        use std::{hint::black_box,time::Instant};
        const N:usize=100_000;
        let start=Instant::now();
        let p=Profile::parse(&bytes)?;
        let l=p.cmyk_mab(b"A2B1")?;
        let parse_us=start.elapsed().as_micros();
        let mut state=3403u32;
        let mut samples=Vec::with_capacity(N);
        for _ in 0..N{
            samples.push(std::array::from_fn(|_|{
                state=state.wrapping_mul(1664525).wrapping_add(1013904223);
                f64::from(state>>8)/16777215.
            }));
        }
        // Warm up once, then three measured passes over identical input.
        // Hash the output to prevent dead-code elimination and check repeatability.
        let evaluate=||->std::result::Result<u64,rpcore::Error>{
            let mut hash=0xcbf29ce484222325u64;
            for x in &samples{
                for v in black_box(l.to_xyz_d50(black_box(*x))?){
                    if !v.is_finite(){return Err(rpcore::Error::Invalid("ICC benchmark nonfinite XYZ"));}
                    hash=(hash^v.to_bits()).wrapping_mul(0x100000001b3);
                }
            }
            Ok(black_box(hash))
        };
        let hash=evaluate()?;
        let mut times=[0u128;3];
        for t in &mut times{
            let start=Instant::now();let next=evaluate()?;*t=start.elapsed().as_nanos();
            if next!=hash{return Err("ICC benchmark nondeterministic output".into());}
        }
        times.sort_unstable();
        println!("samples={N}\nparse_us={parse_us}\nmin_ns={}\nmedian_ns={}\nmax_ns={}\nchecksum={hash:016x}",times[0],times[1],times[2]);
        return Ok(());
    }
    let p=Profile::parse(&bytes)?;
    if std::env::args_os().nth(2).is_some_and(|s|s=="rgb"){
        let m=p.rgb_matrix()?;
        for x in [[0.,0.,0.],[1.,1.,1.],[1.,0.,0.],[0.,1.,0.],[0.,0.,1.],[0.02,0.5,0.8],[0.5,0.5,0.5],[0.8,0.3,0.1]]{
            let v=m.to_xyz_d50(x)?;println!("{:.12},{:.12},{:.12}",v[0],v[1],v[2]);
        }
        return Ok(());
    }
    if std::env::args_os().nth(2).is_some_and(|s|s=="mab"){
        let l=p.cmyk_mab(b"A2B0")?;
        for c in [0.,0.25,1.]{for m in [0.,0.25,1.]{for y in [0.,0.25,1.]{for k in [0.,0.25,1.]{
            let x=[c,m,y,k];let v=l.evaluate_encoded(x)?;let lab=l.to_lab_d50(x)?;let xyz=l.to_xyz_d50(x)?;
            println!("{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12}",v[0],v[1],v[2],lab[0],lab[1],lab[2],xyz[0],xyz[1],xyz[2]);
        }}}}
        return Ok(());
    }
    let l=p.cmyk_a2b0()?;
    for c in [0.,0.25,1.]{for m in [0.,0.25,1.]{for y in [0.,0.25,1.]{for k in [0.,0.25,1.]{
        let x=[c,m,y,k];let v=l.evaluate_encoded(x)?;let lab=l.to_lab_d50(x)?;let xyz=l.to_xyz_d50(x)?;
        println!("{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12}",v[0],v[1],v[2],lab[0],lab[1],lab[2],xyz[0],xyz[1],xyz[2]);
    }}}}
    Ok(())
}
