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
//! $ deduposaur --verify_and_update_archive=.
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
//! $ deduposaur --verify_and_update_archive=/my_archive
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
//! $ deduposaur --verify_and_update_archive=.
//! WARNING 2.jpg is renamed to 2.hawaii.jpg
//! Accept (y/n) or revert (r)? y
//! WARNING 3.jpg is deleted
//! Accept change? (y/n) y
//! WARNING 4.jpg is changed
//! Accept change? (y/n) n
//! WARNING 5.jpg mtime changed 2021-07-10T12:30:00-0700 -> 2021-07-20T15:11:03-0700
//! Accept (y/n) or revert (r)? r
//! $ cp /another_backup/4.jpg .
//! $ deduposaur --verify_and_update_archive=.
//! Verified .
//! ```
//!
//! ## Add Files to Your Archive
//! First, run `deduposaur` on the new files:
//! ```text
//! $ cd /new_files
//! $ ls
//! 1.jpg 2.jpg 3.jpg 4.jpg 5.jpg 6.jpg 7.jpg
//! $ deduposaur --verify_and_update_archive=/my_archive --rename_known_files_check_new_and_remember_deletions=.
//! Verified /my_archive
//! Created deduposaur.new_files_metadata.json
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
//! deduposaur.new_files_metadata.json
//! $
//! ```
//!
//! Finally, run `deduposaur` again to update your archive and remember the deleted files.
//! ```text
//! $ deduposaur --verify_and_update_archive=/my_archive --rename_known_files_check_new_and_remember_deletions=/new_files
//! /my_archive/4.jpg is replaced by 4.jpg
//! /my_archive/6.jpg is new
//! Verified /my_archive
//! METADATA.5.jpg was deleted
//! 7.jpg was deleted
//! Deleting deduposaur.new_files_metadata.json since it is now empty.
//! $ ls
//! $ cd ..
//! $ rmdir /new_files
//! $
//! ```
//!
//! # TO DO
//! - Integration tests
//! - Make tests pass.
fn main() {
    println!("Deduposaur");
}
