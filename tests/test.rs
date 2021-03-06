use assert_cmd::Command;
use assert_that::assert_that;
use filetime::FileTime;
use predicates::boolean::PredicateBooleanExt;
use std::convert::TryInto;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use temp_dir::TempDir;

const ARCHIVE_METADATA_JSON: &str = "deduposaur.archive_metadata.json";
const PROCESS_METADATA_JSON: &str = "deduposaur.process_metadata.json";
const BIN_NAME: &str = "deduposaur";
/// 2011-11-11T19:11:11Z 2011-11-11T11:11:11-08:00
#[allow(clippy::unreadable_literal)]
const TIME1: i64 = 1321038671;
/// 2021-07-01T19:00:00Z 2021-07-01T12:00:00-07:00
#[allow(clippy::unreadable_literal)]
const TIME2: i64 = 1625166000;

fn get_mtime(p: impl AsRef<Path>) -> i64 {
    std::fs::metadata(p.as_ref())
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .try_into()
        .unwrap()
}

fn write_file(path: PathBuf, contents: impl AsRef<[u8]>, mtime: i64) -> PathBuf {
    std::fs::write(&path, contents.as_ref()).unwrap();
    filetime::set_file_mtime(&path, FileTime::from_unix_time(mtime, 0)).unwrap();
    path
}

fn print_file(path: &Path) {
    println!(
        "{}: {}",
        path.to_string_lossy(),
        std::fs::read_to_string(&path)
            .or_else(|e| Ok::<String, ()>(format!("error: {}", e)))
            .unwrap()
    );
}

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
fn test_new_file() {
    let dir = TempDir::new().unwrap();
    write_file(dir.child("file1"), "contents1", TIME1);
    write_file(dir.child("file2"), "contents2", TIME2);
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
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            dir.path().to_string_lossy()
        )));
    assert_that!(
        &std::fs::read_to_string(dir.child(ARCHIVE_METADATA_JSON)).unwrap(),
        predicates::str::diff(
            r#"{"expected":[{"path":"file1","mtime":1321038671,"digest":"809da78733fb34d7548ff1a8abe962ec865f8db07820e00f7a61ba79e2b6ff9f"},{"path":"file2","mtime":1625166000,"digest":"869ed4d9645d8f65f6650ff3e987e335183c02ebed99deccea2917c6fd7be006"}],"deleted":[]}"#
        )
    );
}

#[test]
fn test_contents_changed() {
    let dir = TempDir::new().unwrap();
    write_file(dir.child("file1"), "contents2", TIME2);
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
        .stdout(predicates::str::diff(
            "WARNING file1 is changed\nAccept change? (y/n) \n",
        ));
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .write_stdin("y") // <------------
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "WARNING file1 is changed\nAccept change? (y/n) \nVerified {}\n",
            dir.path().to_string_lossy()
        )));
    assert_that!(
        &std::fs::read_to_string(dir.child(ARCHIVE_METADATA_JSON)).unwrap(),
        predicates::str::contains(
            "869ed4d9645d8f65f6650ff3e987e335183c02ebed99deccea2917c6fd7be006"
        )
    );
    assert_that!(
        &std::fs::read_to_string(dir.child(ARCHIVE_METADATA_JSON)).unwrap(),
        predicates::str::contains("1625166000")
    );
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            dir.path().to_string_lossy()
        )));
}

#[test]
fn test_accept_mtime_change() {
    let dir = TempDir::new().unwrap();
    let file1 = write_file(dir.child("file1"), "contents1", TIME2);
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
        .stdout(predicates::str::diff(
            "WARNING file1 mtime changed 2011-11-11T11:11:11-08:00 -> 2021-07-01T12:00:00-07:00\nAccept (y/n) or revert (r)? \n"
        ));
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .write_stdin("y") // <------------
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "WARNING file1 mtime changed 2011-11-11T11:11:11-08:00 -> 2021-07-01T12:00:00-07:00\nAccept (y/n) or revert (r)? \nVerified {}\n",
            dir.path().to_string_lossy()
        )));
    assert_eq!(get_mtime(&file1), TIME2);
    assert_that!(
        &std::fs::read_to_string(dir.child(ARCHIVE_METADATA_JSON)).unwrap(),
        predicates::str::contains(TIME2.to_string())
    );
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            dir.path().to_string_lossy()
        )));
}

#[test]
fn test_revert_mtime_change() {
    let dir = TempDir::new().unwrap();
    let file1 = write_file(dir.child("file1"), "contents1", TIME2);
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
        .stdout(predicates::str::diff(
            "WARNING file1 mtime changed 2011-11-11T11:11:11-08:00 -> 2021-07-01T12:00:00-07:00\nAccept (y/n) or revert (r)? \n"
        ));
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .write_stdin("r") // <------------
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "WARNING file1 mtime changed 2011-11-11T11:11:11-08:00 -> 2021-07-01T12:00:00-07:00\nAccept (y/n) or revert (r)? \nVerified {}\n",
            dir.path().to_string_lossy()
        )));
    assert_eq!(get_mtime(&file1), TIME1);
    assert_that!(
        &std::fs::read_to_string(dir.child(ARCHIVE_METADATA_JSON)).unwrap(),
        predicates::str::contains(TIME1.to_string())
    );
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            dir.path().to_string_lossy()
        )));
}

#[test]
fn test_renamed() {
    let dir = TempDir::new().unwrap();
    write_file(dir.child("file2"), "contents1", TIME1);
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
        .stdout(predicates::str::diff(
            "WARNING file1 is renamed to file2\nAccept change? (y/n) \n",
        ));
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .write_stdin("y") // <------------
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "WARNING file1 is renamed to file2\nAccept change? (y/n) \nVerified {}\n",
            dir.path().to_string_lossy()
        )));
    assert_that!(
        &std::fs::read_to_string(dir.child(ARCHIVE_METADATA_JSON)).unwrap(),
        predicates::str::contains("file2")
    );
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            dir.path().to_string_lossy()
        )));
}

#[test]
fn test_deleted() {
    let dir = TempDir::new().unwrap();
    write_file(dir.child("file1"), "contents1", TIME1);
    std::fs::write(
        dir.child(ARCHIVE_METADATA_JSON),
        r#"{"expected":[
        {"path":"file1","mtime":1321038671,"digest":"809da78733fb34d7548ff1a8abe962ec865f8db07820e00f7a61ba79e2b6ff9f"},
        {"path":"file2","mtime":1321038671,"digest":"809da78733fb34d7548ff1a8abe962ec865f8db07820e00f7a61ba79e2b6ff9f"}
        ],"deleted":[]}"#,
    )
        .unwrap();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .write_stdin("n") // <------------
        .assert()
        .success()
        .stdout(predicates::str::diff(
            "WARNING file2 is deleted\nAccept change? (y/n) \n",
        ));
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .write_stdin("y") // <------------
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "WARNING file2 is deleted\nAccept change? (y/n) \nVerified {}\n",
            dir.path().to_string_lossy()
        )));
    assert_that!(
        &std::fs::read_to_string(dir.child(ARCHIVE_METADATA_JSON)).unwrap(),
        predicates::str::diff(
            r#"{"expected":[{"path":"file1","mtime":1321038671,"digest":"809da78733fb34d7548ff1a8abe962ec865f8db07820e00f7a61ba79e2b6ff9f"}],"deleted":[{"path":"file2","mtime":1321038671,"digest":"809da78733fb34d7548ff1a8abe962ec865f8db07820e00f7a61ba79e2b6ff9f"}]}"#
        )
    );
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            dir.path().to_string_lossy()
        )));
}

fn list_metadata_backups(dir: impl AsRef<Path>) -> Vec<String> {
    dir.as_ref()
        .read_dir()
        .map_err(|e| {
            format!(
                "error reading dir {}: {}",
                dir.as_ref().to_string_lossy(),
                e
            )
        })
        .unwrap()
        .map(|result| {
            result
                .map_err(|e| format!("error reading dir entry: {}", e))
                .unwrap()
        })
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|filename| filename.starts_with(ARCHIVE_METADATA_JSON))
        .filter(|filename| filename != ARCHIVE_METADATA_JSON)
        .collect()
}

#[test]
fn test_metadata_json_file_backups() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.child(ARCHIVE_METADATA_JSON), b"").unwrap();
    assert!(list_metadata_backups(dir.path()).is_empty());
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", dir.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            dir.path().to_string_lossy()
        )));
    assert_that!(
        &std::fs::read_to_string(dir.child(ARCHIVE_METADATA_JSON)).unwrap(),
        predicates::str::diff(r#"{"expected":[],"deleted":[]}"#)
    );
    let metadata_backups = list_metadata_backups(dir.path());
    assert_eq!(1, metadata_backups.len());
    let first_backup = metadata_backups.first().unwrap();
    assert_that!(
        &std::fs::read_to_string(dir.child(first_backup)).unwrap(),
        predicates::str::diff("")
    );

    write_file(dir.child("file1"), "contents1", TIME1);
    for _ in [0, 1] {
        std::thread::sleep(Duration::from_secs(1));
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
            &std::fs::read_to_string(dir.child(ARCHIVE_METADATA_JSON)).unwrap(),
            predicates::str::diff(
                r#"{"expected":[{"path":"file1","mtime":1321038671,"digest":"809da78733fb34d7548ff1a8abe962ec865f8db07820e00f7a61ba79e2b6ff9f"}],"deleted":[]}"#
            )
        );
        let metadata_backups = list_metadata_backups(dir.path());
        println!("metadata_backups {:?}", metadata_backups);
        assert_eq!(2, metadata_backups.len());
        let second_backup = metadata_backups
            .iter()
            .find(|filename| filename != &first_backup)
            .unwrap();
        assert_that!(
            &std::fs::read_to_string(dir.child(first_backup)).unwrap(),
            predicates::str::diff("")
        );
        assert_that!(
            &std::fs::read_to_string(dir.child(second_backup)).unwrap(),
            predicates::str::diff(r#"{"expected":[],"deleted":[]}"#)
        );
    }
}

#[test]
fn test_renames_dupe() {
    let archive = TempDir::new().unwrap();
    let archive_sub1 = archive.path().join("sub1");
    std::fs::create_dir(&archive_sub1).unwrap();
    let archive_sub1_file1 = write_file(archive_sub1.join("file1"), "contents1", TIME1);
    std::fs::write(archive.child(ARCHIVE_METADATA_JSON), "").unwrap();
    let process = TempDir::new().unwrap();
    let process_sub2 = process.path().join("sub2");
    std::fs::create_dir(&process_sub2).unwrap();
    let process_sub2_file1 = write_file(process_sub2.join("file1"), "contents1", TIME1);
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\nRenamed sub2/DUPE.file1 - {}/sub1/file1\n",
            archive.path().to_string_lossy(),
            archive.path().to_string_lossy(),
        )));
    let check = || {
        assert!(archive_sub1_file1.exists());
        assert!(!process_sub2_file1.exists());
        assert!(process_sub2.join("DUPE.file1").exists());
        assert!(!process.child(PROCESS_METADATA_JSON).exists());
    };
    check();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            archive.path().to_string_lossy()
        )));
    check();
}

#[test]
fn test_renames_deleted() {
    let archive = TempDir::new().unwrap();
    let archive_file1 = write_file(archive.child("file1"), "contents1", TIME1);
    std::fs::write(archive.child(ARCHIVE_METADATA_JSON), "").unwrap();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            archive.path().to_string_lossy()
        )));
    std::fs::remove_file(&archive_file1).unwrap();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .write_stdin("y") // <------------
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "WARNING file1 is deleted\nAccept change? (y/n) \nVerified {}\n",
            archive.path().to_string_lossy()
        )));

    let process = TempDir::new().unwrap();
    let process_file2 = write_file(process.child("file2"), "contents1", TIME1);
    let process_sub2 = process.path().join("sub2");
    std::fs::create_dir(&process_sub2).unwrap();
    let process_sub2_file3 = write_file(process_sub2.join("file3"), "contents1", TIME1);
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\nRenamed DELETED.file2\nRenamed sub2/DELETED.file3\n",
            archive.path().to_string_lossy(),
        )));
    let check = || {
        assert!(!process_file2.exists());
        assert!(process.child("DELETED.file2").exists());
        assert!(!process_sub2_file3.exists());
        assert!(process_sub2.join("DELETED.file3").exists());
        assert!(!process.child(PROCESS_METADATA_JSON).exists());
    };
    check();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            archive.path().to_string_lossy()
        )));
    check();
}

#[test]
fn test_renames_changed() {
    let archive = TempDir::new().unwrap();
    std::fs::write(archive.child(ARCHIVE_METADATA_JSON), "").unwrap();
    let archive_sub1 = archive.path().join("sub1");
    std::fs::create_dir(&archive_sub1).unwrap();
    write_file(archive_sub1.join("file1"), "contents1", TIME1);
    let process = TempDir::new().unwrap();
    let process_sub1 = process.path().join("sub1");
    std::fs::create_dir(&process_sub1).unwrap();
    let process_sub1_file1 = write_file(process_sub1.join("file1"), "contents2", TIME2);
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\nRenamed sub1/CHANGED.file1\n",
            archive.path().to_string_lossy(),
        )));
    let check = || {
        assert!(!process_sub1_file1.exists());
        assert!(process_sub1.join("CHANGED.file1").exists());
        assert!(!process.child(PROCESS_METADATA_JSON).exists());
    };
    check();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            archive.path().to_string_lossy()
        )));
    check();
}

#[test]
fn test_renames_metadata_changed() {
    let archive = TempDir::new().unwrap();
    std::fs::write(archive.child(ARCHIVE_METADATA_JSON), "").unwrap();
    let archive_sub1 = archive.path().join("sub1");
    std::fs::create_dir(&archive_sub1).unwrap();
    write_file(archive_sub1.join("file1"), "contents1", TIME1);
    let process = TempDir::new().unwrap();
    let process_sub1 = process.path().join("sub1");
    std::fs::create_dir(&process_sub1).unwrap();
    let process_sub1_file1 = write_file(process_sub1.join("file1"), "contents1", TIME2);
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\nRenamed sub1/METADATA.file1\n",
            archive.path().to_string_lossy(),
        )));
    let check = || {
        assert!(!process_sub1_file1.exists());
        assert!(process_sub1.join("METADATA.file1").exists());
        assert!(!process.child(PROCESS_METADATA_JSON).exists());
    };
    check();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            archive.path().to_string_lossy()
        )));
    check();
}

#[test]
fn test_remembers_process_deleted() {
    let archive = TempDir::new().unwrap();
    std::fs::write(archive.child(ARCHIVE_METADATA_JSON), "").unwrap();
    let process = TempDir::new().unwrap();
    let process_sub1 = process.path().join("sub1");
    std::fs::create_dir(&process_sub1).unwrap();
    let process_sub1_file1 = write_file(process_sub1.join("file1"), "contents1", TIME1);
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            archive.path().to_string_lossy()
        )));

    std::fs::remove_file(&process_sub1_file1).unwrap();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\nsub1/file1 was deleted\n",
            archive.path().to_string_lossy()
        )));
    let process_metadata_json = process.child(PROCESS_METADATA_JSON);
    print_file(&process_metadata_json);
    assert!(!process_metadata_json.exists());

    let process2 = TempDir::new().unwrap();
    let process2_file2 = write_file(process2.child("file2"), "contents1", TIME2);
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process2.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\nRenamed DELETED.file2\n",
            archive.path().to_string_lossy(),
        )));
    let process_deleted_file2 = process2.child("DELETED.file2");
    assert!(!process2_file2.exists());
    assert!(process_deleted_file2.exists());
    assert!(!process_metadata_json.exists());

    std::fs::remove_file(&process_deleted_file2).unwrap();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process2.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            archive.path().to_string_lossy()
        )));
    assert!(!process_metadata_json.exists());
}

#[test]
fn test_user_renames_process_file() {
    let archive = TempDir::new().unwrap();
    std::fs::write(archive.child(ARCHIVE_METADATA_JSON), "").unwrap();
    let process = TempDir::new().unwrap();
    let file1 = write_file(process.child("file1"), "contents1", TIME1);
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            archive.path().to_string_lossy()
        )));

    let file1_renamed = process.child("file1_renamed");
    std::fs::rename(&file1, &file1_renamed).unwrap();
    // Program should not treat file as deleted.
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            archive.path().to_string_lossy()
        )));
}

#[test]
fn test_user_moves_file_to_archive() {
    let archive = TempDir::new().unwrap();
    std::fs::write(archive.child(ARCHIVE_METADATA_JSON), "").unwrap();
    let process = TempDir::new().unwrap();
    let process_file1 = write_file(process.child("file1"), "contents1", TIME1);
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            archive.path().to_string_lossy()
        )));

    let archive_file1 = archive.child("file1");
    std::fs::rename(&process_file1, &archive_file1).unwrap();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\n",
            archive.path().to_string_lossy()
        )));
    let process_metadata_json = process.child(PROCESS_METADATA_JSON);
    print_file(&process_metadata_json);
    assert!(!process_metadata_json.exists());

    let process_file2 = write_file(process.child("file2"), "contents1", TIME1);
    // Program should treat file2 as a duplicate, not deleted.
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(format!("--archive={}", archive.path().to_string_lossy()))
        .arg(format!("--process={}", process.path().to_string_lossy()))
        .assert()
        .success()
        .stdout(predicates::str::diff(format!(
            "Verified {}\nRenamed DUPE.file2 - {}/file1\n",
            archive.path().to_string_lossy(),
            archive.path().to_string_lossy(),
        )));
    assert!(!process_file2.exists());
    assert!(process.child("DUPE.file2").exists());
    print_file(&process_metadata_json);
    assert!(!process_metadata_json.exists());
}
