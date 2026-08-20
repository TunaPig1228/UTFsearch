use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use utfsearch_core::{
    build, parse_size, parse_time, Catalog, Query, Result, Root, RootSet, View, DEFAULT_LIMIT,
};

use crate::config::{default_config_path, FileConfig};

#[derive(Parser)]
#[command(name = "utfsearch", version, about = "Compact Unicode path catalog")]
pub struct Cli {
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
    #[arg(long, global = true)]
    pub catalog: Option<PathBuf>,
    #[arg(long, global = true, default_value = "text")]
    pub format: Format,
    #[arg(long, global = true)]
    pub quiet: bool,
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Format {
    Text,
    Json,
}

#[derive(Subcommand)]
pub enum Cmd {
    Init {
        #[arg(long)]
        path: Option<PathBuf>,
        #[arg(long)]
        catalog: Option<PathBuf>,
    },
    /// Full walk. Pass --root on the first run; later runs reuse the catalog.
    Index {
        /// Folder to index. First run only. Repeat for multiple roots.
        #[arg(long, short = 'r')]
        root: Vec<PathBuf>,
        #[arg(long)]
        exclude: Vec<String>,
        /// Also walk OS system folders (Windows, $Recycle.Bin, SYSTEM files).
        #[arg(long)]
        include_system: bool,
    },
    /// Incremental update. Reuses roots stored in the catalog.
    Refresh {
        /// Optional override. Omit to keep the previous roots.
        #[arg(long, short = 'r')]
        root: Vec<PathBuf>,
        #[arg(long)]
        exclude: Vec<String>,
        #[arg(long)]
        include_system: bool,
    },
    Search {
        fragment: Option<String>,
        #[arg(long)]
        name: Vec<String>,
        /// Substring match on the relative path.
        #[arg(long)]
        path: Option<String>,
        /// Exact directory scope: a known relative directory from the root
        /// (e.g. finance/2024). Restricts the search to that folder's subtree
        /// for a large speed-up. Use --path instead for a path fragment.
        #[arg(long)]
        dir: Option<String>,
        #[arg(long)]
        ext: Option<String>,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long)]
        after: Option<String>,
        #[arg(long)]
        before: Option<String>,
        #[arg(long)]
        min_size: Option<String>,
        #[arg(long)]
        max_size: Option<String>,
        #[arg(long)]
        root: Option<String>,
        #[arg(long, default_value_t = DEFAULT_LIMIT)]
        limit: u16,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        full: bool,
    },
    Tree {
        path: PathBuf,
        #[arg(long)]
        root: Option<String>,
        #[arg(long)]
        full: bool,
    },
    Status,
    Mcp {
        #[arg(long)]
        http: Option<String>,
        #[arg(long)]
        token: Option<String>,
    },
}

pub fn run(cli: Cli) -> ExitCode {
    match run_inner(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let _ = writeln!(io::stderr(), "{e}");
            exit_for(&e)
        }
    }
}

fn exit_for(e: &utfsearch_core::Error) -> ExitCode {
    use utfsearch_core::Error::*;
    ExitCode::from(match e {
        Query(_) => 5,
        Jail | NestedRoot(_) | MissingRoot(_) => 2,
        Corrupt(_) | Version(_) => 3,
        Io(_) => 4,
        _ => 1,
    })
}

fn run_inner(cli: Cli) -> Result<()> {
    match &cli.cmd {
        Cmd::Init { path, catalog } => return init(path.clone(), catalog.clone()),
        Cmd::Mcp { http, token } => {
            return crate::mcp::run(resolve_catalog(&cli)?, http.clone(), token.clone());
        }
        _ => {}
    }
    let format = cli.format;
    let quiet = cli.quiet;
    let config = cli.config.clone();
    let catalog = cli.catalog.clone();
    let cmd = cli.cmd;
    let cli = Cli {
        config,
        catalog,
        format,
        quiet,
        cmd: Cmd::Status,
    };
    dispatch(cli, cmd)
}

fn dispatch(cli: Cli, cmd: Cmd) -> Result<()> {
    match cmd {
        Cmd::Index {
            root,
            exclude,
            include_system,
        } => {
            let catalog = resolve_catalog(&cli)?;
            let roots = resolve_roots(&catalog, &root, &exclude, !include_system, false)?;
            remember_catalog(&catalog);
            let stats = build(&catalog, &roots, None)?;
            emit(&cli, &stats)
        }
        Cmd::Refresh {
            root,
            exclude,
            include_system,
        } => {
            let catalog = resolve_catalog(&cli)?;
            let old = Catalog::open(&catalog).ok();
            let roots = resolve_roots(&catalog, &root, &exclude, !include_system, true)?;
            remember_catalog(&catalog);
            let stats = build(&catalog, &roots, old.as_ref())?;
            emit(&cli, &stats)
        }
        Cmd::Search {
            fragment,
            name,
            path,
            dir,
            ext,
            owner,
            after,
            before,
            min_size,
            max_size,
            root,
            limit,
            cursor,
            full,
        } => {
            let catalog = resolve_catalog(&cli)?;
            remember_catalog(&catalog);
            let timing = std::env::var_os("UTFSEARCH_TIMING").is_some();
            let t_open = std::time::Instant::now();
            let cat = Catalog::open(&catalog)?;
            if timing {
                eprintln!("[timing] open: {:?}", t_open.elapsed());
            }
            let mut q = Query::new();
            // Use fragment as fallback if no --name args provided
            if name.is_empty() {
                q.name = fragment;
            } else {
                q.names = name;
            }
            q.name_or_path = None;
            q.path = path;
            q.dir = dir;
            q.ext = ext;
            q.owner = owner;
            q.mtime_min = after.as_deref().map(parse_time).transpose()?;
            q.mtime_max = before.as_deref().map(parse_time).transpose()?;
            q.size_min = min_size.as_deref().map(parse_size).transpose()?;
            q.size_max = max_size.as_deref().map(parse_size).transpose()?;
            q.root = root;
            q.limit = limit;
            q.cursor = cursor
                .as_deref()
                .map(utfsearch_core::Cursor::decode)
                .transpose()?;
            q.view = if full { View::Full } else { View::Compact };
            let t_search = std::time::Instant::now();
            let page = cat.search(q)?;
            if timing {
                eprintln!(
                    "[timing] search: {:?} ({} hits)",
                    t_search.elapsed(),
                    page.hits.len()
                );
            }
            emit(&cli, &page)
        }
        Cmd::Tree { path, root, full } => {
            let cat = Catalog::open(&resolve_catalog(&cli)?)?;
            let view = if full { View::Full } else { View::Compact };
            emit(&cli, &cat.children_of(&path, root.as_deref(), view)?)
        }
        Cmd::Status => {
            let cat = Catalog::open(&resolve_catalog(&cli)?)?;
            emit(&cli, &cat.status())
        }
        _ => Ok(()),
    }
}

fn resolve_roots(
    catalog: &Path,
    roots: &[PathBuf],
    exclude: &[String],
    skip_system: bool,
    allow_remembered: bool,
) -> Result<RootSet> {
    if !roots.is_empty() {
        return RootSet::new(
            roots
                .iter()
                .map(|p| Root {
                    id: 0,
                    name: String::new(),
                    path: p.clone(),
                    follow_links: false,
                    excludes: exclude.to_vec(),
                    skip_system,
                })
                .collect(),
        );
    }
    if allow_remembered || catalog.exists() {
        let cat = Catalog::open(catalog)?;
        return cat.root_set();
    }
    Err(utfsearch_core::Error::Msg(
        "first run needs --root <folder>".into(),
    ))
}

/// `--catalog` > remembered path next to the exe > `catalog.uts` beside the exe.
fn resolve_catalog(cli: &Cli) -> Result<PathBuf> {
    if let Some(p) = &cli.catalog {
        return Ok(p.clone());
    }
    if let Some(cfg) = &cli.config {
        if cfg.exists() {
            return Ok(FileConfig::load(cfg)?.catalog);
        }
    }
    if let Some(p) = read_last_catalog() {
        return Ok(p);
    }
    Ok(default_catalog_file())
}

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn default_catalog_file() -> PathBuf {
    exe_dir().join("catalog.uts")
}

fn last_catalog_pointer() -> PathBuf {
    exe_dir().join("utfsearch.last")
}

fn read_last_catalog() -> Option<PathBuf> {
    let text = std::fs::read_to_string(last_catalog_pointer()).ok()?;
    let p = PathBuf::from(text.trim());
    if p.as_os_str().is_empty() {
        None
    } else {
        Some(p)
    }
}

fn remember_catalog(path: &Path) {
    let _ = std::fs::write(last_catalog_pointer(), path.display().to_string());
}

fn init(path: Option<PathBuf>, catalog: Option<PathBuf>) -> Result<()> {
    let dest = path.unwrap_or_else(default_config_path);
    let cat = catalog.unwrap_or_else(|| PathBuf::from("catalog.uts"));
    let cwd = std::env::current_dir()?;
    let body = format!(
        "catalog = {}\n\n[[roots]]\nname = \"main\"\npath = {}\nexcludes = []\n",
        toml_str(&cat),
        toml_str(&cwd)
    );
    std::fs::write(&dest, body)?;
    println!("wrote {}", dest.display());
    Ok(())
}

fn toml_str(p: &Path) -> String {
    toml::Value::String(p.display().to_string()).to_string()
}

fn emit<T: serde::Serialize>(cli: &Cli, val: &T) -> Result<()> {
    match cli.format {
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(val).map_err(|e| utfsearch_core::Error::Msg(e.to_string()))?);
        }
        Format::Text => {
            println!("{}", serde_json::to_string_pretty(val).map_err(|e| utfsearch_core::Error::Msg(e.to_string()))?);
        }
    }
    Ok(())
}
