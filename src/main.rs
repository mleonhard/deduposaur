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
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sha2::Digest;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::io::Read;
use std::io::Write;
use std::iter::FromIterator;
use std::os::macos::fs::MetadataExt;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

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

#[serde_as]
#[derive(Clone, Deserialize, PartialEq, Serialize)]
pub struct FileDigest(#[serde_as(as = "serde_with::hex::Hex")] [u8; 32]);
impl Debug for FileDigest {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "FileDigest({})", hex::encode(&self.0))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileRecord {
    path: String,
    mtime: i64,
    digest: FileDigest,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ArchiveMetadata {
    expected: Vec<FileRecord>,
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
                });
            } else {
                writeln!(
                    std::io::stderr(),
                    "WARNING Ignoring non-file {}",
                    entry.path().to_string_lossy()
                )
                .unwrap();
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<String>> {
    let opt: Opt = Opt::from_args();
    println!("{:?}", opt);
    if opt.archive.as_path().as_os_str().is_empty() {
        panic!("expected path, got empty string '--archive='");
    }
    if let Some(process) = opt.process {
        if process.as_path().as_os_str().is_empty() {
            panic!("expected path, got empty string '--process='");
        }
    }
    let mut all_ok = true;
    let archive_metadata_path = opt.archive.join("deduposaur.archive_metadata.json");
    let archive_metadata: ArchiveMetadata = read_json_file(&archive_metadata_path)?;
    let mut archive_files: Vec<FileRecord> = Vec::new();
    walk_dir(&opt.archive, &mut archive_files)?;
    // TODO(mleonhard) Warn about changed files.
    // TODO(mleonhard) Warn about deleted files.
    // TODO(mleonhard) Warn about renamed files.
    // TODO(mleonhard) Warn about files with changed mtime.
    // TODO(mleonhard) Add new files.
    // TODO(mleonhard) Write new archive_metadata file.
    // let expected_set = HashSet::from_iter(archive_metadata.expected.iter().cloned());
    // let actual_set = HashSet::from_iter(archive_files.iter().cloned());
    // for record in expected_set.difference(&actual_set) {
    //     writeln!(
    //         std::io::stderr(),
    //         "WARNING Ignoring non-file {}",
    //         entry.path().to_string_lossy()
    //     )
    //     .unwrap();
    // }
    if all_ok {
        println!("Verified {}", archive_metadata_path.to_string_lossy());
    }
    // TODO(mleonhard) Process files.
    Ok(())
}
