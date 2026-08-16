use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct Root {
    pub id: u16,
    pub name: String,
    pub path: PathBuf,
    pub follow_links: bool,
    pub excludes: Vec<String>,
    /// Skip OS system dirs/files (Recycle Bin, Windows, SYSTEM attribute). Default true.
    pub skip_system: bool,
}

impl Root {
    pub fn globset(&self) -> Result<GlobSet> {
        let mut b = GlobSetBuilder::new();
        for pat in &self.excludes {
            let g = Glob::new(pat).map_err(|e| Error::Msg(e.to_string()))?;
            b.add(g);
        }
        b.build().map_err(|e| Error::Msg(e.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct RootSet {
    pub roots: Vec<Root>,
}

impl RootSet {
    pub fn new(mut roots: Vec<Root>) -> Result<Self> {
        if roots.is_empty() {
            return Err(Error::Msg("at least one Root is required".into()));
        }
        for (i, r) in roots.iter_mut().enumerate() {
            r.id = i as u16;
            if r.name.is_empty() {
                r.name = r
                    .path
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| format!("root{i}"));
            }
            let meta = std::fs::metadata(&r.path).map_err(|_| Error::MissingRoot(r.path.clone()))?;
            if !meta.is_dir() {
                return Err(Error::MissingRoot(r.path.clone()));
            }
        }
        for i in 0..roots.len() {
            for j in 0..roots.len() {
                if i == j {
                    continue;
                }
                if roots[i].path.starts_with(&roots[j].path) {
                    return Err(Error::NestedRoot(roots[i].path.display().to_string()));
                }
            }
        }
        Ok(Self { roots })
    }

    pub fn by_name(&self, name: &str) -> Option<&Root> {
        self.roots.iter().find(|r| r.name == name || r.id.to_string() == name)
    }

    pub fn paths(&self) -> Vec<&Path> {
        self.roots.iter().map(|r| r.path.as_path()).collect()
    }
}
