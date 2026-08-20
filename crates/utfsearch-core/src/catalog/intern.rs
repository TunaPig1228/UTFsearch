use std::collections::HashMap;
use std::sync::OnceLock;

/// String intern table. Id 0 is the empty string.
///
/// Strings are stored as a single contiguous `blob` with an `offsets` array
/// (string `i` = `blob[offsets[i]..offsets[i + 1]]`). Loading a serialized
/// table is therefore two bulk copies — not one allocation per string — and the
/// reverse `bytes -> id` map (needed only by `lookup`) is built lazily on first
/// use. This keeps `Catalog::open` cheap on multi-million-entry catalogs, where
/// eagerly materializing millions of tiny strings and a full reverse map used
/// to dominate startup.
#[derive(Debug, Default)]
pub struct Intern {
    blob: Vec<u8>,
    /// `count + 1` boundaries into `blob`.
    offsets: Vec<u32>,
    /// Reverse index maintained incrementally while *building* (so `intern`
    /// dedups). Empty after `from_serialized`.
    build_map: HashMap<Box<[u8]>, u32>,
    /// Reverse index built lazily on the *read* path, only if `lookup` is
    /// called (e.g. an `--ext` / `--owner` filter). Name-only searches never
    /// pay for it.
    read_map: OnceLock<HashMap<Box<[u8]>, u32>>,
}

impl Intern {
    pub fn new() -> Self {
        let mut s = Self {
            blob: Vec::new(),
            offsets: vec![0],
            build_map: HashMap::new(),
            read_map: OnceLock::new(),
        };
        // Id 0 is always the empty string.
        s.intern_bytes(&[]);
        s
    }

    pub fn intern(&mut self, s: &str) -> u32 {
        self.intern_bytes(s.as_bytes())
    }

    pub fn intern_bytes(&mut self, b: &[u8]) -> u32 {
        if let Some(&id) = self.build_map.get(b) {
            return id;
        }
        let id = (self.offsets.len() - 1) as u32;
        self.blob.extend_from_slice(b);
        self.offsets.push(self.blob.len() as u32);
        self.build_map.insert(b.into(), id);
        id
    }

    pub fn get(&self, id: u32) -> &[u8] {
        let i = id as usize;
        match (self.offsets.get(i), self.offsets.get(i + 1)) {
            (Some(&a), Some(&b)) if b as usize <= self.blob.len() => {
                &self.blob[a as usize..b as usize]
            }
            _ => &[],
        }
    }

    pub fn get_str(&self, id: u32) -> String {
        String::from_utf8_lossy(self.get(id)).into_owned()
    }

    pub fn lookup(&self, s: &str) -> Option<u32> {
        self.reverse().get(s.as_bytes()).copied()
    }

    /// The reverse `bytes -> id` index. On the build path it already exists; on
    /// the read path it is materialized once, on demand.
    fn reverse(&self) -> &HashMap<Box<[u8]>, u32> {
        if !self.build_map.is_empty() {
            return &self.build_map;
        }
        self.read_map.get_or_init(|| {
            let count = self.offsets.len().saturating_sub(1);
            let mut map = HashMap::with_capacity(count);
            for id in 0..count as u32 {
                map.insert(Box::from(self.get(id)), id);
            }
            map
        })
    }

    #[allow(dead_code)]
    pub fn len(&self) -> u32 {
        self.offsets.len().saturating_sub(1) as u32
    }

    /// offsets[i]..offsets[i+1] into blob.
    pub fn serialize(&self) -> (Vec<u32>, Vec<u8>) {
        (self.offsets.clone(), self.blob.clone())
    }

    pub fn from_serialized(offsets: &[u32], blob: &[u8]) -> Self {
        if offsets.len() < 2 {
            return Self::new();
        }
        Self {
            blob: blob.to_vec(),
            offsets: offsets.to_vec(),
            build_map: HashMap::new(),
            read_map: OnceLock::new(),
        }
    }
}

