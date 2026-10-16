use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    match dozr::run() {
        Ok(code) => code,
        Err(e) => {
            // `{:#}` prints the cause chain on one line, without the
            // backtrace that `Debug` would add when RUST_BACKTRACE is set.
            // Best effort: a closed stderr must not turn an error into a panic.
            let _ = writeln!(std::io::stderr(), "error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
