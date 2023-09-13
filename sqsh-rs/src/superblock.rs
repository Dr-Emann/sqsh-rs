use crate::InodeRef;
use sqsh_sys as ffi;
use std::fmt;

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
