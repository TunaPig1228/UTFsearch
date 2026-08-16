use unicode_normalization::UnicodeNormalization;

/// NFC, then lowercase when `casefold` is set (Windows-style catalogs).
pub fn search_key(s: &str, casefold: bool) -> String {
    let nfc: String = s.nfc().collect();
    if casefold {
        nfc.to_lowercase()
    } else {
        nfc
    }
}

/// Display-friendly relative path using `/`.
pub fn rel_display(parts: &[&str]) -> String {
    parts.join("/")
}

pub fn ext_key(ext: &str) -> String {
    ext.trim().trim_start_matches('.').to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfc_and_casefold() {
        let nfd = "e\u{0301}";
        assert_eq!(search_key(nfd, false), "é");
        assert_eq!(search_key("發票", true), "發票");
        assert_eq!(search_key("Invoice", true), "invoice");
        assert_eq!(ext_key(".XLSX"), "xlsx");
    }
}
