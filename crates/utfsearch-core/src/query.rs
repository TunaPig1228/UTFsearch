use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub const DEFAULT_LIMIT: u16 = 200;
pub const MAX_LIMIT: u16 = 5000;
pub const MAX_FILTER_CHARS: usize = 256;
pub const PAGE_BUDGET: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    #[default]
    Compact,
    Full,
}

#[derive(Debug, Clone, Default)]
pub struct Query {
    pub name: Option<String>,
    pub path: Option<String>,
    /// Convenience: matches filename **or** relative path.
    pub name_or_path: Option<String>,
    pub ext: Option<String>,
    pub owner: Option<String>,
    pub mtime_min: Option<i64>,
    pub mtime_max: Option<i64>,
    pub size_min: Option<u64>,
    pub size_max: Option<u64>,
    pub root: Option<String>,
    pub limit: u16,
    pub cursor: Option<Cursor>,
    pub view: View,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub last_mtime: i64,
    pub last_id: u32,
}

impl Cursor {
    pub fn encode(self) -> String {
        format!("{}:{}", self.last_mtime, self.last_id)
    }

    pub fn decode(s: &str) -> Result<Self> {
        let (a, b) = s.split_once(':').ok_or(Error::Cursor)?;
        Ok(Self {
            last_mtime: a.parse().map_err(|_| Error::Cursor)?,
            last_id: b.parse().map_err(|_| Error::Cursor)?,
        })
    }
}

impl Query {
    pub fn new() -> Self {
        Self {
            limit: DEFAULT_LIMIT,
            ..Self::default()
        }
    }

    pub fn sanitize(mut self) -> Result<Self> {
        if self.limit == 0 {
            self.limit = DEFAULT_LIMIT;
        }
        if self.limit > MAX_LIMIT {
            self.limit = MAX_LIMIT;
        }
        for text in [
            self.name.as_deref(),
            self.path.as_deref(),
            self.name_or_path.as_deref(),
            self.owner.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if text.chars().count() > MAX_FILTER_CHARS {
                return Err(Error::Query("filter longer than 256 characters"));
            }
        }
        if let Some(ext) = self.ext.as_mut() {
            *ext = crate::normalize::ext_key(ext);
        }
        Ok(self)
    }
}

pub fn parse_size(s: &str) -> Result<u64> {
    let raw = s.trim().to_ascii_lowercase();
    let (num, mul) = if let Some(x) = raw.strip_suffix('g') {
        (x, 1u64 << 30)
    } else if let Some(x) = raw.strip_suffix('m') {
        (x, 1u64 << 20)
    } else if let Some(x) = raw.strip_suffix('k') {
        (x, 1u64 << 10)
    } else {
        (raw.as_str(), 1)
    };
    let n: u64 = num
        .trim()
        .parse()
        .map_err(|_| Error::Query("invalid size"))?;
    Ok(n.saturating_mul(mul))
}

/// Unix seconds, or `YYYY-MM-DD` (UTC midnight).
pub fn parse_time(s: &str) -> Result<i64> {
    let s = s.trim();
    if let Ok(n) = s.parse::<i64>() {
        return Ok(n);
    }
    let mut it = s.split('-');
    let y: i32 = it
        .next()
        .and_then(|x| x.parse().ok())
        .ok_or(Error::Query("invalid date"))?;
    let m: u32 = it
        .next()
        .and_then(|x| x.parse().ok())
        .ok_or(Error::Query("invalid date"))?;
    let d: u32 = it
        .next()
        .and_then(|x| x.parse().ok())
        .ok_or(Error::Query("invalid date"))?;
    if it.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return Err(Error::Query("invalid date"));
    }
    Ok(civil_unix(y, m, d))
}

fn civil_unix(y: i32, m: u32, d: u32) -> i64 {
    // Howard Hinnant civil-from-days, days since 1970-01-01.
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era as i64 * 146097 + doe as i64 - 719468;
    days * 86400
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_and_date() {
        assert_eq!(parse_size("10k").unwrap(), 10240);
        assert_eq!(parse_size("2m").unwrap(), 2 << 20);
        assert_eq!(parse_time("1970-01-02").unwrap(), 86400);
        assert!(Query::new().sanitize().is_ok());
        let mut q = Query::new();
        q.limit = 6000;
        assert_eq!(q.sanitize().unwrap().limit, MAX_LIMIT);
    }
}
