use std::path::PathBuf;
use std::process::Command;

#[cfg(unix)]
#[test]
fn closed_stderr_preserves_error_exit_codes_without_panicking() {
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;
    use std::process::Stdio;

    let invalid = fixture("invalid.salmon");
    let valid = fixture("valid.salmon");
    let cases = [
        (vec![], 2),
        (vec!["check", "does-not-exist.salmon"], 2),
        (vec!["check", invalid.to_str().unwrap()], 1),
        (vec!["run", invalid.to_str().unwrap(), "a"], 2),
        (vec!["run", valid.to_str().unwrap(), "\"unclosed"], 2),
    ];
    for (args, expected) in cases {
        let (reader, writer) = UnixStream::pair().unwrap();
        drop(reader);
        let output = Command::new(env!("CARGO_BIN_EXE_salmon"))
            .args(&args)
            .stderr(Stdio::from(OwnedFd::from(writer)))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(expected), "args: {args:?}");
    }
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn valid_file_exits_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_salmon"))
        .args(["check", fixture("valid.salmon").to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8(output.stderr).unwrap().is_empty());
}

#[test]
fn invalid_definition_exits_one_with_a_positioned_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_salmon"))
        .args(["check", fixture("invalid.salmon").to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("invalid.salmon:4:9: error[SEM004]"));
}

#[cfg(unix)]
#[test]
fn positioned_error_preserves_a_non_utf8_unix_filename() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let directory = std::env::temp_dir().join(format!(
        "salmon-cli-non-utf8-filename-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(OsString::from_vec(b"invalid-\xff.salmon".to_vec()));
    std::fs::write(&path, include_bytes!("fixtures/invalid.salmon")).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_salmon"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap();

    std::fs::remove_dir_all(&directory).unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("invalid-\u{fffd}.salmon:4:9: error[SEM004]"));
}

#[test]
fn usage_and_io_failures_exit_two() {
    let no_args = Command::new(env!("CARGO_BIN_EXE_salmon")).output().unwrap();
    assert_eq!(no_args.status.code(), Some(2));
    let missing = Command::new(env!("CARGO_BIN_EXE_salmon"))
        .args(["check", "does-not-exist.salmon"])
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
}

fn check_with_contents(name: &str, contents: &[u8]) -> std::process::Output {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(&path, contents).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_salmon"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap();
    std::fs::remove_file(&path).unwrap();
    output
}

#[test]
fn utf8_file_with_byte_order_mark_is_valid() {
    let mut contents = b"\xef\xbb\xbf".to_vec();
    contents.extend_from_slice(include_bytes!("fixtures/valid.salmon"));

    let output = check_with_contents("utf8-bom.salmon", &contents);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
}

#[test]
fn utf8_byte_order_mark_does_not_shift_error_columns() {
    let mut contents = b"\xef\xbb\xbf".to_vec();
    contents.extend_from_slice(b"dfa x { @ }\n");

    let output = check_with_contents("utf8-bom-invalid.salmon", &contents);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("utf8-bom-invalid.salmon:1:9: error[SYN001]"),
        "{stderr}"
    );
}

#[test]
fn utf16_files_are_rejected_with_a_clear_message() {
    // PowerShell 5.1 writes UTF-16 LE with `>`; UTF-16 BE is less common but is detected too.
    let text = std::str::from_utf8(include_bytes!("fixtures/valid.salmon")).unwrap();
    let little_endian = [0xff, 0xfe]
        .into_iter()
        .chain(text.encode_utf16().flat_map(u16::to_le_bytes))
        .collect::<Vec<_>>();
    let big_endian = [0xfe, 0xff]
        .into_iter()
        .chain(text.encode_utf16().flat_map(u16::to_be_bytes))
        .collect::<Vec<_>>();

    for (name, contents) in [
        ("utf16-le.salmon", little_endian),
        ("utf16-be.salmon", big_endian),
    ] {
        let output = check_with_contents(name, &contents);
        assert_eq!(output.status.code(), Some(2), "{name}");
        assert!(output.stdout.is_empty(), "{name}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains(name)
                && stderr.contains("UTF-16")
                && stderr.contains("save it as UTF-8"),
            "{name}: {stderr}"
        );
    }
}
