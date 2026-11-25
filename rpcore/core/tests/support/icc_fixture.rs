// Original synthetic ICC profiles shared by vector and image tests.
fn test_icc(class:&[u8;4],device:&[u8;4],pcs:&[u8;4],tags:Vec<([u8;4],Vec<u8>)>)->Vec<u8>{
    let mut b=vec![0;132+12*tags.len()];b[8]=2;
    b[12..16].copy_from_slice(class);b[16..20].copy_from_slice(device);b[20..24].copy_from_slice(pcs);
    b[36..40].copy_from_slice(b"acsp");
    let put=|b:&mut[u8],p:usize,v:u32|b[p..p+4].copy_from_slice(&v.to_be_bytes());
    for (i,v) in [0.9642_f64,1.,0.8249].into_iter().enumerate(){put(&mut b,68+4*i,(v*65536.).round()as u32);}
    put(&mut b,128,tags.len()as u32);
    for (i,(name,data)) in tags.into_iter().enumerate(){
        let p=132+12*i;let off=b.len()as u32;
        b[p..p+4].copy_from_slice(&name);put(&mut b,p+4,off);put(&mut b,p+8,data.len()as u32);
        b.extend(data);while b.len()%4!=0{b.push(0);}
    }
    let len=b.len()as u32;put(&mut b,0,len);b
}
pub fn rgb_icc()->Vec<u8>{
    let mut tags=Vec::new();
    // Original analytic matrix: each column contributes 1/3 D50 white.
    for name in [*b"rXYZ",*b"gXYZ",*b"bXYZ"]{
        let mut v=b"XYZ \0\0\0\0".to_vec();
        for x in [0.9642_f64,1.,0.8249]{v.extend_from_slice(&((x/3.*65536.).round()as i32).to_be_bytes());}
        tags.push((name,v));
    }
    for name in [*b"rTRC",*b"gTRC",*b"bTRC"]{tags.push((name,b"curv\0\0\0\0\0\0\0\x01\x02\0".to_vec()));}
    test_icc(b"mntr",b"RGB ",b"XYZ ",tags)
}
pub fn cmyk_icc()->Vec<u8>{
    // Relative A2B1: neutral lightness controlled by K. Perceptual A2B0
    // deliberately constant black, so choosing the wrong intent fails.
    let lut=|relative:bool|{
        let mut t=vec![0;52];t[..4].copy_from_slice(b"mft2");t[8..11].copy_from_slice(&[4,3,2]);
        for i in [0,4,8]{t[12+4*i..16+4*i].copy_from_slice(&65536_u32.to_be_bytes());}
        t[48..52].copy_from_slice(&[0,2,0,2]);
        for _ in 0..4{t.extend_from_slice(&[0,0,255,255]);}
        for i in 0..16{
            let l=if relative&&i%2==0{65280_u16}else{0};
            for v in [l,32768,32768]{t.extend_from_slice(&v.to_be_bytes());}
        }
        for _ in 0..3{t.extend_from_slice(&[0,0,255,255]);}t
    };
    test_icc(b"prtr",b"CMYK",b"Lab ",vec![(*b"A2B0",lut(false)),(*b"A2B1",lut(true))])
}

pub fn gray_icc()->Vec<u8>{
    let mut wp=b"XYZ \0\0\0\0".to_vec();
    for x in [0.9642_f64,1.,0.8249]{wp.extend_from_slice(&((x*65536.).round()as i32).to_be_bytes());}
    test_icc(b"mntr",b"GRAY",b"XYZ ",vec![(*b"wtpt",wp),(*b"kTRC",b"curv\0\0\0\0\0\0\0\x01\x02\0".to_vec())])
}
