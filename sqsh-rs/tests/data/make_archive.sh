#!/bin/bash

dest_dir="$(dirname "$0")"

# Handle both linux and macos (https://unix.stackexchange.com/questions/30091/fix-or-alternative-for-mktemp-in-os-x)
tempdir=$(mktemp -d 2>/dev/null || mktemp -d -t 'archive_tmp')
trap 'rm -rf $tempdir' EXIT

mksquashfs "$tempdir" "$dest_dir/test.sqsh" -pf "$dest_dir/pseudo-definitions.txt" -reproducible -all-time 1000 -mkfs-time 2000 -root-uid 22 -root-gid 33 -noappend