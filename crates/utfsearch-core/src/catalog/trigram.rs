use roaring::RoaringBitmap;

/// Consecutive 3-character windows. Strings shorter than 3 chars have no grams
/// (callers must scan and verify). No padding — padding breaks `contains`.
pub fn trigrams(s: &str) -> Vec<u32> {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() < 3 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(chars.len() - 2);
    for w in chars.windows(3) {
        out.push(tri_hash(w[0], w[1], w[2]));
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn tri_hash(a: char, b: char, c: char) -> u32 {
    let mut h = 2166136261u32;
    for x in [a, b, c] {
        h ^= x as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

pub fn intersect_postings(
    index: &[(u32, RoaringBitmap)],
    grams: &[u32],
) -> Option<RoaringBitmap> {
    if grams.is_empty() {
        return None;
    }
    let mut acc: Option<RoaringBitmap> = None;
    for g in grams {
        let found = index
            .binary_search_by_key(g, |(k, _)| *k)
            .ok()
            .map(|i| &index[i].1);
        match (acc, found) {
            (None, Some(bm)) => acc = Some(bm.clone()),
            (Some(a), Some(bm)) => acc = Some(a & bm),
            (_, None) => return Some(RoaringBitmap::new()),
        }
        if acc.as_ref().is_some_and(|a| a.is_empty()) {
            return acc;
        }
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_string_shares_grams() {
        let a = trigrams("invoice");
        let b = trigrams("invoice.xlsx");
        assert!(a.iter().any(|g| b.contains(g)));
    }
}
