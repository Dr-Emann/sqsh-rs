# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2](https://github.com/Dr-Emann/sqsh-rs/compare/sqsh-rs-v0.1.1...sqsh-rs-v0.1.2) - 2024-08-01

### Added
- add ability to create an inode with `Inode::new`

### Fixed
- ensure all structs are namable

## [0.1.1](https://github.com/Dr-Emann/sqsh-rs/compare/sqsh-rs-v0.1.0...sqsh-rs-v0.1.1) - 2024-08-01

### Added
- impl From<sqsh::Error> for std::io::Error

### Fixed
- avoid deprecated sqsh_xattr_iterator_value_size fn

### Other
- Add readme, with example test
- release

## [0.1.0](https://github.com/Dr-Emann/sqsh-rs/releases/tag/sqsh-rs-v0.1.0) - 2024-08-01

### Other
- try to fix release
- cleanup changelog
- release
- Publishing prep
- Rexport source
- seal Source trait
- custom error type for unknown xattr types
- xattr entry borrows iterator directly
- Stack alocated cstr when possible
- clippy suggestions
- some path_resolver tests and new methods
- not all mode_t are u16
- Add traverse::Entry::file_type
- High level traverse bindings
- Update to 1.4.0 of libsqsh
- Rename tree walker to path resolver
- Update libsqsh
- Use a lifetime for the archive rather than a type
- Make archive generic over a source type
- Add test for compression options
- Add compression options
- Add skip method on file reader
- Expose compression type in superblock
- Use options for optional superblock fields
- Return an 'entry' from xattr iteration
- Use consistent lifetime param names
- Introduce Inode type
- Transpose Option<Result>
- implement id_table get
- Add example for non-recursive iteration
- Correct lifetimes of iterators
- walker clarifications
- Add recursive list example
- return an 'entry' type from directory iterator
- misc
- Initial commit

