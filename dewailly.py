#!/usr/bin/env python
"""
Dewailly is a tool to help you add files to your personal archive.  It checks the integrity of your archive.  It also helps you process new files by checking if they already exist in your archive (duplicates) or if you have previously processed them and decided not to add them (deletions).

To verify your archive:
$ cd /my_archive
$ ls
archive_metadata.json.txt Movies/ Music/ Photos/ Projects/
$ dewailly.py --verify-archive-and-update-metadata=.
Read 12891 files from /my_archive/dewailly.archive_metadata.json.txt
Verified all files exist and their contents and metadata are unchanged.
No new files found.
$

To prepare to add some new files to your archive:
$ cd /new_files
$ dwailly.py --verify-archive-and-update-metadata=/my_archive --rename_known_files_check_new_and_remember_deletions=.
Read 12891 files from /my_archive/dewailly.archive_metadata.json.txt
Read 6273 deletions from /my_archive/dewailly.deletions.json.txt
File /new_files/dewailly.new_files_metadata.json.txt does not exist.  Creating it.
Found 276 files under /new_files/
Renamed 132 known files: 129 deleted, 3 changed, 0 content changed, and 0 metadata changed.
Remembered 144 new files in /new_files/dewailly.new_files_metadata.json.txt
$

Now go through through the new and renamed files.  Move some to /my_archive/ and delete the rest.  Then run the program again to update your archive's metdata file and remember the deletions.

$ dwailly.py --verify-archive-and-update-metadata=/my_archive --rename_known_files_check_new_and_remember_deletions=.
Read 12891 files from /my_archive/dewailly.archive_metadata.json.txt
Verified all files exist and their contents and metadata are unchanged.
Found 76 new files under /my_archive.  Remembered them in /my_archive/dewailly.archive_metadata.json.txt
Read 6273 deletions from /my_archive/dewailly.deletions.json.txt
Read 144 files from /new_files/dewailly.new_files_metadata.json.txt
Found 0 files under /new_files/
Remembering 68 deletions in /my_archive/dewailly.deletions.json.txt
Deleting /new_files/new_files_metadata.json.txt since it is now empty.
$
"""
import argparse
import getopt
import hashlib
import json
import os
import os.path
import sys

def parseArgs(argv):
    parser = argparse.ArgumentParser(
        description='Process some integers.',
        prog=argv[0])
    parser.add_argument(
        '--verify-archive-and-update-metadata',
        dest='archive_dir',
        required=True,
        help='path of archive directory')
    parser.add_argument(
        '--rename_known_files_check_new_and_remember_deletions',
        dest='new_files_dir',
        required=False,
        help='path of directory with new files')
    parsed = parser.parse_args(argv[1:])
    return (os.path.abspath(parsed.archive_dir),
            (os.path.abspath(parsed.new_files_dir)
             if parsed.new_files_dir
             else None))

def raiseError(e):
    raise e

def scanDirTree(top):
    entries = []
    for dirpath, _, filenames in os.walk(top, onerror=raiseError):
        for filename in filenames:
            filepath = os.path.join(dirpath, filename)
            if os.path.islink(filepath) or not os.path.isfile(filepath):
                sys.stderr.write('Skipping non-file %s\n' % (repr(filepath),))
                continue
            with open(filepath, "rb") as f:
                entries.append({
                    'sha256': hashlib.sha256(f.read()).hexdigest(),
                    'filepath': os.path.relpath(filepath, top),
                    'ctime': int(os.path.getctime(filepath)),
                    'mtime': int(os.path.getmtime(filepath)),
                    'size': os.path.getsize(filepath),
                    })
    return entries

def doArchive(archive_dir):
    if not os.path.isdir(archive_dir):
        raise ValueError('archive_dir is not a directory', archive_dir)
    metadata_path = os.path.join(archive_dir, 'dewailly.archive_metadata.json.txt')
    if os.path.exists(metadata_path):
        with open(metadata_path) as f:
            metadata = json.load(f)
            print 'Read %s files from %s' % (len(metadata), metadata_path)
    else:
        print 'Creating file %s' % (metadata_path,)
        metadata = {}
    print scanDirTree(archive_dir)
    # Verified all files exist and their contents and metadata are unchanged.
    # No new files found.


def doNew(arg):
    print 'do new', arg


if __name__ == '__main__':
    archive_dir, new_files_dir = parseArgs(sys.argv)
    if new_files_dir and (
        not os.path.relpath(archive_dir, new_files_dir).startswith('..') or
        not os.path.relpath(new_files_dir, archive_dir).startswith('..')):
        raise ValueError('one directory contains the other', archive_dir, new_files_dir)
    doArchive(archive_dir)
    if new_files_dir:
        doNew(archive_dir, new_files_dir)
