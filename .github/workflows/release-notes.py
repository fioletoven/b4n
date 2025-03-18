#!/usr/bin/env python3
# based on https://github.com/crate-ci/cargo-release/blob/v0.25.17/.github/workflows/release-notes.py

import argparse, pathlib

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True)
    parser.add_argument("-o", "--output", type=pathlib.Path, required=True)
    args = parser.parse_args()

    input = pathlib.Path("CHANGELOG.md")
    with input.open() as fh:
        changelog = fh.readlines()
    version = args.tag.lstrip("v")

    notes = []
    for line in changelog:
        if line.startswith("## ") and version in line:
            notes.append(line)
        elif notes and line.startswith("## "):
            break
        elif notes:
            notes.append(line)

    args.output.write_text("".join(notes).strip())

if __name__ == "__main__":
    main()
