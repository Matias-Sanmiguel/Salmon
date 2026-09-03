use std::env;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

mod input;

fn main() -> ExitCode {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    match args.as_slice() {
        [command, path] if command == "check" => check(path),
        [command, path, input] if command == "run" => run(path, input),
        _ => usage(),
    }
}

fn usage() -> ExitCode {
    eprintln!("usage:\n  salmon check <file.salmon>\n  salmon run <file.salmon> [input]");
    ExitCode::from(2)
}

fn check(path: &OsString) -> ExitCode {
    match load_automaton(path, 1) {
        Ok(_) => ExitCode::SUCCESS,
        Err(status) => status,
    }
}

fn run(path: &OsString, input: &OsString) -> ExitCode {
    let automaton = match load_automaton(path, 2) {
        Ok(automaton) => automaton,
        Err(status) => return status,
    };

    let input = match input.to_str() {
        Some(input) => input,
        None => {
            eprintln!("ERROR: input must be valid Unicode");
            return ExitCode::from(2);
        }
    };

    let input = match input::parse_input(input, &automaton.alphabet) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("ERROR: {error}");
            return ExitCode::from(2);
        }
    };

    if salmon_lang::accepts(&automaton, &input) {
        println!("OK");
        ExitCode::SUCCESS
    } else {
        println!("NOT");
        ExitCode::from(1)
    }
}

fn load_automaton(
    path: &OsString,
    invalid_automaton_exit: u8,
) -> Result<salmon_lang::Automaton, ExitCode> {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!(
                "error: could not read {}: {error}",
                Path::new(path).display()
            );
            return Err(ExitCode::from(2));
        }
    };

    let source_name = Path::new(path)
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy();

    match salmon_lang::parse(&source_name, &source) {
        Ok(automaton) => Ok(automaton),
        Err(diagnostics) => {
            for diagnostic in diagnostics {
                eprintln!(
                    "{}",
                    salmon_lang::render_diagnostic(&source_name, &source, &diagnostic)
                );
            }
            Err(ExitCode::from(invalid_automaton_exit))
        }
    }
}
