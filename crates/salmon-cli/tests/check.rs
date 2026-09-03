use std::path::PathBuf;
use std::process::Command;

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
