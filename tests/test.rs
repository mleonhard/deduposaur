use assert_cmd::Command;
use predicates::boolean::PredicateBooleanExt;
use temp_dir::TempDir;

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

#[test]
fn error_reading_archive_metadata_json() {
    let file1 = temp_file::empty();
    let empty_dir = TempDir::new().unwrap();
    let dir_with_corrupt_file = TempDir::new().unwrap();
    std::fs::write(
        dir_with_corrupt_file.child("deduposaur.archive_metadata.json"),
        "not-JSON",
    )
    .unwrap();
    let dir_with_non_file = TempDir::new().unwrap();
    std::fs::create_dir(dir_with_non_file.child("deduposaur.archive_metadata.json")).unwrap();
    for path in [
        file1.path(),
        empty_dir.path(),
        &empty_dir.child("nonexistent"),
        dir_with_corrupt_file.path(),
        dir_with_non_file.path(),
    ] {
        Command::cargo_bin(BIN_NAME)
            .unwrap()
            .arg(format!("--archive={}", path.to_string_lossy()))
            .assert()
            .failure()
            .stderr(
                predicates::str::contains("error reading")
                    .and(predicates::str::contains(path.to_string_lossy())),
            );
    }
}

#[test]
fn zero_length_archive_metadata_json() {
    let dir1 = TempDir::new().unwrap();
    std::fs::write(dir1.child("deduposaur.archive_metadata.json"), "").unwrap();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir1.path().to_string_lossy()))
        .assert()
        .success();
}

#[test]
fn empty_archive_metadata_json() {
    let dir1 = TempDir::new().unwrap();
    std::fs::write(
        dir1.child("deduposaur.archive_metadata.json"),
        "{\"expected\":[],\"deleted\":[]}",
    )
    .unwrap();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir1.path().to_string_lossy()))
        .assert()
        .success();
}
