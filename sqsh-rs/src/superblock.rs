use crate::{error, Archive, InodeRef};
use bitflags::bitflags;
use sqsh_sys as ffi;
use std::fmt;
use std::ptr::NonNull;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Compression {
    id: ffi::SqshSuperblockCompressionId,
}

impl Compression {
    pub const GZIP: Self = Self {
        id: ffi::SqshSuperblockCompressionId::SQSH_COMPRESSION_GZIP,
    };

    pub const LZMA: Self = Self {
        id: ffi::SqshSuperblockCompressionId::SQSH_COMPRESSION_LZMA,
    };

    pub const LZO: Self = Self {
        id: ffi::SqshSuperblockCompressionId::SQSH_COMPRESSION_LZO,
    };

    pub const XZ: Self = Self {
        id: ffi::SqshSuperblockCompressionId::SQSH_COMPRESSION_XZ,
    };

    pub const LZ4: Self = Self {
        id: ffi::SqshSuperblockCompressionId::SQSH_COMPRESSION_LZ4,
    };

    pub const ZSTD: Self = Self {
        id: ffi::SqshSuperblockCompressionId::SQSH_COMPRESSION_ZSTD,
    };

    pub fn name(&self) -> Option<&'static str> {
        Some(match *self {
            Self::GZIP => "gzip",
            Self::LZMA => "lzma",
            Self::LZO => "lzo",
            Self::XZ => "xz",
            Self::LZ4 => "lz4",
            Self::ZSTD => "zstd",
            _ => return None,
        })
    }
}

impl fmt::Debug for Compression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name();
        let value: &dyn fmt::Debug = match &name {
            Some(name) => name,
            None => &self.id,
        };
        f.debug_tuple("Compression").field(value).finish()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CompressionOptions {
    Gzip {
        compression_level: u32,
        window_size: u16,
        strategies: GzipStrategies,
    },
    Xz {
        dictionary_size: u32,
        filters: XzFilters,
    },
    Lz4 {
        version: u32,
        flags: Lz4Flags,
    },
    Zstd {
        compression_level: u32,
    },
    Lzo {
        algorithm: LzoAlgorithm,
        compression_level: u32,
    },
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct GzipStrategies: u16 {
        const DEFAULT = 1 << 0;
        const FILTERED = 1 << 1;
        const HUFFMAN_ONLY = 1 << 2;
        const RLE = 1 << 3;
        const FIXED = 1 << 4;

        const _ = !0;
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct XzFilters: u16 {
        const X86 = 1 << 0;
        const POWERPC = 1 << 1;
        const IA64 = 1 << 2;
        const ARM = 1 << 3;
        const ARMTHUMB = 1 << 4;
        const SPARC = 1 << 5;

        const _ = !0;
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct Lz4Flags: u32 {
        const DEFAULT = 1 << 0;
        const HC = 1 << 1;

        const _ = !0;
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct LzoAlgorithm(u32);

impl LzoAlgorithm {
    const LZO1X_1: Self = Self(ffi::SqshLzoAlgorithm::SQSH_LZO_ALGORITHM_LZO1X_1.0);
    const LZO1X_1_11: Self = Self(ffi::SqshLzoAlgorithm::SQSH_LZO_ALGORITHM_LZO1X_1_11.0);
    const LZO1X_1_12: Self = Self(ffi::SqshLzoAlgorithm::SQSH_LZO_ALGORITHM_LZO1X_1_12.0);
    const LZO1X_1_15: Self = Self(ffi::SqshLzoAlgorithm::SQSH_LZO_ALGORITHM_LZO1X_1_15.0);
    const LZO1X_999: Self = Self(ffi::SqshLzoAlgorithm::SQSH_LZO_ALGORITHM_LZO1X_999.0);

    fn as_str(&self) -> Option<&'static str> {
        match *self {
            Self::LZO1X_1 => Some("lzo1x_1"),
            Self::LZO1X_1_11 => Some("lzo1x_1_11"),
            Self::LZO1X_1_12 => Some("lzo1x_1_12"),
            Self::LZO1X_1_15 => Some("lzo1x_1_15"),
            Self::LZO1X_999 => Some("lzo1x_999"),
            _ => None,
        }
    }
}

impl Default for LzoAlgorithm {
    fn default() -> Self {
        Self::LZO1X_999
    }
}

impl fmt::Debug for LzoAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.as_str();
        let value: &dyn fmt::Debug = match &name {
            Some(name) => name,
            None => &self.0,
        };
        f.debug_tuple("LzoAlgorithm").field(value).finish()
    }
}

impl Archive {
    pub fn compression_options(&self) -> error::Result<Option<CompressionOptions>> {
        struct RawCompressionOptions(NonNull<ffi::SqshCompressionOptions>);
        impl Drop for RawCompressionOptions {
            fn drop(&mut self) {
                unsafe { ffi::sqsh_compression_options_free(self.0.as_ptr()) };
            }
        }

        let superblock = self.superblock();
        if !superblock.has_compression_options() {
            return Ok(None);
        }
        let compression_options = unsafe {
            let mut err = 0;
            let raw = ffi::sqsh_compression_options_new(self.inner.as_ptr(), &mut err);
            let raw = match NonNull::new(raw) {
                Some(raw) => raw,
                None => return Err(error::new(err)),
            };
            RawCompressionOptions(raw)
        };

        Ok(Some(match superblock.compression_type() {
            Compression::GZIP => CompressionOptions::Gzip {
                compression_level: unsafe {
                    ffi::sqsh_compression_options_gzip_compression_level(
                        compression_options.0.as_ptr(),
                    )
                },
                window_size: unsafe {
                    ffi::sqsh_compression_options_gzip_window_size(compression_options.0.as_ptr())
                },
                strategies: GzipStrategies::from_bits_retain(unsafe {
                    ffi::sqsh_compression_options_gzip_strategies(compression_options.0.as_ptr()).0
                        as _
                }),
            },
            Compression::XZ => CompressionOptions::Xz {
                dictionary_size: unsafe {
                    ffi::sqsh_compression_options_xz_dictionary_size(compression_options.0.as_ptr())
                },
                filters: XzFilters::from_bits_retain(unsafe {
                    ffi::sqsh_compression_options_xz_filters(compression_options.0.as_ptr()).0 as _
                }),
            },
            Compression::LZ4 => CompressionOptions::Lz4 {
                version: unsafe {
                    ffi::sqsh_compression_options_lz4_version(compression_options.0.as_ptr())
                },
                flags: Lz4Flags::from_bits_retain(unsafe {
                    ffi::sqsh_compression_options_lz4_flags(compression_options.0.as_ptr())
                } as _),
            },
            Compression::ZSTD => CompressionOptions::Zstd {
                compression_level: unsafe {
                    ffi::sqsh_compression_options_zstd_compression_level(
                        compression_options.0.as_ptr(),
                    )
                },
            },
            Compression::LZO => CompressionOptions::Lzo {
                algorithm: LzoAlgorithm(unsafe {
                    ffi::sqsh_compression_options_lzo_algorithm(compression_options.0.as_ptr()).0
                        as _
                }),
                compression_level: unsafe {
                    ffi::sqsh_compression_options_lzo_compression_level(
                        compression_options.0.as_ptr(),
                    )
                },
            },
            _ => return Ok(None),
        }))
    }
}

#[derive(Copy, Clone)]
pub struct Superblock<'archive> {
    inner: &'archive ffi::SqshSuperblock,
}

impl<'archive> Superblock<'archive> {
    pub(crate) unsafe fn new(inner: *const ffi::SqshSuperblock) -> Self {
        let inner = inner.as_ref().expect("null superblock pointer");
        Self { inner }
    }

    pub fn compression_type(&self) -> Compression {
        let compression_id = unsafe { ffi::sqsh_superblock_compression_id(self.inner) };
        Compression { id: compression_id }
    }

    /// Retrieves the number of inodes in an archive.
    pub fn inode_count(&self) -> u32 {
        unsafe { ffi::sqsh_superblock_inode_count(self.inner) }
    }

    /// Retrieves the number of ids in an archive.
    pub fn id_count(&self) -> u16 {
        unsafe { ffi::sqsh_superblock_id_count(self.inner) }
    }

    /// Retrieves the number of fragment entries in an archive.
    pub fn fragment_entry_count(&self) -> u32 {
        unsafe { ffi::sqsh_superblock_fragment_entry_count(self.inner) }
    }

    /// Retrieves the start offset of the inode table in a superblock context.
    pub fn inode_table_start(&self) -> u64 {
        unsafe { ffi::sqsh_superblock_inode_table_start(self.inner) }
    }

    /// Retrieves the start offset of the directory table in a superblock context.
    pub fn directory_table_start(&self) -> u64 {
        unsafe { ffi::sqsh_superblock_directory_table_start(self.inner) }
    }

    /// Retrieves the start offset of the fragment table in a superblock context.
    pub fn fragment_table_start(&self) -> Option<u64> {
        self.has_fragments()
            .then(|| unsafe { ffi::sqsh_superblock_fragment_table_start(self.inner) })
    }

    /// Retrieves the start offset of the export table in a superblock context.
    pub fn export_table_start(&self) -> Option<u64> {
        self.has_export_table()
            .then(|| unsafe { ffi::sqsh_superblock_export_table_start(self.inner) })
    }

    /// Retrieves the start offset of the id table in a superblock context.
    pub fn id_table_start(&self) -> u64 {
        unsafe { ffi::sqsh_superblock_id_table_start(self.inner) }
    }

    /// Retrieves the start offset of the xattr id table in a superblock context.
    pub fn xattr_id_table_start(&self) -> Option<u64> {
        self.has_xattr_table()
            .then(|| unsafe { ffi::sqsh_superblock_xattr_id_table_start(self.inner) })
    }

    /// Retrieves the reference of the root inode in a superblock context.
    pub fn root_inode_ref(&self) -> InodeRef {
        let inode_ref = unsafe { ffi::sqsh_superblock_inode_root_ref(self.inner) };
        InodeRef(inode_ref)
    }

    /// Checks if a superblock context has fragment table.
    pub fn has_fragments(&self) -> bool {
        unsafe { ffi::sqsh_superblock_has_fragments(self.inner) }
    }

    /// Checks if a superblock context has an export table.
    pub fn has_export_table(&self) -> bool {
        unsafe { ffi::sqsh_superblock_has_export_table(self.inner) }
    }

    /// Checks if a superblock context has an xattr table.
    pub fn has_xattr_table(&self) -> bool {
        unsafe { ffi::sqsh_superblock_has_xattr_table(self.inner) }
    }

    /// Checks if a superblock context has compression options.
    pub fn has_compression_options(&self) -> bool {
        unsafe { ffi::sqsh_superblock_has_compression_options(self.inner) }
    }

    /// Retrieves the major version of an archive.
    pub fn version_major(&self) -> u16 {
        unsafe { ffi::sqsh_superblock_version_major(self.inner) }
    }

    /// Retrieves the minor version of an archive.
    pub fn version_minor(&self) -> u16 {
        unsafe { ffi::sqsh_superblock_version_minor(self.inner) }
    }

    /// Retrieves the block size of an archive.
    pub fn block_size(&self) -> u32 {
        unsafe { ffi::sqsh_superblock_block_size(self.inner) }
    }

    /// Retrieves the modification time of an archive, as seconds since the Unix epoch.
    pub fn modification_time(&self) -> u32 {
        unsafe { ffi::sqsh_superblock_modification_time(self.inner) }
    }

    /// Retrieves the number of bytes used in an archive.
    pub fn bytes_used(&self) -> u64 {
        unsafe { ffi::sqsh_superblock_bytes_used(self.inner) }
    }
}

impl<'archive> fmt::Debug for Superblock<'archive> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Superblock")
            .field("compression_type", &self.compression_type())
            .field("inode_count", &self.inode_count())
            .field("id_count", &self.id_count())
            .field("fragment_entry_count", &self.fragment_entry_count())
            .field("inode_table_start", &self.inode_table_start())
            .field("directory_table_start", &self.directory_table_start())
            .field("fragment_table_start", &self.fragment_table_start())
            .field("export_table_start", &self.export_table_start())
            .field("id_table_start", &self.id_table_start())
            .field("xattr_id_table_start", &self.xattr_id_table_start())
            .field("root_inode_ref", &self.root_inode_ref())
            .field("has_fragments", &self.has_fragments())
            .field("has_export_table", &self.has_export_table())
            .field("has_xattr_table", &self.has_xattr_table())
            .field("has_compression_options", &self.has_compression_options())
            .field("version_major", &self.version_major())
            .field("version_minor", &self.version_minor())
            .field("block_size", &self.block_size())
            .field("modification_time", &self.modification_time())
            .field("bytes_used", &self.bytes_used())
            .finish()
    }
}
