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
    // Resolve every gram to its posting list. A missing gram means no entry can
    // contain the substring, so the result is empty.
    let mut posts: Vec<&RoaringBitmap> = Vec::with_capacity(grams.len());
    for g in grams {
        match index
            .binary_search_by_key(g, |(k, _)| *k)
            .ok()
            .map(|i| &index[i].1)
        {
            Some(bm) => posts.push(bm),
            None => return Some(RoaringBitmap::new()),
        }
    }
    // Selectivity ordering: intersect smallest posting lists first so the
    // running accumulator stays as small as possible, minimizing work.
    posts.sort_unstable_by_key(|bm| bm.len());
    let mut acc = posts[0].clone();
    for bm in &posts[1..] {
        if acc.is_empty() {
            break;
        }
        acc &= *bm;
    }
    Some(acc)
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
