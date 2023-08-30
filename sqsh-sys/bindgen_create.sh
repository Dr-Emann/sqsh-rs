#!/bin/sh

cd "$(dirname "$0")" || exit 1

bindgen \
  --no-layout-tests \
  --allowlist-type='(?i)sqsh.*' \
  --allowlist-var='(?i)sqsh.*' \
  --allowlist-function='(?i)sqsh.*' \
  --default-enum-style newtype \
  --default-alias-style type_alias \
  --enable-function-attribute-detection \
  --blocklist-type 'FILE|mode_t|fpos_t|time_t|__.*' \
  --raw-line 'use libc::{FILE, mode_t, time_t};' \
  --no-doc-comments \
  --sort-semantically \
  src/libsqsh/include/sqsh.h > src/bindings.rs