#!/bin/bash

cd "$(dirname "$0")" || exit 1

args=(
  --no-layout-tests
  --allowlist-item='(?i-u:sqsh).*'
  --default-enum-style newtype
  --default-alias-style type_alias
  --enable-function-attribute-detection
  --blocklist-type 'FILE|mode_t|fpos_t|time_t|__.*'
  --raw-line 'use libc::{FILE, mode_t, time_t};'
  --no-doc-comments
  --sort-semantically
  submodules/sqsh-tools/include/sqsh.h
  -o src/bindings.rs
  --
  -I submodules/sqsh-tools/include
)

bindgen "${args[@]}"