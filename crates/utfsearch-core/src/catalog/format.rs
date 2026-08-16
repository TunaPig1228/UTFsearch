use crate::error::{Error, Result};

pub const MAGIC: &[u8; 4] = b"UTFS";
pub const VERSION: u16 = 2;
pub const HEADER_LEN: usize = 256;
pub const ENTRY_LEN: usize = 32;
pub const NONE: u32 = u32::MAX;

pub const FLAG_DIR: u8 = 1;
pub const FLAG_OTHER: u8 = 2;
pub const FLAG_MTIME_MISSING: u8 = 4;
pub const FLAG_SYMLINK: u8 = 8;
pub const HDR_CASEFOLD: u16 = 1;

#[derive(Debug, Clone, Copy)]
pub struct PackedEntry {
    pub parent: u32,
    pub name_id: u32,
    pub ext_id: u16,
    pub owner_id: u16,
    pub root_id: u8,
    pub flags: u8,
    pub size: u64,
    pub mtime: i64,
}

impl PackedEntry {
    pub fn encode(self) -> [u8; ENTRY_LEN] {
        let mut b = [0u8; ENTRY_LEN];
        b[0..4].copy_from_slice(&self.parent.to_le_bytes());
        b[4..8].copy_from_slice(&self.name_id.to_le_bytes());
        b[8..10].copy_from_slice(&self.ext_id.to_le_bytes());
        b[10..12].copy_from_slice(&self.owner_id.to_le_bytes());
        b[12] = self.root_id;
        b[13] = self.flags;
        b[16..24].copy_from_slice(&self.size.to_le_bytes());
        b[24..32].copy_from_slice(&self.mtime.to_le_bytes());
        b
    }

    pub fn decode(b: &[u8]) -> Result<Self> {
        if b.len() < ENTRY_LEN {
            return Err(Error::Corrupt("short entry"));
        }
        Ok(Self {
            parent: u32::from_le_bytes(b[0..4].try_into().unwrap()),
            name_id: u32::from_le_bytes(b[4..8].try_into().unwrap()),
            ext_id: u16::from_le_bytes(b[8..10].try_into().unwrap()),
            owner_id: u16::from_le_bytes(b[10..12].try_into().unwrap()),
            root_id: b[12],
            flags: b[13],
            size: u64::from_le_bytes(b[16..24].try_into().unwrap()),
            mtime: i64::from_le_bytes(b[24..32].try_into().unwrap()),
        })
    }

    pub fn is_dir(self) -> bool {
        self.flags & FLAG_DIR != 0
    }

    pub fn kind_str(self) -> &'static str {
        if self.flags & FLAG_DIR != 0 {
            "dir"
        } else if self.flags & FLAG_OTHER != 0 {
            "other"
        } else {
            "file"
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Header {
    pub version: u16,
    pub flags: u16,
    pub built_at: i64,
    pub entry_count: u64,
    pub intern_off: u64,
    pub intern_len: u64,
    pub entries_off: u64,
    pub entries_len: u64,
    pub tri_off: u64,
    pub tri_len: u64,
    pub ext_off: u64,
    pub ext_len: u64,
    pub owner_off: u64,
    pub owner_len: u64,
    pub dirstat_off: u64,
    pub dirstat_len: u64,
    pub mtime_off: u64,
    pub mtime_len: u64,
    pub roots_off: u64,
    pub roots_len: u64,
    pub path_tri_off: u64,
    pub path_tri_len: u64,
}

impl Header {
    pub fn encode(self) -> [u8; HEADER_LEN] {
        let mut b = [0u8; HEADER_LEN];
        b[0..4].copy_from_slice(MAGIC);
        b[4..6].copy_from_slice(&self.version.to_le_bytes());
        b[6..8].copy_from_slice(&self.flags.to_le_bytes());
        put_i64(&mut b, 8, self.built_at);
        put_u64(&mut b, 16, self.entry_count);
        put_u64(&mut b, 24, self.intern_off);
        put_u64(&mut b, 32, self.intern_len);
        put_u64(&mut b, 40, self.entries_off);
        put_u64(&mut b, 48, self.entries_len);
        put_u64(&mut b, 56, self.tri_off);
        put_u64(&mut b, 64, self.tri_len);
        put_u64(&mut b, 72, self.ext_off);
        put_u64(&mut b, 80, self.ext_len);
        put_u64(&mut b, 88, self.owner_off);
        put_u64(&mut b, 96, self.owner_len);
        put_u64(&mut b, 104, self.dirstat_off);
        put_u64(&mut b, 112, self.dirstat_len);
        put_u64(&mut b, 120, self.mtime_off);
        put_u64(&mut b, 128, self.mtime_len);
        put_u64(&mut b, 136, self.roots_off);
        put_u64(&mut b, 144, self.roots_len);
        put_u64(&mut b, 152, self.path_tri_off);
        put_u64(&mut b, 160, self.path_tri_len);
        b
    }

    pub fn decode(b: &[u8]) -> Result<Self> {
        if b.len() < HEADER_LEN {
            return Err(Error::Corrupt("short header"));
        }
        if &b[0..4] != MAGIC {
            return Err(Error::Corrupt("bad magic"));
        }
        let version = u16::from_le_bytes(b[4..6].try_into().unwrap());
        if version != VERSION {
            return Err(Error::Version(version));
        }
        Ok(Self {
            version,
            flags: u16::from_le_bytes(b[6..8].try_into().unwrap()),
            built_at: get_i64(b, 8),
            entry_count: get_u64(b, 16),
            intern_off: get_u64(b, 24),
            intern_len: get_u64(b, 32),
            entries_off: get_u64(b, 40),
            entries_len: get_u64(b, 48),
            tri_off: get_u64(b, 56),
            tri_len: get_u64(b, 64),
            ext_off: get_u64(b, 72),
            ext_len: get_u64(b, 80),
            owner_off: get_u64(b, 88),
            owner_len: get_u64(b, 96),
            dirstat_off: get_u64(b, 104),
            dirstat_len: get_u64(b, 112),
            mtime_off: get_u64(b, 120),
            mtime_len: get_u64(b, 128),
            roots_off: get_u64(b, 136),
            roots_len: get_u64(b, 144),
            path_tri_off: get_u64(b, 152),
            path_tri_len: get_u64(b, 160),
        })
    }

    pub fn casefold(self) -> bool {
        self.flags & HDR_CASEFOLD != 0
    }
}

pub fn put_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

pub fn put_u16(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_le_bytes());
}

pub fn read_u32(b: &[u8], off: &mut usize) -> Result<u32> {
    let s = *off;
    *off += 4;
    b.get(s..s + 4)
        .ok_or(Error::Corrupt("u32"))
        .map(|x| u32::from_le_bytes(x.try_into().unwrap()))
}

pub fn read_u16(b: &[u8], off: &mut usize) -> Result<u16> {
    let s = *off;
    *off += 2;
    b.get(s..s + 2)
        .ok_or(Error::Corrupt("u16"))
        .map(|x| u16::from_le_bytes(x.try_into().unwrap()))
}

fn put_u64(b: &mut [u8], off: usize, v: u64) {
    b[off..off + 8].copy_from_slice(&v.to_le_bytes());
}
fn put_i64(b: &mut [u8], off: usize, v: i64) {
    b[off..off + 8].copy_from_slice(&v.to_le_bytes());
}
fn get_u64(b: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}
fn get_i64(b: &[u8], off: usize) -> i64 {
    i64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}
