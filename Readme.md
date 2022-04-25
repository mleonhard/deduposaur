[![crates.io version](https://img.shields.io/crates/v/deduposaur.svg)](https://crates.io/crates/deduposaur)
[![license: Apache 2.0](https://raw.githubusercontent.com/mleonhard/deduposaur/main/license-apache-2.0.svg)](https://github.com/mleonhard/deduposaur/blob/main/LICENSE)
[![unsafe forbidden](https://gitlab.com/leonhard-llc/ops/-/raw/main/unsafe-forbidden.svg)](https://github.com/rust-secure-code/safety-dance/)

Deduposaur is a command-line program to help you add files to your personal archive.

Functions:
- It checks the integrity of your archive.
- It helps you process files before adding them to your archive.
   - Renames files that you previously processed and decided not to add,
     adding DELETED to the filename.
   - Renames files that already exist in your archive,
     adding DUPE to the filename.
   - Renames files that already exist in your archive but their contents
     are different, adding CHANGED to the filename.
   - Renames files that already exist in your archive, but their names
     or dates are different, adding METADATA to the filename.
   - Leaves new files untouched.
   - Remembers files that you delete.

## Install
```
$ cargo install deduposaur
```

## Create a New Archive
First create an empty `deduposaur.archive_metadata.json` file:
```
$ cd /my_archive
$ ls
1.jpg 2.jpg 3.jpg 4.jpg 5.jpg
$ touch deduposaur.archive_metadata.json
```
Then run `deduposaur`:
```
$ deduposaur --archive=.
1.jpg is new
2.jpg is new
3.jpg is new
4.jpg is new
5.jpg is new
Verified /my_archive
$
```

## Check Your Archive
To check your archive, simply run `deduposaur` again:
```
$ deduposaur --archive=/my_archive
Verified /my_archive
$
```

## Update your Archive
After updating your archive, run `deduposaur` again and respond to the prompts:
```
$ cd /my_archive
$ ls
deduposaur.archive_metadata.json 1.jpg 2.jpg 3.jpg 4.jpg 5.jpg
$ mv 2.jpg 2.hawaii.jpg
$ rm 3.jpg
$ echo 'corrupted' > 4.jpg
$ touch 5.jpg
$ deduposaur --archive=.
WARNING 2.jpg is renamed to 2.hawaii.jpg
Accept (y/n) or revert (r)? y
WARNING 3.jpg is deleted
Accept change? (y/n) y
WARNING 4.jpg is changed
Accept change? (y/n) n
WARNING 5.jpg mtime changed 2021-07-10T12:30:00-0700 -> 2021-07-20T15:11:03-0700
Accept (y/n) or revert (r)? r
$ cp /another_backup/4.jpg .
$ deduposaur --archive=.
Verified .
```

## Add Files to Your Archive
First, run `deduposaur` and it will record metadata of new files and rename known files:
```
$ cd /new_files
$ ls
1.jpg 2.jpg 3.jpg 4.jpg 5.jpg 6.jpg 7.jpg
$ deduposaur --archive=/my_archive --process=.
Verified /my_archive
Created deduposaur.process_metadata.json
Renamed DUPE.1.jpg - /my_archive/1.jpg
Renamed DUPE.2.jpg - /my_archive/2.hawaii.jpg
Renamed DELETED.3.jpg
Renamed CHANGED.4.jpg - /my_archive/4.jpg
Renamed METADATA.5.jpg - /my_archive/5.jpg
$
```

Second, go through through the files.
Move some to your archive and delete the rest.
```
$ rm DUPE.1.jpg
$ rm DUPE.2.jpg
$ rm DELETED.3.jpg
$ mv CHANGED.4.jpg /my_archive/4.jpg
$ rm METADATA.5.jpg
$ mv 6.jpg /my_archive/
$ rm 7.jpg
$ ls
deduposaur.process_metadata.json
$
```

Finally, run `deduposaur` again to update your archive and remember the deleted files.
```
$ deduposaur --archive=/my_archive --process=.
/my_archive/4.jpg is replaced by 4.jpg
/my_archive/6.jpg is new
Verified /my_archive
METADATA.5.jpg was deleted
7.jpg was deleted
Deleting deduposaur.process_metadata.json since it is now empty.
$ ls
$
```

# TO DO
- Find a way to make it run faster.  Re-validating the archive takes a long time.
  Consider skipping validating the archive when the user specifies `--process`.
- Report duplicates in archive and process dir
- DONE - Integration tests
- DONE - Make tests pass.
- Switch away from libraries with unsafe code:
   - `structopt` (Why does command-line processing need unsafe code for?)
   - `serde_json`
   - `sha2`

# Cargo Geiger Safety Report
```

Metric output format: x/y
    x = unsafe code used by the build
    y = total unsafe code found in the crate

Symbols: 
    🔒  = No `unsafe` usage found, declares #![forbid(unsafe_code)]
    ❓  = No `unsafe` usage found, missing #![forbid(unsafe_code)]
    ☢️  = `unsafe` usage found

Functions  Expressions  Impls  Traits  Methods  Dependency

0/0        0/0          0/0    0/0     0/0      ❓  deduposaur 0.1.0
1/1        44/90        2/2    0/0     0/0      ☢️  ├── chrono 0.4.19
1/20       10/365       0/2    0/0     5/40     ☢️  │   ├── libc 0.2.124
0/0        0/0          0/0    0/0     0/0      ❓  │   ├── num-integer 0.1.44
                                                       │   │   [build-dependencies]
0/0        0/0          0/0    0/0     0/0      ❓  │   │   └── autocfg 1.1.0
0/0        4/10         0/0    0/0     0/0      ☢️  │   │   └── num-traits 0.2.14
                                                       │   │       [build-dependencies]
0/0        0/0          0/0    0/0     0/0      ❓  │   │       └── autocfg 1.1.0
0/0        4/10         0/0    0/0     0/0      ☢️  │   ├── num-traits 0.2.14
0/0        5/5          0/0    0/0     0/0      ☢️  │   ├── serde 1.0.136
0/0        0/0          0/0    0/0     0/0      ❓  │   │   └── serde_derive 1.0.136
0/0        12/12        0/0    0/0     3/3      ☢️  │   │       ├── proc-macro2 1.0.37
0/0        0/0          0/0    0/0     0/0      🔒  │   │       │   └── unicode-xid 0.2.2
0/0        0/0          0/0    0/0     0/0      ❓  │   │       ├── quote 1.0.18
0/0        12/12        0/0    0/0     3/3      ☢️  │   │       │   └── proc-macro2 1.0.37
0/0        47/47        3/3    0/0     2/2      ☢️  │   │       └── syn 1.0.91
0/0        12/12        0/0    0/0     3/3      ☢️  │   │           ├── proc-macro2 1.0.37
0/0        0/0          0/0    0/0     0/0      ❓  │   │           ├── quote 1.0.18
0/0        0/0          0/0    0/0     0/0      🔒  │   │           └── unicode-xid 0.2.2
1/1        218/218      0/0    0/0     0/0      ☢️  │   └── time 0.1.44
1/20       10/365       0/2    0/0     5/40     ☢️  │       └── libc 0.2.124
0/0        35/78        0/0    0/0     0/0      ☢️  ├── filetime 0.2.16
0/0        0/0          0/0    0/0     0/0      ❓  │   ├── cfg-if 1.0.0
1/20       10/365       0/2    0/0     5/40     ☢️  │   └── libc 0.2.124
0/0        0/0          0/0    0/0     0/0      ❓  ├── hex 0.4.3
0/0        5/5          0/0    0/0     0/0      ☢️  │   └── serde 1.0.136
0/0        5/5          0/0    0/0     0/0      ☢️  ├── serde 1.0.136
0/0        4/7          0/0    0/0     0/0      ☢️  ├── serde_json 1.0.79
0/0        7/7          0/0    0/0     0/0      ☢️  │   ├── itoa 1.0.1
7/9        587/723      0/0    0/0     2/2      ☢️  │   ├── ryu 1.0.9
0/0        5/5          0/0    0/0     0/0      ☢️  │   └── serde 1.0.136
0/0        6/6          0/0    0/0     0/0      ☢️  ├── serde_with 1.13.0
1/1        44/90        2/2    0/0     0/0      ☢️  │   ├── chrono 0.4.19
0/0        0/0          0/0    0/0     0/0      ❓  │   ├── doc-comment 0.3.3
0/0        0/0          0/0    0/0     0/0      ❓  │   ├── hex 0.4.3
0/1        0/1          0/0    0/0     0/0      ❓  │   ├── rustversion 1.0.6
0/0        5/5          0/0    0/0     0/0      ☢️  │   ├── serde 1.0.136
0/0        4/7          0/0    0/0     0/0      ☢️  │   ├── serde_json 1.0.79
0/0        0/0          0/0    0/0     0/0      🔒  │   └── serde_with_macros 1.5.2
0/0        0/0          0/0    0/0     0/0      ❓  │       ├── darling 0.13.4
0/0        0/0          0/0    0/0     0/0      ❓  │       │   ├── darling_core 0.13.4
0/0        0/0          0/0    0/0     0/0      ❓  │       │   │   ├── fnv 1.0.7
0/0        0/0          0/0    0/0     0/0      ❓  │       │   │   ├── ident_case 1.0.1
0/0        12/12        0/0    0/0     3/3      ☢️  │       │   │   ├── proc-macro2 1.0.37
0/0        0/0          0/0    0/0     0/0      ❓  │       │   │   ├── quote 1.0.18
0/0        0/0          0/0    0/0     0/0      🔒  │       │   │   ├── strsim 0.10.0
0/0        47/47        3/3    0/0     2/2      ☢️  │       │   │   └── syn 1.0.91
0/0        0/0          0/0    0/0     0/0      ❓  │       │   └── darling_macro 0.13.4
0/0        0/0          0/0    0/0     0/0      ❓  │       │       ├── darling_core 0.13.4
0/0        0/0          0/0    0/0     0/0      ❓  │       │       ├── quote 1.0.18
0/0        47/47        3/3    0/0     2/2      ☢️  │       │       └── syn 1.0.91
0/0        12/12        0/0    0/0     3/3      ☢️  │       ├── proc-macro2 1.0.37
0/0        0/0          0/0    0/0     0/0      ❓  │       ├── quote 1.0.18
0/0        47/47        3/3    0/0     2/2      ☢️  │       └── syn 1.0.91
8/8        202/202      0/0    0/0     0/0      ☢️  ├── sha2 0.9.9
0/0        6/6          0/0    0/0     0/0      ☢️  │   ├── block-buffer 0.9.0
1/1        292/292      20/20  8/8     5/5      ☢️  │   │   └── generic-array 0.14.5
0/0        5/5          0/0    0/0     0/0      ☢️  │   │       ├── serde 1.0.136
0/0        0/0          0/0    0/0     0/0      🔒  │   │       └── typenum 1.15.0
                                                       │   │       [build-dependencies]
0/0        0/0          0/0    0/0     0/0      ❓  │   │       └── version_check 0.9.4
0/0        0/0          0/0    0/0     0/0      ❓  │   ├── cfg-if 1.0.0
0/1        0/14         0/0    0/0     0/0      ❓  │   ├── cpufeatures 0.2.2
0/0        0/0          0/0    0/0     0/0      🔒  │   ├── digest 0.9.0
1/1        292/292      20/20  8/8     5/5      ☢️  │   │   └── generic-array 0.14.5
0/0        0/0          0/0    0/0     0/0      ❓  │   └── opaque-debug 0.3.0
0/0        0/0          0/0    0/0     0/0      🔒  └── structopt 0.3.26
0/0        1/1          0/0    0/0     0/0      ☢️      ├── clap 2.34.0
0/0        32/32        0/0    0/0     0/0      ☢️      │   ├── ansi_term 0.12.1
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   └── serde 1.0.136
2/2        45/45        0/0    0/0     0/0      ☢️      │   ├── atty 0.2.14
1/20       10/365       0/2    0/0     5/40     ☢️      │   │   └── libc 0.2.124
0/0        0/0          0/0    0/0     0/0      ❓      │   ├── bitflags 1.3.2
0/0        0/0          0/0    0/0     0/0      ❓      │   ├── strsim 0.8.0
0/0        0/0          0/0    0/0     0/0      ❓      │   ├── textwrap 0.11.0
0/0        0/0          0/0    0/0     0/0      ❓      │   │   └── unicode-width 0.1.9
0/0        0/0          0/0    0/0     0/0      ❓      │   ├── unicode-width 0.1.9
0/0        0/0          0/0    0/0     0/0      ❓      │   └── vec_map 0.8.2
0/0        5/5          0/0    0/0     0/0      ☢️      │       └── serde 1.0.136
0/0        7/7          1/1    0/0     0/0      ☢️      ├── lazy_static 1.4.0
0/0        0/0          0/0    0/0     0/0      🔒      └── structopt-derive 0.4.18
0/0        0/0          0/0    0/0     0/0      ❓          ├── heck 0.3.3
0/0        0/0          0/0    0/0     0/0      ❓          │   └── unicode-segmentation 1.9.0
0/0        0/0          0/0    0/0     0/0      🔒          ├── proc-macro-error 1.0.4
                                                               │   [build-dependencies]
0/0        0/0          0/0    0/0     0/0      ❓          │   └── version_check 0.9.4
0/0        0/0          0/0    0/0     0/0      ❓          │   ├── proc-macro-error-attr 1.0.4
                                                               │   │   [build-dependencies]
0/0        0/0          0/0    0/0     0/0      ❓          │   │   └── version_check 0.9.4
0/0        12/12        0/0    0/0     3/3      ☢️          │   │   ├── proc-macro2 1.0.37
0/0        0/0          0/0    0/0     0/0      ❓          │   │   └── quote 1.0.18
0/0        12/12        0/0    0/0     3/3      ☢️          │   ├── proc-macro2 1.0.37
0/0        0/0          0/0    0/0     0/0      ❓          │   ├── quote 1.0.18
0/0        47/47        3/3    0/0     2/2      ☢️          │   └── syn 1.0.91
0/0        12/12        0/0    0/0     3/3      ☢️          ├── proc-macro2 1.0.37
0/0        0/0          0/0    0/0     0/0      ❓          ├── quote 1.0.18
0/0        47/47        3/3    0/0     2/2      ☢️          └── syn 1.0.91

21/44      1564/2168    26/28  8/8     17/52  

```
