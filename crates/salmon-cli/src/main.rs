use std::env;
use std::ffi::OsString;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process::ExitCode;

mod input;

// Reporting an error must not panic if the diagnostic stream is also closed.
macro_rules! report_error {
    ($($argument:tt)*) => {
        { let _ = writeln!(io::stderr().lock(), $($argument)*); }
    };
}

fn main() -> ExitCode {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    match args.as_slice() {
        [command, path] if command == "check" => check(path),
        [command, path] if command == "run" => run_interactive_command(path),
        [command, path, input] if command == "run" => run(path, input),
        _ => usage(),
    }
}

fn usage() -> ExitCode {
    report_error!("usage:\n  salmon check <file.salmon>\n  salmon run <file.salmon> [input]");
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
            report_error!("ERROR: input must be valid Unicode");
            return ExitCode::from(2);
        }
    };

    let input = match input::parse_input(input, &automaton.alphabet) {
        Ok(input) => input,
        Err(error) => {
            report_error!("ERROR: {error}");
            return ExitCode::from(2);
        }
    };

    let accepted = salmon_lang::accepts(&automaton, &input);
    match write_one_shot_result(accepted, io::stdout().lock()) {
        Ok(status) => status,
        Err(error) => {
            let detail = match error.kind() {
                io::ErrorKind::BrokenPipe => "broken pipe".to_owned(),
                _ => error.to_string(),
            };
            let _ = writeln!(io::stderr().lock(), "error: one-shot I/O: {detail}");
            ExitCode::from(2)
        }
    }
}

fn write_one_shot_result<W: Write>(accepted: bool, mut writer: W) -> io::Result<ExitCode> {
    if accepted {
        writeln!(writer, "OK")?;
        Ok(ExitCode::SUCCESS)
    } else {
        writeln!(writer, "NOT")?;
        Ok(ExitCode::from(1))
    }
}

fn run_interactive_command(path: &OsString) -> ExitCode {
    let automaton = match load_automaton(path, 2) {
        Ok(automaton) => automaton,
        Err(status) => return status,
    };

    match run_interactive(&automaton, io::stdin().lock(), io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            report_error!("error: interactive I/O: {error}");
            ExitCode::from(2)
        }
    }
}

fn run_interactive<R: BufRead, W: Write>(
    automaton: &salmon_lang::Automaton,
    mut reader: R,
    mut writer: W,
) -> io::Result<()> {
    writeln!(writer, "Salmon — {}", automaton.name)?;

    let mut bytes = Vec::new();
    let mut first_line = true;
    loop {
        write!(writer, "> ")?;
        writer.flush()?;

        // Read raw bytes so a line that is not UTF-8 (for example, from cmd with code page 850)
        // prints `ERROR` and the session continues instead of ending.
        bytes.clear();
        if reader.read_until(b'\n', &mut bytes)? == 0 {
            return Ok(());
        }

        let is_first_line = std::mem::replace(&mut first_line, false);
        let Ok(line) = std::str::from_utf8(&bytes) else {
            writeln!(writer, "ERROR: input must be valid UTF-8")?;
            continue;
        };
        let line = line.strip_suffix('\n').unwrap_or(line);
        let line = line.strip_suffix('\r').unwrap_or(line);
        // PowerShell 5.1 with `$OutputEncoding` set to UTF-8 sends two BOMs at the start of a pipe.
        let line = if is_first_line {
            line.trim_start_matches('\u{feff}')
        } else {
            line
        };
        if line == ":quit" {
            return Ok(());
        }

        match input::parse_input(line, &automaton.alphabet) {
            Ok(input) if salmon_lang::accepts(automaton, &input) => writeln!(writer, "OK")?,
            Ok(_) => writeln!(writer, "NOT")?,
            Err(error) => writeln!(writer, "ERROR: {error}")?,
        }
    }
}

// PowerShell 5.1 writes UTF-16 LE with a BOM when redirecting with `>`. Only the BOM is checked:
// without it UTF-16 cannot be told apart reliably, so that case keeps the generic UTF-8 error.
fn is_utf16(path: &OsString) -> bool {
    let mut start = [0; 2];
    std::fs::File::open(path)
        .and_then(|mut file| io::Read::read_exact(&mut file, &mut start))
        .is_ok_and(|()| matches!(start, [0xff, 0xfe] | [0xfe, 0xff]))
}

fn load_automaton(
    path: &OsString,
    invalid_automaton_exit: u8,
) -> Result<salmon_lang::Automaton, ExitCode> {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == io::ErrorKind::InvalidData && is_utf16(path) => {
            report_error!(
                "error: could not read {}: the file is saved as UTF-16; save it as UTF-8 and try again",
                Path::new(path).display()
            );
            return Err(ExitCode::from(2));
        }
        Err(error) => {
            report_error!(
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
                report_error!(
                    "{}",
                    salmon_lang::render_diagnostic(&source_name, &source, &diagnostic)
                );
            }
            Err(ExitCode::from(invalid_automaton_exit))
        }
    }
}
