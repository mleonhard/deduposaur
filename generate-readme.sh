#!/usr/bin/env bash
set -e
usage() {
  echo "$(basename "$0")": ERROR: "$@" >&2
  echo usage: "$(basename "$0")" '[--directory DIRECTORY] [--filename FILENAME]' >&2
  exit 1
}

directory=.
filename=Readme.md
while [ $# -gt 0 ]; do
  case "$1" in
  --directory)
    shift
    [ -n "$1" ] || usage "missing parameter to --directory argument"
    directory="$1"
    ;;
  --filename)
    shift
    filename="$1"
    [ -n "$1" ] || usage "missing parameter to --filename argument"
    echo -e "$filename" | tr '\r\n\t' '   ' | grep --quiet -E '\.md|\.tmp$' || usage "argument to --filename parameter must end in .md or .tmp"
    ;;
  '') break ;;
  *) usage "bad argument '$1'" ;;
  esac
  shift
done

cd "$directory"
echo "Running Cargo Readme."
echo "PWD=$(pwd)"
set -x
cargo readme --no-title --no-indent-headings --output "$filename"
set +x

if grep --quiet '//! ## Cargo Geiger Safety Report' src/main.rs; then
  echo ''
  echo "Running Cargo Geiger."
  echo "PWD=$(pwd)"
  if [ ! -f "src/main.rs" ]; then
    echo "not found: src/main.rs" >&2
    exit 1
  fi
  time (
    set -x
    cargo geiger --update-readme --readme-path "$filename" --output-format GitHubMarkdown
    set +x
    echo -n "Cargo Geiger done."
  )
fi
