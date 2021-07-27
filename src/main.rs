//! [![crates.io version](https://img.shields.io/crates/v/deduposaur.svg)](https://crates.io/crates/deduposaur)
//! [![license: Apache 2.0](https://raw.githubusercontent.com/mleonhard/deduposaur/main/license-apache-2.0.svg)](https://github.com/mleonhard/deduposaur/blob/main/LICENSE)
//! [![unsafe forbidden](https://gitlab.com/leonhard-llc/ops/-/raw/main/unsafe-forbidden.svg)](https://github.com/rust-secure-code/safety-dance/)
//!
//! Deduposaur is a command-line program to help you add files to your personal archive.
//!
//! Functions:
//! - It checks the integrity of your archive.
//! - It helps you process files before adding them to your archive.
//!    - Renames files that you previously processed and decided not to add,
//!      adding DELETED to the filename.
//!    - Renames files that already exist in your archive,
//!      adding DUPE to the filename.
//!    - Renames files that already exist in your archive but their contents
//!      are different, adding CHANGED to the filename.
//!    - Renames files that already exist in your archive, but their names
//!      or dates are different, adding METADATA to the filename.
//!    - Leaves new files untouched.
//!    - Remembers files that you delete.
//!
//! ## Install
//! ```text
//! $ cargo install deduposaur
//! ```
//!
//! ## Create a New Archive
//! First create an empty `deduposaur.archive_metadata.json` file:
//! ```text
//! $ cd /my_archive
//! $ ls
//! 1.jpg 2.jpg 3.jpg 4.jpg 5.jpg
//! $ touch deduposaur.archive_metadata.json
//! ```
//! Then run `deduposaur`:
//! ```text
//! $ deduposaur --archive=.
//! 1.jpg is new
//! 2.jpg is new
//! 3.jpg is new
//! 4.jpg is new
//! 5.jpg is new
//! Verified /my_archive
//! $
//! ```
//!
//! ## Check Your Archive
//! To check your archive, simply run `deduposaur` again:
//! ```text
//! $ deduposaur --archive=/my_archive
//! Verified /my_archive
//! $
//! ```
//!
//! ## Update your Archive
//! After updating your archive, run `deduposaur` again and respond to the prompts:
//! ```text
//! $ cd /my_archive
//! $ ls
//! deduposaur.archive_metadata.json 1.jpg 2.jpg 3.jpg 4.jpg 5.jpg
//! $ mv 2.jpg 2.hawaii.jpg
//! $ rm 3.jpg
//! $ echo 'corrupted' > 4.jpg
//! $ touch 5.jpg
//! $ deduposaur --archive=.
//! WARNING 2.jpg is renamed to 2.hawaii.jpg
//! Accept (y/n) or revert (r)? y
//! WARNING 3.jpg is deleted
//! Accept change? (y/n) y
//! WARNING 4.jpg is changed
//! Accept change? (y/n) n
//! WARNING 5.jpg mtime changed 2021-07-10T12:30:00-0700 -> 2021-07-20T15:11:03-0700
//! Accept (y/n) or revert (r)? r
//! $ cp /another_backup/4.jpg .
//! $ deduposaur --archive=.
//! Verified .
//! ```
//!
//! ## Add Files to Your Archive
//! First, run `deduposaur` and it will record metadata of new files and rename known files:
//! ```text
//! $ cd /new_files
//! $ ls
//! 1.jpg 2.jpg 3.jpg 4.jpg 5.jpg 6.jpg 7.jpg
//! $ deduposaur --archive=/my_archive --process=.
//! Verified /my_archive
//! Created deduposaur.process_metadata.json
//! Renamed DUPE.1.jpg - /my_archive/1.jpg
//! Renamed DUPE.2.jpg - /my_archive/2.hawaii.jpg
//! Renamed DELETED.3.jpg
//! Renamed CHANGED.4.jpg - /my_archive/4.jpg
//! Renamed METADATA.5.jpg - /my_archive/5.jpg
//! $
//! ```
//!
//! Second, go through through the files.
//! Move some to your archive and delete the rest.
//! ```text
//! $ rm DUPE.1.jpg
//! $ rm DUPE.2.jpg
//! $ rm DELETED.3.jpg
//! $ mv CHANGED.4.jpg /my_archive/4.jpg
//! $ rm METADATA.5.jpg
//! $ mv 6.jpg /my_archive/
//! $ rm 7.jpg
//! $ ls
//! deduposaur.process_metadata.json
//! $
//! ```
//!
//! Finally, run `deduposaur` again to update your archive and remember the deleted files.
//! ```text
//! $ deduposaur --archive=/my_archive --process=.
//! /my_archive/4.jpg is replaced by 4.jpg
//! /my_archive/6.jpg is new
//! Verified /my_archive
//! METADATA.5.jpg was deleted
//! 7.jpg was deleted
//! Deleting deduposaur.process_metadata.json since it is now empty.
//! $ ls
//! $
//! ```
//!
//! # TO DO
//! - Integration tests
//! - Make tests pass.
//! - Switch away from libraries with unsafe code:
//!    - `structopt` (WTF does command-line processing need unsafe code for?)
//!    - `serde_json`
//!    - `sha2`
//! - Check json file backups for corruption.  Automatically accept them.
use chrono::TimeZone;
use filetime::FileTime;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sha2::Digest;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::io::Read;
use std::iter::FromIterator;
use std::os::macos::fs::MetadataExt;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

const ARCHIVE_METADATA_JSON: &'static str = "deduposaur.archive_metadata.json";

#[derive(Debug, StructOpt)]
#[structopt(about)]
struct Opt {
    /// Path to the archive dir.
    /// You must add an empty 'deduposaur.archive_metadata.json' file to the dir.
    #[structopt(long, parse(from_os_str))]
    archive: PathBuf,
    /// Dir with files to process.
    /// Renames files inside this dir.
    /// Automatically creates a 'deduposaur.process_metadata.json' file and
    /// deletes it when all files are processed.
    /// When using this option, run the command one last time
    /// so it can record deleted files.
    #[structopt(long, parse(from_os_str))]
    process: Option<PathBuf>,
}

pub fn read_json_file<T: for<'a> Deserialize<'a> + Default>(path: &Path) -> Result<T, String> {
    let reader = std::fs::File::open(path)
        .map_err(|e| format!("error reading {}: {}", path.to_string_lossy(), e))?;
    let metadata = reader
        .metadata()
        .map_err(|e| format!("error reading {}: {}", path.to_string_lossy(), e))?;
    if metadata.len() == 0 {
        return Ok(Default::default());
    }
    serde_json::from_reader(reader)
        .map_err(|e| format!("error reading {}: {}", path.to_string_lossy(), e))
}

pub fn write_json_file(value: &impl Serialize, path: &Path) -> Result<(), String> {
    let writer = std::fs::File::create(path)
        .map_err(|e| format!("error writing {}: {}", path.to_string_lossy(), e))?;
    serde_json::to_writer(writer, value)
        .map_err(|e| format!("error writing {}: {}", path.to_string_lossy(), e))
}

#[serde_as]
#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct FileDigest(#[serde_as(as = "serde_with::hex::Hex")] [u8; 32]);
impl Debug for FileDigest {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "FileDigest({})", hex::encode(&self.0))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct FileRecord {
    path: String,
    mtime: i64,
    digest: FileDigest,
    #[serde(skip)]
    processed: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ArchiveMetadata {
    expected: Vec<RefCell<FileRecord>>,
    deleted: Vec<FileRecord>,
}
impl Default for ArchiveMetadata {
    fn default() -> Self {
        ArchiveMetadata {
            expected: Vec::new(),
            deleted: Vec::new(),
        }
    }
}

fn read_file_digest(path: &Path) -> Result<FileDigest, String> {
    let mut reader = std::fs::File::open(path)
        .map_err(|e| format!("error reading {}: {}", path.to_string_lossy(), e))?;
    let mut hasher = sha2::Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let num_bytes_read = reader
            .read(&mut buffer)
            .map_err(|e| format!("error reading {}: {}", path.to_string_lossy(), e))?;
        if num_bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..num_bytes_read]);
    }
    Ok(FileDigest(hasher.finalize().into()))
}

fn walk_dir(path: &Path, records: &mut Vec<FileRecord>) -> Result<(), String> {
    let mut dirs: Vec<PathBuf> = vec![path.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry_result in dir
            .read_dir()
            .map_err(|e| format!("error reading dir {}: {}", dir.to_string_lossy(), e))?
        {
            let entry = entry_result
                .map_err(|e| format!("error reading dir {}: {}", dir.to_string_lossy(), e))?;
            if entry.path().starts_with(ARCHIVE_METADATA_JSON) {
                continue;
            }
            let metadata = entry
                .metadata()
                .map_err(|e| format!("error reading {}: {}", entry.path().to_string_lossy(), e))?;
            if metadata.is_dir() {
                dirs.push(entry.path());
            } else if metadata.is_file() {
                records.push(FileRecord {
                    path: entry
                        .path()
                        .strip_prefix(path)
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    mtime: metadata.st_mtime(),
                    digest: read_file_digest(&entry.path())?,
                    processed: false,
                });
            } else {
                println!(
                    "WARNING Ignoring non-file {}",
                    entry.path().to_string_lossy()
                );
            }
        }
    }
    Ok(())
}

pub fn read_byte_from_stdin() -> Result<u8, String> {
    std::io::stdin()
        .bytes()
        .next()
        .ok_or_else(|| "stdin closed".to_string())?
        .map_err(|e| format!("error reading stdin: {}", e))
}

#[derive(PartialEq)]
enum PromptResponse {
    Yes,
    No,
}
impl PromptResponse {
    pub fn prompt_and_read() -> Result<PromptResponse, String> {
        loop {
            println!("Accept change? (y/n) ");
            match read_byte_from_stdin()? {
                b'y' => return Ok(PromptResponse::Yes),
                b'n' => return Ok(PromptResponse::No),
                _ => {}
            }
        }
    }
}

enum PromptWithRevertResponse {
    Yes,
    No,
    Revert,
}
impl PromptWithRevertResponse {
    pub fn prompt_and_read() -> Result<PromptWithRevertResponse, String> {
        loop {
            println!("Accept (y/n) or revert (r)? ");
            match read_byte_from_stdin()? {
                b'y' => return Ok(PromptWithRevertResponse::Yes),
                b'n' => return Ok(PromptWithRevertResponse::No),
                b'r' => return Ok(PromptWithRevertResponse::Revert),
                _ => {}
            }
        }
    }
}

fn main() -> Result<(), Box<String>> {
    let opt: Opt = Opt::from_args();
    if opt.archive.as_path().as_os_str().is_empty() {
        panic!("expected path, got empty string '--archive='");
    }
    if let Some(process) = opt.process {
        if process.as_path().as_os_str().is_empty() {
            panic!("expected path, got empty string '--process='");
        }
    }
    let mut all_ok = true;
    let archive_metadata_path = opt.archive.join(ARCHIVE_METADATA_JSON);
    let mut archive_metadata: ArchiveMetadata = read_json_file(&archive_metadata_path)?;
    let mut actual_records: Vec<FileRecord> = Vec::new();
    walk_dir(&opt.archive, &mut actual_records)?;
    //writeln!(stderr(), "actual_records {:?}", actual_records).unwrap();
    // Check for existing and changed files.
    {
        let index: HashMap<String, &RefCell<FileRecord>> = HashMap::from_iter(
            archive_metadata
                .expected
                .iter()
                .map(|cell| (cell.borrow().path.clone(), cell)),
        );
        for actual in actual_records.iter_mut().filter(|elem| !elem.processed) {
            if let Some(expected_cell) = index.get(&actual.path) {
                actual.processed = true;
                let mut expected = expected_cell.borrow_mut();
                expected.processed = true;
                if expected.digest != actual.digest {
                    println!("WARNING {} is changed", actual.path);
                    if PromptResponse::prompt_and_read()? == PromptResponse::Yes {
                        expected.digest.0 = actual.digest.0;
                        expected.mtime = actual.mtime;
                    } else {
                        all_ok = false;
                    }
                } else if expected.mtime != actual.mtime {
                    println!(
                        "WARNING {} mtime changed {} -> {}",
                        actual.path,
                        chrono::Local.timestamp(expected.mtime, 0).to_rfc3339(),
                        chrono::Local.timestamp(actual.mtime, 0).to_rfc3339(),
                    );
                    match PromptWithRevertResponse::prompt_and_read()? {
                        PromptWithRevertResponse::Yes => {
                            expected.mtime = actual.mtime;
                        }
                        PromptWithRevertResponse::No => {
                            all_ok = false;
                        }
                        PromptWithRevertResponse::Revert => {
                            let path = opt.archive.join(&actual.path);
                            filetime::set_file_mtime(
                                &path,
                                FileTime::from_unix_time(expected.mtime, 0),
                            )
                            .map_err(|e| format!("error setting {:?} mtime: {}", path, e))?
                        }
                    }
                }
            }
        }
    }
    // Check for renamed files.
    {
        let index: HashMap<(i64, FileDigest), &RefCell<FileRecord>> = HashMap::from_iter(
            archive_metadata
                .expected
                .iter()
                .filter(|elem| !elem.borrow().processed)
                .map(|cell| ((cell.borrow().mtime, cell.borrow().digest.clone()), cell)),
        );
        for actual in actual_records.iter_mut().filter(|elem| !elem.processed) {
            if let Some(expected_cell) = index.get(&(actual.mtime, actual.digest.clone())) {
                actual.processed = true;
                let mut expected = expected_cell.borrow_mut();
                expected.processed = true;
                if expected.path != actual.path {
                    println!("WARNING {} is renamed to {}", expected.path, actual.path);
                    if PromptResponse::prompt_and_read()? == PromptResponse::Yes {
                        expected.path = actual.path.clone();
                    } else {
                        all_ok = false;
                    }
                }
            }
        }
    }
    // All remaining unprocessed expected files must have been deleted.
    let expected_copies: Vec<FileRecord> = archive_metadata
        .expected
        .iter()
        .filter(|elem| !elem.borrow().processed)
        .map(|elem| elem.borrow().clone())
        .collect();
    for expected_copy in expected_copies {
        println!("WARNING {} is deleted", expected_copy.path);
        if PromptResponse::prompt_and_read()? == PromptResponse::Yes {
            archive_metadata
                .expected
                .retain(|elem| *elem.borrow() != expected_copy);
            archive_metadata.deleted.push(expected_copy);
        } else {
            all_ok = false;
        }
    }
    // TODO(mleonhard) Add new files.
    // TODO(mleonhard) Write new archive_metadata file.
    if all_ok {
        println!("Verified {}", opt.archive.to_string_lossy());
    }
    let temp_archive_metadata_path = {
        let mut s = archive_metadata_path.clone().into_os_string();
        s.push(".tmp");
        PathBuf::from(s)
    };
    write_json_file(&archive_metadata, &temp_archive_metadata_path).unwrap();
    // TODO(mleonhard) Skip replacing file if they are identical.
    let backup_archive_metadata_path = {
        let mut s = archive_metadata_path.clone().into_os_string();
        s.push(format!(
            ".{}~",
            chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        ));
        PathBuf::from(s)
    };
    std::fs::rename(&archive_metadata_path, &backup_archive_metadata_path).unwrap();
    std::fs::rename(&temp_archive_metadata_path, &archive_metadata_path).unwrap();
    // TODO(mleonhard) Process files.
    Ok(())
}
