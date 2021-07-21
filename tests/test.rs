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

// #[test]
// fn missing_archive_metadata_json() {
//     let dir = temp_dir::TempDir::new().unwrap();
//
// }
