use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cam::cmd::dispatch(&args, std::path::Path::new(cam::ownership::RUNTIME_LOCK)) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("cam: diagnostic ownership refused: {error}");
            ExitCode::FAILURE
        }
    }
}
