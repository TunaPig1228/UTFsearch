# Data Model: UTFsearch Catalog

Domain terms match `CONTEXT.md`. No implementation types leak into the glossary; this file is the physical and validation model.

## Root

| Field | Type | Rules |
| --- | --- | --- |
| id | u16 | Dense, assigned at config load. Stable for one catalog generation. |
| path_faithful | Os path | Absolute after canonicalize at config load. Must exist and be a directory. |
| display_name | string | Optional; default is the last component. |
| follow_links | bool | Default false. |
| excludes | glob list | Applied relative to this Root during walk. |

**Validation**:
- Zero Roots is a config error.
- A Root nested inside another Root is a config error (FR-020).
- Canonicalize failure is a config error (or skip-missing if opted in).

**State**: `Declared` → `Reachable` | `Missing` | `Denied`.

## Entry

One file or directory observed under a Root.

| Field | Type | Rules |
| --- | --- | --- |
| id | u64 | Dense id in this catalog generation. |
| root_id | u16 | Owning Root. |
| parent_id | u64? | None only for the Root directory itself. |
| name_id | intern id | Last path component, faithful. |
| extension | intern id? | Lowercased, no leading dot. |
| owner_id | intern id? | OS file owner (修改者). Missing if the walk could not read it. |
| kind | enum | `File` \| `Dir` \| `Other` (symlink as Other when not followed). |
| size | u64 | 0 for directories. |
| mtime_unix | i64 | Seconds; missing mtime stored as 0 and flagged. |
| flags | bitset | `mtime_missing`, `non_utf8`, `symlink`. |

Full faithful path is reconstructed: Root path + interned components along the parent chain. Never store 30M full strings.

## DirectoryStat (prune map)

| Field | Type | Rules |
| --- | --- | --- |
| dir_entry_id | u64 | Must be kind=Dir. |
| mtime_unix | i64 | Compared to `metadata.modified` on refresh. |
| child_count | u32 | Diagnostic. |

If disk mtime equals stored mtime **and** follow_links is false, the walker MUST clear descendants (prune).

## Catalog

| Field | Type | Rules |
| --- | --- | --- |
| magic | `[u8;4]` | `UTFS` |
| format_version | u16 | Breaking bump rejects old readers. |
| flags | bitset | `complete`, `casefold_windows` |
| built_at | i64 | Unix seconds. |
| entry_count | u64 | |
| root_table | [Root] | |
| intern | FST or blob | Component bytes → intern id |
| entries | packed table | Indexed by entry id |
| name_trigrams | postings | trigram → roaring bitmap of entry ids |
| dir_stats | table | For prune |
| checksum | blake3 or xxh3 | Header + section lengths at minimum |

**Lifecycle**:
`Empty` → `Building` (temp file) → `Complete` (atomic rename) → `Building` (refresh) → `Complete`.
Readers always see the last `Complete` map. `catalog_status` reports whether a build is in flight.

## Query

| Field | Type | Rules |
| --- | --- | --- |
| name | string? | Filename contains, normalized. ≤256. |
| path | string? | Relative path contains, normalized. ≤256. |
| name_or_path | string? | Positional convenience: name **or** path contains. |
| ext | string? | Exact extension, case-insensitive, with or without a dot. |
| owner | string? | Owner contains, normalized. ≤256. |
| mtime_min / mtime_max | i64? | Inclusive unix-second range. |
| size_min / size_max | u64? | Inclusive byte range. |
| root_id | u16? | Restrict. Must refer to a configured Root. |
| limit | u16 | Default 200, max 200. |
| cursor | opaque | Resume after (mtime, id). |
| view | enum | `Compact` \| `Full`. |

**Execution** (logical):
1. Normalize fragment the same way names were stored.
2. If fragment length ≥ 3, lookup trigrams, intersect postings, filter by glob/root.
3. If fragment length < 3, scan the interned name dictionary (FST prefix / contains on short keys) — still not a live walk.
4. Verify survivors with real normalized contains.
5. Rank with nucleo-matcher.
6. Reconstruct path, run Jail; deny drops the candidate.
7. Project through View, then fill the Page until `limit` or the 8 KiB budget.

Jail is not a caller step. A Hit that exists has already passed Jail.

## Page

| Field | Type | Rules |
| --- | --- | --- |
| hits | [Hit] | ≤ 200, and serialized size ≤ 131072 bytes |
| more | bool | True if more hits exist after this page. |
| next_cursor | opaque? | Required when `more` is true. |
| dropped_unsafe | u32 | Jail denials on this page, usually 0. |

Do not include exact total counts on Compact pages — counting is extra work and unused tokens.

## Hit

Compact (default):

| Field | Type | Rules |
| --- | --- | --- |
| rel | string | Path relative to the owning Root, `/` separators, lossy UTF-8. |
| root | string | Root display name. |
| kind | enum | `file` \| `dir` \| `other` |
| ext | string? | |
| size | u64 | |
| mtime | i64 | |
| owner | string? | |

Full: Compact plus `path` (absolute display). Hits are ordered by `mtime` descending, then id.

## Jail Decision

| Field | Type | Rules |
| --- | --- | --- |
| input | Os path | |
| allowed | bool | |
| reason | enum | `InsideRoot`, `Outside`, `CanonicalizeFailed`, `SymlinkDenied`, `NotUtf8Denied` (only if policy says so — default allow non-UTF8 if inside root) |

Invariant: `allowed` implies the canonical path equals or is a descendant of a Root path, using platform prefix rules (Windows long-path and case).

## State transitions

```text
Config load --invalid--> Error
Config load --ok--> Ready (no catalog)
Ready --index--> Building --> Complete
Complete --search--> Page
Complete --refresh--> Building --> Complete
Building --crash--> previous Complete remains
```
