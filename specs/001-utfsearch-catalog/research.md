# Research: UTFsearch Catalog

Date: 2026-08-16. Sources: crate docs, vendor blogs, GitHub, and public benchmarks cited below.

## 1. Do not use a columnar SQL engine for path search

**Decision**: The default catalog is a purpose-built, memory-mapped path index (interned components + filename trigrams + optional FST). DuckDB, DataFusion, and Tantivy are rejected for the default binary.

**Rationale**:
- 30 million *paths* are a dictionary problem, not an analytics problem. `contains(file_name, ?)` on a VARCHAR column is a scan unless a trigram/GIN structure exists — which neither DuckDB nor DataFusion give you as a first-class, tiny, mmap-friendly path type.
- DuckDB is C++ behind an FFI. That violates Constitution V (no C/C++ engine) and inflates the binary by tens of megabytes.
- DataFusion + Arrow + Parquet is pure Rust and won ClickBench on Parquet (Nov 2024), but the crate graph is huge and the in-memory format is columnar batches, not a prefix-compressed path dictionary. Binary and RSS miss SC-007 / SC-008.
- Tantivy is the right tool for *content* full text. For filenames it stores inverted postings plus a store; default heap behavior is heavier than a specialized trigram file. Keep it off the default path so a later content-search spec can add it behind a feature.

**Alternatives considered**:

| Engine | Pure Rust | Typical default binary tax | 30M `contains` on names | Fits compactness? |
| --- | --- | --- | --- | --- |
| DuckDB-rs | No (C++ libduckdb) | ~20–80 MB | Fast if you add your own tricks; still a SQL surface and FFI | No |
| DataFusion | Yes | ~30 MB+ with Arrow/Parquet | Fast scans, not a path dictionary | No |
| Tantivy | Yes | ~8–15 MB | Good inverted index; general search engine | Not for v1 default |
| Custom mmap catalog | Yes | ~1–3 MB + rmcp/tokio | Designed for this workload | Yes |

Memory sketch for 30M entries, average path 120 bytes, average 6 unique components reused heavily:

| Layout | On disk (order of) | Query RSS if mmap'd |
| --- | --- | --- |
| Naive VARCHAR + DuckDB | 4–8 GB | process buffer pool, often GBs |
| Interned components + packed entries | 400–900 MB | working set of postings + touched entries |
| + filename trigram postings (roaring/varint) | +150–400 MB | intersect lists only |
| FST of normalized names (optional) | +50–200 MB | mmap, prefix/range cheap |

BurntSushi's `fst` crate is the proven compact automaton for sorted byte keys (the 1.6B-key write-up). Use it for the component dictionary and for prefix lookup. Do not store every full path as an independent FST key if interned components are smaller.

## 2. Trigrams beat SQL `contains` and beat naive linear scans

**Decision**: Index Unicode-normalized *filenames* (and optionally the last N path components) as character trigrams with compressed posting lists. Verify survivors with the real normalized string. Rank with `nucleo-matcher` (fzf-compatible, Helix).

**Rationale**:
- This is the Google Code Search / Zoekt family of designs. Sourcegraph still uses Zoekt-style trigrams for indexed code search. Hexops (2021) notes Zoekt's *in-memory* posting lists are RAM-heavy; the fix is on-disk compressed postings plus mmap, which is what Postgres `pg_trgm` GIN does well. We take the algorithm, not the process.
- `nucleo` / `nucleo-matcher` is currently the fastest widely used Rust fuzzy ranker and matches fzf scoring. Use it only on the (already small) candidate page, never on 30M raw strings.
- Default query is a literal fragment (FR-003, FR-021). Trigrams implement `contains` without a regex engine.

**Alternatives considered**: Aho-Corasick / `memchr` over mmap'd name blobs (simple, but linear in catalog size — fails SC-001). Suffix arrays (excellent, huge build RAM). ngram-in-ripgrep (never shipped; issue #1497). `fastripgrep` (2026) shows community demand for sparse n-gram indexes; we own the format so we can jail and incrementally update.

## 3. Walking: `ignore` plus prune, not raw `jwalk`

**Decision**: The Walker adapter uses `ignore::WalkParallel` (ripgrep's walker) for excludes and parallelism, with an explicit prune callback driven by the catalog's directory mtime map. `jwalk` remains an allowed implementation if measurements win, behind the same interface.

**Rationale**:
- `ignore` gives gitignore-class excludes, hidden-file policy, and `WalkParallel` — required by FR-014 and open-source familiarity.
- `jwalk` streams sorted results earlier; `ignore` is better at exclude semantics. The original note picked `jwalk` only. We keep the *prune algorithm* and swap the walker if needed.
- There is no mature crate that issues SMB-specific metadata prefetch. NAS performance is "use the OS mount, issue fewer stats." Directory mtime prune is the portable win. `notify`'s own docs warn that NFS often emits no events — so watchers are optional, prune is mandatory.

**CDC / journal adapters** (optional cargo features, not default):

| Adapter | Platform | Crate / API | When it helps | Why not default |
| --- | --- | --- | --- | --- |
| Directory mtime prune | All | catalog map + walker | Always | — (baseline) |
| `notify` / `PollWatcher` | All | `notify` | Local disks; poll on SMB/NFS | Extra thread, noisy on shares |
| NTFS USN Journal | Windows | `usn-journal-rs` (2026) | Local NTFS incremental without walk | Privileges, local only, not SMB |
| eBPF (`aya`) | Linux | `aya` | Host-local vnode events | Root/capabilities, not NAS, huge ops surface |

**Decision**: Watcher is a real seam because three adapters already exist conceptually (prune-only, notify, USN). eBPF is *not* an adapter in v1 — it fails generality and security review.

## 4. Persistence: mmap a custom format, not Parquet

**Decision**: Persist a versioned catalog file (magic + header + sections). Map it with `memmap2`. Use `rkyv` or a hand-rolled packed layout for the entry table if measurements favor zero-copy. Prefer explicit packed structs over a general serializer for the hot path.

**Rationale**: Parquet/Arrow pull the entire DataFusion/arrow stack. `rkyv` is pure Rust zero-copy and is designed for large mmap'd data. A hand-rolled format is even smaller and easier to jail (no parser gadget surface from a general codec). Start hand-rolled; introduce `rkyv` only if the entry table becomes error-prone.

Refresh atomicity (FR-017): write `catalog.uts.tmp`, `fsync`, rename over `catalog.uts`. Readers hold the previous map until the new file is opened.

## 5. MCP: official `rmcp`, stdio, pagination, tasks

**Decision**: Use the official `rmcp` SDK (MCP spec **2026-07-28**, compatible with 2025-11-25). Tools: `search_files`, `list_children`, `catalog_status`. Transport default: stdio. Streamable HTTP is a feature flag and requires a shared secret (FR-018). Long `reindex` uses MCP Tasks (SEP-2663) so agents poll instead of blocking a 30M walk.

**Rationale**:
- The original snippet used a fictional `rmcp 0.1` `ToolHandler` API. The real crate is `modelcontextprotocol/rust-sdk` with `#[tool]`, `#[tool_router]`, pagination, subscriptions, and tasks.
- MCP Roots are deprecated (SEP-2577). **Do not** trust client-supplied roots as authorization. Operator-configured Roots in *our* config are the capability.
- Pagination is in the spec; do not stream 30M paths into a model context. Hard cap 200.
- `outputSchema` may be any JSON Schema in 2026-07-28 — return a structured page object, not a concatenated string.

## 6. Unicode

**Decision**: Store `PathBuf`/`OsString` faithful bytes plus a `norm` key: NFC then compatible case-fold for search. Windows Roots match case-insensitively; Unix Roots match case-sensitively after NFC. Never silently drop non-UTF-8 Unix paths — represent them as lossy display + raw bytes.

**Rationale**: Project name and real NAS inventories are multi-script. macOS NFD vs NFC is a classic miss. `unicode-normalization` + `nucleo-matcher`'s Unicode path is sufficient; do not pull ICU.

## 7. Security research

**Decision**: Capability Roots + canonicalize-and-jail + metadata-only + owner-only catalog files + literal queries + no default network.

Known failure modes to test:
- `..` and extra slashes after canonicalize.
- Symlink / junction pointing outside the Root (default: do not follow).
- Windows `\\?\`, UNC, alternate data streams in names.
- TOCTOU between jail check and any later filesystem touch (v1 should not touch file contents at all).
- Agent as confused deputy: tools cannot add Roots.

## Resolved NEEDS CLARIFICATION

None remain. Spec clarifications (2026-08-16) plus this research close the technical context.
