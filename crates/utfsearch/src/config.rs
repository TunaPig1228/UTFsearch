use std::path::{Path, PathBuf};

use serde::Deserialize;
use utfsearch_core::{Error, Result, Root, RootSet};

#[derive(Debug, Deserialize)]
pub struct FileConfig {
    pub catalog: PathBuf,
    #[serde(default)]
    pub roots: Vec<RootConfig>,
}

#[derive(Debug, Deserialize)]
pub struct RootConfig {
    #[serde(default)]
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub follow_links: bool,
    #[serde(default)]
    pub excludes: Vec<String>,
}

impl FileConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(Error::from)?;
        toml::from_str(&text).map_err(|e| Error::Msg(e.to_string()))
    }

    pub fn root_set(&self) -> Result<RootSet> {
        let roots = self
            .roots
            .iter()
            .map(|r| Root {
                id: 0,
                name: r.name.clone(),
                path: r.path.clone(),
                follow_links: r.follow_links,
                excludes: r.excludes.clone(),
                skip_system: true,
            })
            .collect();
        RootSet::new(roots)
    }
}

pub fn default_config_path() -> PathBuf {
    PathBuf::from("utfsearch.toml")
}
