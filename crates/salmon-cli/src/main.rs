use std::env;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

#[allow(dead_code)] // The run command consumes this private module in the next implementation task.
mod input;

fn main() -> ExitCode {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    let [command, path] = args.as_slice() else {
        eprintln!("usage: salmon check <file.salmon>");
        return ExitCode::from(2);
    };

    if command != "check" {
        eprintln!("usage: salmon check <file.salmon>");
        return ExitCode::from(2);
    }

    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!(
                "error: could not read {}: {error}",
                Path::new(path).display()
            );
            return ExitCode::from(2);
        }
    };

    let source_name = Path::new(path)
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy();

    match salmon_lang::parse(&source_name, &source) {
        Ok(_) => ExitCode::SUCCESS,
        Err(diagnostics) => {
            for diagnostic in diagnostics {
                eprintln!(
                    "{}",
                    salmon_lang::render_diagnostic(&source_name, &source, &diagnostic)
                );
            }
            ExitCode::from(1)
        }
    }
}
