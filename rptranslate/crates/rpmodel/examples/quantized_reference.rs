//! Independent byte-format probe; writer and reader do not share Python code.
use rpmodel::quantized::{write_container, Reader, TensorInput};
use std::{fs::OpenOptions, path::Path};

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [mode, path] = args.as_slice() else {
        return Err("usage: quantized_reference write|read FILE".into());
    };
    let path = Path::new(path);
    if mode == "write" {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        write_container(
            &mut file,
            &[
                TensorInput {
                    name: "a.weight",
                    shape: &[2, 3],
                    block_size: 4,
                    codes: &[127, -127, 1, 0, 2, -3],
                    scales: &[0.5, 0.25],
                },
                TensorInput {
                    name: "b.weight",
                    shape: &[1],
                    block_size: 1,
                    codes: &[0],
                    scales: &[1.],
                },
            ],
            1024,
        )
        .map_err(|e| format!("{e:?}"))?;
    } else if mode == "read" {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut reader = Reader::open(file, 1024).map_err(|e| format!("{e:?}"))?;
        let infos = reader.tensors().clone();
        for (name, info) in infos {
            for block in 0..info.blocks {
                let b = reader.read_block(&name, block, 4100).map_err(|e| format!("{e:?}"))?;
                println!("{name}|{block}|{:08x}|{:?}", b.scale.to_bits(), b.codes);
            }
        }
    } else {
        return Err("unknown mode".into());
    }
    Ok(())
}
