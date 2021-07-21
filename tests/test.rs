use assert_cmd::Command;

const BIN_NAME: &'static str = "deduposaur";

#[test]
fn no_args() {
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .assert()
        .failure()
        .stderr(predicates::str::contains("USAGE"));
}

#[test]
fn empty_archive_arg() {
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg("--archive=")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "expected path, got empty string '--archive='",
        ));
}

#[test]
fn empty_process_arg() {
    let archive_dir = temp_dir::TempDir::new().unwrap();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!(
            "--archive={}",
            archive_dir.path().to_string_lossy()
        ))
        .arg("--process=")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "expected path, got empty string '--process='",
        ));
}
