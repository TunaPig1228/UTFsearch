# UTFsearch

Local-first catalog of file **names and paths** for humans and AI agents. Built to stay small and fast at tens of millions of deeply nested, multi-script files — including NAS folders the operating system already mounted.

This repository is in **specification phase**. Implementation follows `specs/001-utfsearch-catalog/tasks.md`.

## Why not a SQL engine?

A path catalog is a dictionary plus substring lookup, not an analytics warehouse. The default design is a memory-mapped catalog (interned components + filename trigrams). No C++ database, no background server, no file-content read.

## Build

```text
cargo build -p utfsearch --release
```

```text
utfsearch index --root D:\資料
utfsearch refresh
utfsearch search 發票
```

`--root` 只在第一次需要。Root 會寫進 catalog，之後 `refresh` / `search` / `index` 都不加參數也會沿用。

`catalog.uts` 是索引本體，預設放在 **exe 同一個資料夾**。不必每次下 `--catalog`。只有你想把索引放到別的路徑時才指定一次，程式會記住。

Default search returns **200** hits (override with `--limit`, max **5000**), newest modification time first. `root` is the absolute Root path; `path` is the absolute file path.

Filters: `--name`, `--path`, `--ext`, `--owner` (OS file owner / 修改者), `--after` / `--before`, `--min-size` / `--max-size`.

## What it does

- Indexes operator-declared **Roots** only
- Searches a compact mmap catalog (interned paths + trigrams), not the live tree
- Refreshes by skipping unchanged directories (millisecond mtimes)
- CLI and MCP share `Catalog::search`
- Jails every path; one structured page; no prose dump of the same hits

## What it does not do (v1)

- Read or index file contents
- Speak SMB/NFS itself (use an OS mount)
- Follow symlinks/junctions unless you say so
- Listen on the network without a shared secret
- Accept unbounded regular expressions

## Spec Kit

| Artifact | Path |
| --- | --- |
| Constitution | `.specify/memory/constitution.md` |
| Glossary | `CONTEXT.md` |
| Feature spec | `specs/001-utfsearch-catalog/spec.md` |
| Plan | `specs/001-utfsearch-catalog/plan.md` |
| Research | `specs/001-utfsearch-catalog/research.md` |
| Tasks | `specs/001-utfsearch-catalog/tasks.md` |
| ADRs | `docs/adr/` |

Next implementation command: `/speckit-implement` (or work `tasks.md` from T001).

## License

Not yet chosen. Recommendation for a Rust open-source repo: **MIT OR Apache-2.0**.
