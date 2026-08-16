use std::collections::HashMap;

/// String intern table. Id 0 is the empty string.
#[derive(Debug, Default)]
pub struct Intern {
    to_id: HashMap<Vec<u8>, u32>,
    bytes: Vec<Vec<u8>>,
}

impl Intern {
    pub fn new() -> Self {
        let mut s = Self {
            to_id: HashMap::new(),
            bytes: Vec::new(),
        };
        s.bytes.push(Vec::new());
        s.to_id.insert(Vec::new(), 0);
        s
    }

    pub fn intern(&mut self, s: &str) -> u32 {
        self.intern_bytes(s.as_bytes())
    }

    pub fn intern_bytes(&mut self, b: &[u8]) -> u32 {
        if let Some(&id) = self.to_id.get(b) {
            return id;
        }
        let id = self.bytes.len() as u32;
        let owned = b.to_vec();
        self.to_id.insert(owned.clone(), id);
        self.bytes.push(owned);
        id
    }

    pub fn get(&self, id: u32) -> &[u8] {
        self.bytes.get(id as usize).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn get_str(&self, id: u32) -> String {
        String::from_utf8_lossy(self.get(id)).into_owned()
    }

    pub fn lookup(&self, s: &str) -> Option<u32> {
        self.to_id.get(s.as_bytes()).copied()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> u32 {
        self.bytes.len() as u32
    }

    /// offsets[i]..offsets[i+1] into blob.
    pub fn serialize(&self) -> (Vec<u32>, Vec<u8>) {
        let mut offsets = Vec::with_capacity(self.bytes.len() + 1);
        let mut blob = Vec::new();
        offsets.push(0);
        for s in &self.bytes {
            blob.extend_from_slice(s);
            offsets.push(blob.len() as u32);
        }
        (offsets, blob)
    }

    pub fn from_serialized(offsets: &[u32], blob: &[u8]) -> Self {
        let mut s = Self {
            to_id: HashMap::new(),
            bytes: Vec::new(),
        };
        if offsets.len() < 2 {
            return Self::new();
        }
        for w in offsets.windows(2) {
            let a = w[0] as usize;
            let b = w[1] as usize;
            let slice = blob.get(a..b).unwrap_or(&[]).to_vec();
            let id = s.bytes.len() as u32;
            s.to_id.insert(slice.clone(), id);
            s.bytes.push(slice);
        }
        s
    }
}
