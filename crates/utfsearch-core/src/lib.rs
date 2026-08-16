//! Compact, mmap'd path catalog. Callers use [`Catalog::search`].

pub mod catalog;
pub mod error;
pub mod jail;
pub mod normalize;
pub mod owner;
pub mod paths;
pub mod query;
pub mod root;
pub mod skip;
pub mod walk;
pub mod watch;

pub use catalog::{build, BuildStats, Catalog, Hit, Page, Status, StoredRoot};
pub use error::{Error, Result};
pub use query::{parse_size, parse_time, Cursor, Query, View, DEFAULT_LIMIT, MAX_LIMIT};
pub use root::{Root, RootSet};
