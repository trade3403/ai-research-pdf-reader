//! Test-only ICC differential probe. No external engine linkage.
use rpcore::icc::Profile;
use std::io::Read;
fn main()->Result<(),Box<dyn std::error::Error>>{
    let path=std::env::args_os().nth(1).ok_or("usage: icc_probe PROFILE")?;
    let mut bytes=Vec::new();std::fs::File::open(path)?.take(16*1024*1024+1).read_to_end(&mut bytes)?;
    let p=Profile::parse(&bytes)?;let l=p.cmyk_a2b0()?;
    for c in [0.,0.25,1.]{for m in [0.,0.25,1.]{for y in [0.,0.25,1.]{for k in [0.,0.25,1.]{
        let x=[c,m,y,k];let v=l.evaluate_encoded(x)?;let lab=l.to_lab_d50(x)?;let xyz=l.to_xyz_d50(x)?;
        println!("{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12}",v[0],v[1],v[2],lab[0],lab[1],lab[2],xyz[0],xyz[1],xyz[2]);
    }}}}
    Ok(())
}
