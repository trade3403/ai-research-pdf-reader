use rpinfer::toy_training::Trainer;

fn main() -> Result<(), String> {
    let mut trainer = Trainer::new(42, 0.2).map_err(|e| format!("{e:?}"))?;
    let mut restored = None;
    for step in 0..=100 {
        if step > 0 {
            trainer.step().map_err(|e| format!("{e:?}"))?;
            if let Some(other) = restored.as_mut() {
                Trainer::step(other).map_err(|e| format!("{e:?}"))?;
            }
        }
        if [0, 1, 37, 100].contains(&step) {
            print!(
                "STEP {step} {:.17}",
                trainer.loss().map_err(|e| format!("{e:?}"))?
            );
            for w in trainer.weights() {
                print!(" {w:.17}");
            }
            println!();
        }
        if step == 37 {
            let bytes = trainer.checkpoint().map_err(|e| format!("{e:?}"))?;
            print!("CHECKPOINT ");
            for b in &bytes {
                print!("{b:02x}");
            }
            println!();
            restored = Some(Trainer::restore(&bytes).map_err(|e| format!("{e:?}"))?);
        }
    }
    let other = restored.ok_or("no checkpoint")?;
    if other.checkpoint() != trainer.checkpoint() {
        return Err("resume mismatch".into());
    }
    println!("RESUME {} bitwise=true", trainer.steps());
    Ok(())
}
