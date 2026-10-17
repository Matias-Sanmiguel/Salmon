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
