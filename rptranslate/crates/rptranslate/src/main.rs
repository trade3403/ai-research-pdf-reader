use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 1 || args[0] != "self-check" {
        eprintln!("Usage: rptranslate self-check (D1.0; translation not implemented)");
        return ExitCode::from(2);
    }
    match rptranslate::self_check() {
        Ok(bytes) => {
            println!("D1.0 PASS: synthetic corpus, scalar dot, model budget = {bytes} bytes");
            println!("Translation model: NOT READY; no network requests");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("D1.0 FAIL: {error}");
            ExitCode::FAILURE
        }
    }
}
