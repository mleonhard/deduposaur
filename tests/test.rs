use assert_cmd::Command;
use assert_that::assert_that;
use filetime::FileTime;
use predicates::boolean::PredicateBooleanExt;
use temp_dir::TempDir;

pub const ARCHIVE_METADATA_JSON: &'static str = "deduposaur.archive_metadata.json";
const BIN_NAME: &'static str = "deduposaur";
/// 2011-11-11T19:11:11Z 2011-11-11T11:11:11-0700
const TIME1: i64 = 1321038671;
/// 2021-07-01T19:00:00Z 2020-07-01T12:00:00-0700
const TIME2: i64 = 1625166000;

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
        dir_with_corrupt_file.child(ARCHIVE_METADATA_JSON),
        "not-JSON",
    )
    .unwrap();
    let dir_with_non_file = TempDir::new().unwrap();
    std::fs::create_dir(dir_with_non_file.child(ARCHIVE_METADATA_JSON)).unwrap();
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
    std::fs::write(dir1.child(ARCHIVE_METADATA_JSON), "").unwrap();
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
        dir1.child(ARCHIVE_METADATA_JSON),
        "{\"expected\":[],\"deleted\":[]}",
    )
    .unwrap();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir1.path().to_string_lossy()))
        .assert()
        .success();
}

#[test]
fn test_verify() {
    let dir = TempDir::new().unwrap();
    let file1 = dir.child("file1");
    std::fs::write(&file1, "contents1").unwrap();
    filetime::set_file_mtime(&file1, FileTime::from_unix_time(TIME1, 0)).unwrap();
    std::fs::write(
        dir.child(ARCHIVE_METADATA_JSON),
        r#"{"expected":[
        {"path":"file1","mtime":1321038671,"digest":"809da78733fb34d7548ff1a8abe962ec865f8db07820e00f7a61ba79e2b6ff9f"}
        ],"deleted":[]}"#,
    )
    .unwrap();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::contains(format!(
            "Verified {}",
            dir.path().to_string_lossy()
        )));
}

#[test]
fn test_changed() {
    let dir = TempDir::new().unwrap();
    let file1 = dir.child("file1");
    std::fs::write(&file1, "contents2").unwrap();
    filetime::set_file_mtime(&file1, FileTime::from_unix_time(TIME1, 0)).unwrap();
    std::fs::write(
        dir.child(ARCHIVE_METADATA_JSON),
        r#"{"expected":[
        {"path":"file1","mtime":1321038671,"digest":"809da78733fb34d7548ff1a8abe962ec865f8db07820e00f7a61ba79e2b6ff9f"}
        ],"deleted":[]}"#,
    )
        .unwrap();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .write_stdin("n") // <------------
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "WARNING file1 is changed\nAccept change? (y/n) \n"
        )));
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .write_stdin("y") // <------------
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "WARNING file1 is changed\nAccept change? (y/n) \n"
        )));
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::contains(format!(
            "Verified {}",
            dir.path().to_string_lossy()
        )));
    assert_that!(
        &std::fs::read_to_string(ARCHIVE_METADATA_JSON).unwrap(),
        predicates::str::contains(
            "869ed4d9645d8f65f6650ff3e987e335183c02ebed99deccea2917c6fd7be006"
        )
    );
}

#[test]
fn test_renamed() {
    let dir = TempDir::new().unwrap();
    let file1 = dir.child("file1");
    let file2 = dir.child("file2");
    std::fs::write(&file2, "contents1").unwrap();
    filetime::set_file_mtime(&file2, FileTime::from_unix_time(TIME1, 0)).unwrap();
    std::fs::write(
        dir.child(ARCHIVE_METADATA_JSON),
        r#"{"expected":[
        {"path":"file1","mtime":1321038671,"digest":"809da78733fb34d7548ff1a8abe962ec865f8db07820e00f7a61ba79e2b6ff9f"}
        ],"deleted":[]}"#,
    )
        .unwrap();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .write_stdin("n") // <------------
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "WARNING file1 is renamed to file2\nAccept (y/n) or revert (r)? \n"
        )));
    assert!(!file1.exists());
    assert!(file2.exists());
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .write_stdin("y") // <------------
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "WARNING file1 is renamed to file2\nAccept (y/n) or revert (r)? \nVerified {}",
            dir.path().to_string_lossy()
        )));
    assert!(file1.exists());
    assert_eq!("contents1", &std::fs::read_to_string(&file1).unwrap());
    assert!(!file2.exists());
}
