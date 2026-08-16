# Implementation Plan: UTFsearch Catalog

**Branch**: `001-utfsearch-catalog` | **Date**: 2026-08-16 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-utfsearch-catalog/spec.md`

## Summary

Ship a single local program that builds a durable, memory-mapped **Catalog** of file metadata under operator-declared **Roots**, answers Unicode-aware filename/path searches in tens of milliseconds at 30M scale, refreshes by pruning unchanged directories, and exposes the same **Query** interface through a CLI adapter and an official MCP adapter. Default build has no C/C++ engine, no listening port, and no content search.

Technical approach: interned path components + filename trigram postings + mmap, not DuckDB/DataFusion/Tantivy. Walker uses ripgrep's `ignore` crate plus directory-mtime prune. MCP uses official `rmcp` (2026-07-28) with Compact structured pages (relative path + span, 20 hits / 8 KiB).

### Catalog interface (the deep module)

Callers and tests use only this. Jail, View, and the 8 KiB budget live behind it.

```text
Catalog::open(path) -> Catalog
Catalog::search(Query) -> Page      # Query includes View
Catalog::children(ChildQuery) -> Page
Catalog::status() -> Status
CatalogWriter::build | refresh      # atomic publish
```

CLI adapter: View=Full, limit 50. MCP adapter: View=Compact, limit 20, `content` is a one-line count. Neither adapter implements paging or Jail.

## Technical Context

**Language/Version**: Rust 1.88+ (edition 2024 if the toolchain in CI supports it; otherwise edition 2021). rustc via `rust-toolchain.toml`.

**Primary Dependencies** (default):
- `ignore` — parallel walk + exclude patterns
- `memmap2` — catalog mapping
- `fst` — compact component / prefix dictionaries
- `roaring` — compressed trigram postings
- `nucleo-matcher` — ranking
- `unicode-normalization` — NFC
- `compact_str` — short name storage
- `clap` — CLI
- `serde` / `serde_json` / `schemars` — config and MCP schemas
- `rmcp` (features: `server`, `macros`) — MCP adapter
- `tokio` (features: `rt-multi-thread`, `io-std`, `macros`, `sync`) — MCP runtime only

**Optional features**:
- `watch` → `notify`
- `usn` (Windows) → `usn-journal-rs`
- `http` → `rmcp` streamable-http + shared-secret gate

**Storage**: Versioned local catalog file (`*.uts`), atomic replace. No server database.

**Testing**: `cargo test` (unit + interface), contract tests under `tests/contract`, filesystem integration under `tests/integration` using temp trees. `cargo llvm-cov` optional in CI.

**Target Platform**: Windows 10+, Linux (glibc and musl), macOS. Roots may be local or OS-mounted SMB/NFS.

**Project Type**: library crate + one binary crate (CLI and MCP subcommands).

**Performance Goals**: SC-001 (p95 < 50 ms @ 30M), SC-002 (unchanged refresh < 2 min local, no descendant visits), SC-008 (RSS < 400 MB query).

**Constraints**: Constitution I–IX. Default binary without C++ FFI. Owner-only catalog permissions. Query cap 256 chars. Page default 50, hard max 200. No regex. No telemetry.

**Scale/Scope**: 30 million entries, deep trees, multi-script names, 1–N Roots per catalog.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Article | Status | Evidence |
| --- | --- | --- |
| I. Library-First | Pass | `utfsearch-core` owns Catalog, Query, Jail, Walker, Watcher interfaces. |
| II. Dual Surface | Pass | `utfsearch search` and MCP `search_files` call `Catalog::search`. |
| III. Test-First | Pass | tasks.md writes contract/integration tests before adapters grow behavior. |
| IV. Capability Security | Pass | Jail module; metadata-only; caps; stdio default; HTTP requires secret. |
| V. Compactness | Pass | No DuckDB/DataFusion/Tantivy/Arrow. Optional features off by default. |
| VI. Incremental Truth | Pass | mtime prune is baseline; search never live-walks. |
| VII. Simplicity | Pass | Two crates. Watcher seam has ≥2 adapters. No eBPF. |
| VIII. Unicode Fidelity | Pass | Faithful + normalized forms in data-model.md. |
| IX. Integration-First | Pass | Temp filesystem fixtures; CLI and MCP contracts. |

Post-design re-check: still Pass. Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/001-utfsearch-catalog/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli.md
│   ├── mcp.md
│   └── catalog-format.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/utfsearch-core/
├── Cargo.toml
└── src/
    ├── lib.rs                 # public interface: Catalog, Query, Page, Jail, RootSet
    ├── root.rs
    ├── jail.rs
    ├── entry.rs
    ├── query.rs
    ├── normalize.rs
    ├── catalog/
    │   ├── mod.rs             # Catalog::open, search, children, status
    │   ├── format.rs          # on-disk layout
    │   ├── intern.rs          # component dictionary (fst)
    │   ├── trigram.rs         # postings
    │   └── writer.rs          # atomic rebuild / incremental merge
    ├── walk/
    │   ├── mod.rs             # Walker interface
    │   └── ignore_walk.rs     # ignore::WalkParallel adapter
    └── watch/
        ├── mod.rs             # Watcher interface (optional)
        ├── prune.rs           # mtime map used by walker
        ├── notify_watch.rs    # feature = watch
        └── usn_watch.rs       # feature = usn

crates/utfsearch/
├── Cargo.toml
└── src/
    ├── main.rs                # clap: index | refresh | search | tree | mcp | status
    ├── config.rs
    ├── cli.rs
    └── mcp.rs                 # rmcp adapter

tests/
├── contract/
│   ├── cli_search.rs
│   └── mcp_search.rs
├── integration/
│   ├── index_and_search.rs
│   ├── incremental_prune.rs
│   ├── jail_escape.rs
│   └── unicode_names.rs
└── fixtures/
    └── trees/

CONTEXT.md
docs/adr/
.specify/
```

**Structure Decision**: Two crates. Core is the deep module. The binary is two adapters (CLI, MCP) over that interface. Tests live at the workspace root so they compile against the published interface only.

## Complexity Tracking

> No constitution violations.
