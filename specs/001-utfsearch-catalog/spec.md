# Feature Specification: UTFsearch Catalog

**Feature Branch**: `001-utfsearch-catalog`

**Created**: 2026-08-16

**Status**: Draft

**Input**: User description: "Projectize a 30-million-file, deeply nested NAS file search system as an open-source, security-first, general-purpose catalog. Extremely compact and fast. Humans and AI agents must search filenames and paths without walking the live tree. Future public repo."

## Clarifications

### Session 2026-08-16

- Q: What is the first product surface — content search or path/metadata catalog? → A: Metadata and path catalog only. File contents are out of scope for this feature.
- Q: How do remote NAS volumes appear to the product? → A: Through the operating system's already-mounted filesystem. The product does not speak SMB or NFS itself.
- Q: What may an AI agent see? → A: Only paths under operator-declared Roots, never file bytes, always page-capped.
- Q: Default matching style? → A: Literal, Unicode-normalized fragment match on name and path. Optional glob. No unbounded regular expressions.
- Q: Default install shape? → A: One local program, no background server required, no telemetry.
- Q: Deepen Catalog or keep SQL-shaped search? → A: One Catalog interface (`search` / `children` / `status`). Jail runs before a Hit exists. CLI and MCP are adapters only.
- Q: How should agent answers spend model tokens? → A: One structured Page, no duplicated prose dump. Default page is 200 hits (also the hard ceiling). Compact fields still omit a second absolute-path copy unless Full is requested.
- Q: Default result count? → A: 200 matching files, which is also the maximum.
- Q: What can a user filter and how are hits ordered? → A: Filter by extension, filename, path, owner (OS file owner / 修改者), mtime range, and size range. Default order is modification time newest-first.
- Q: Are walk and storage engines mandated? → A: No. Choose whatever maximizes storage density and query time. The Catalog interface stays stable.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Index a root and find a file by name (Priority: P1)

An operator points the product at one allowed folder (a local disk or an OS-mounted network share) and waits until the catalog is ready. They then type a fragment of a filename — possibly in Chinese, Japanese, accented Latin, or mixed scripts — and receive a short, ranked list of matching paths in well under a second on a warm catalog of tens of millions of entries.

**Why this priority**: Without a trustworthy first search, nothing else matters. This is the MVP.

**Independent Test**: Create a temporary tree with mixed-script names, build a catalog over it, search a unique fragment, and confirm the expected path is returned and nothing outside the tree is returned.

**Acceptance Scenarios**:

1. **Given** an empty catalog and an allowed Root with known files, **When** the operator builds the catalog and searches a unique filename fragment, **Then** that file's path appears in the first page of results.
2. **Given** a catalog of a Root that contains CJK and combining-mark names, **When** the operator searches the precomposed equivalent of a name, **Then** the file is found.
3. **Given** a ready catalog, **When** the operator searches a fragment that matches nothing, **Then** they receive an empty result, not an error.
4. **Given** a ready catalog, **When** a search would match thousands of files, **Then** only the first page is returned and the operator is told more exist.

---

### User Story 2 - Refresh the catalog without walking unchanged trees (Priority: P2)

After the first build, files appear, disappear, and change under the Root. The operator (or a scheduled refresh) updates the catalog. Unchanged directory trees are skipped. Changed trees are merged. Search answers come from the updated catalog, not from a live walk.

**Why this priority**: A 30-million-file NAS cannot be fully walked on every query or every hour. Incremental refresh is what makes the product usable on real shares.

**Independent Test**: Build a catalog, add one file in a previously empty folder, change nothing else, refresh, and confirm the new file is searchable and the refresh did not re-visit pruned unchanged directories (observable via a dry-run or visit count).

**Acceptance Scenarios**:

1. **Given** a catalog whose recorded directory times still match the tree, **When** a refresh runs, **Then** it completes without enumerating files under those unchanged directories.
2. **Given** a new file in a modified directory, **When** a refresh completes, **Then** a search for that file succeeds.
3. **Given** a deleted file, **When** a refresh completes, **Then** a search no longer returns it.
4. **Given** a refresh interrupted halfway, **When** the operator searches, **Then** they either see the previous complete catalog or a clearly marked incomplete state — never a silently half-applied mix.

---

### User Story 3 - An AI agent searches through a bounded, sandboxed tool (Priority: P3)

An AI coding or operations agent, acting for the same operator, asks "where is the invoice workbook?" through the product's agent interface. It receives a short list of paths inside the allowed Roots. It cannot widen the Roots, cannot read file bytes through this feature, and cannot dump the entire catalog into its context.

**Why this priority**: Agent use is a primary reason to ship a protocol surface, but it is unsafe to expose until human search and incremental refresh exist.

**Independent Test**: Configure one Root. Invoke the agent search tool with a keyword. Confirm matches are inside the Root, the payload is page-capped, and a request that tries to name a path outside the Root is refused.

**Acceptance Scenarios**:

1. **Given** a ready catalog and an agent session, **When** the agent searches a keyword, **Then** it receives at most the configured page size of matching paths plus a continuation token if more exist.
2. **Given** an agent request that includes a path outside every Root, **When** the tool runs, **Then** it reports denial and returns no such path.
3. **Given** an agent, **When** it requests file contents through this feature's tools, **Then** no such tool exists (or the request is rejected as unsupported).
4. **Given** a huge result set, **When** the agent omits a limit, **Then** the server returns at most 20 Compact hits, a cursor if more exist, and a payload under the page byte budget.
5. **Given** a successful search, **When** the agent reads the tool result, **Then** each hit appears once as structured fields (relative path, kind, match span) — not also as a concatenated path list.

---

### User Story 4 - Inspect a folder without listing the live NAS (Priority: P4)

The operator or agent asks for the immediate children of a folder they are allowed to see. The answer comes from the catalog. Deep expansion is capped so a pathological directory cannot flood the session.

**Why this priority**: Useful for navigation after a search hit, but not required for the first useful search.

**Independent Test**: Catalog a tree, request children of a known folder, confirm names and types match, then request a folder outside Roots and confirm denial.

**Acceptance Scenarios**:

1. **Given** a cataloged folder with files and subfolders, **When** the user asks for its children, **Then** they receive those names, types, and sizes as of the last refresh.
2. **Given** a folder with more children than the expansion cap, **When** they ask for children, **Then** they receive a truncated page and a continuation signal.
3. **Given** a path outside Roots, **When** they ask for children, **Then** the request is denied.

---

### User Story 5 - Configure Roots, excludes, and a portable catalog location (Priority: P5)

The operator declares one or more Roots, optional exclude patterns (for example trash, recycle, or cache directories), and where the catalog file lives. The same configuration works for a human command and for an agent session. Another machine can be given the same Roots (if it can see them) and rebuild.

**Why this priority**: Generality for an open-source tool. Necessary before a public repo is honest about multi-root use, but not required to prove search.

**Independent Test**: Write a config with two Roots and one exclude. Build. Search. Confirm excluded paths never appear and each Root's files do.

**Acceptance Scenarios**:

1. **Given** two Roots, **When** the operator searches, **Then** hits from both appear and each hit identifies which Root it belongs to.
2. **Given** an exclude pattern matching a cache directory, **When** the catalog is built, **Then** no path under that directory is searchable.
3. **Given** a missing Root path, **When** a build starts, **Then** the operator receives a clear error naming that Root and no partial silent skip unless they opted into "skip missing."

---

### Edge Cases

- Root path does not exist, is a file, or is unreadable.
- Path contains characters illegal on another OS; catalog is used on the OS that produced it.
- Filename is valid on disk but not valid UTF-8 (Unix).
- Filename differs only by Unicode normalization or letter case.
- Directory is a symbolic link, junction, or symlink loop.
- Network share is briefly unreachable during refresh.
- Catalog file is on a different volume from the Root.
- Two Roots where one is nested inside the other.
- Query is empty, extremely long, or full of combining marks.
- Concurrent search while a refresh is swapping in a new catalog.
- Operator runs the agent interface bound to a network port without a secret (MUST be rejected).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The product MUST let an operator declare one or more Roots that bound every walk and every result.
- **FR-002**: The product MUST build a durable catalog of file and directory metadata under those Roots (path, name, extension, size, modification time, type, and OS file owner when the platform provides it).
- **FR-003**: The product MUST search the catalog using any combination of: filename fragment, path fragment, extension, owner, modification-time range, and size range. Hits MUST be ordered by modification time newest-first unless the caller requests another order.
- **FR-004**: The product MUST treat an extension filter as an exact (case-insensitive) match on the recorded extension (with or without a leading dot).
- **FR-005**: The product MUST NOT offer file-content search or file-content read in this feature.
- **FR-006**: The product MUST refresh a catalog incrementally, skipping directory trees whose recorded modification time still matches disk.
- **FR-007**: A search MUST be answered from the catalog, never by walking the live tree.
- **FR-008**: Every path emitted to a human or an agent MUST jail against declared Roots after canonicalization. Failures deny, they do not skip the check.
- **FR-009**: Symbolic links and directory junctions MUST NOT be followed unless the operator explicitly enables following, per Root.
- **FR-010**: Default page size MUST be 200. The hard ceiling MUST be 5000. Requested pages larger than 5000 MUST be clamped to 5000.
- **FR-010a**: The Catalog MUST project Hits through a View. Default View includes relative path, kind, extension, size, mtime, and owner. Full View also includes the absolute display path. Adapters MUST NOT re-serialize the same hits as prose.
- **FR-010b**: The Catalog MUST enforce a serialized Page budget of 128 KiB so a full 200-hit Compact page fits. If hits would exceed the budget, the Catalog MUST return fewer hits and a continuation cursor.
- **FR-011**: Each text filter (filename, path, owner) longer than 256 characters MUST be rejected. A query with no text fragment is valid and returns the newest files matching the other filters (or the newest 200 files if no filter is set).
- **FR-012**: The agent interface MUST expose search and folder-children tools only, both paginated, both jailed.
- **FR-013**: The human interface MUST expose build, refresh, search, and children commands that share configuration with the agent interface.
- **FR-014**: Exclude patterns MUST be applied at catalog build and refresh time so excluded paths are absent from search.
- **FR-015**: Catalog files MUST be created with owner-only readability on platforms that support it.
- **FR-016**: The product MUST keep an OS-faithful path and a search-normalized path so NFC/NFD and case-fold equivalents can match without destroying the on-disk name.
- **FR-017**: Refresh MUST be atomic from a searcher's point of view: previous complete catalog or new complete catalog, plus an explicit "rebuild in progress" status.
- **FR-018**: The product MUST refuse to start an agent network listener without an operator-supplied shared secret. Standard input/output is the default agent transport.
- **FR-019**: The product MUST record build and refresh status (counts, started/finished times, last error) so an operator can tell whether answers are fresh.
- **FR-020**: Nested Roots MUST be rejected or automatically collapsed to the outer Root, with the chosen rule documented and tested. Default: reject with an error.
- **FR-021**: Unbounded regular-expression search MUST NOT be available in this feature.

### Key Entities

- **Root**: An operator-declared directory that the product is allowed to see. Attributes: path, follow-links flag, optional exclude patterns, display name.
- **Catalog**: The durable, searchable snapshot of metadata under one configuration of Roots. Attributes: format version, built-at, entry count, completeness.
- **Entry**: One file or directory in the Catalog. Attributes: faithful path, normalized name, extension, size, modification time, kind, parent, owning Root.
- **Query**: A search request. Attributes: fragment, optional glob, optional Root restriction, page size, page cursor.
- **Page**: A bounded list of Entries plus whether more exist and a cursor.
- **Jail Decision**: Allow or deny for a candidate path relative to Roots.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On a warm catalog of 30 million entries, a typical filename-fragment search returns its first page in under 50 milliseconds at the 95th percentile on a contemporary workstation.
- **SC-002**: A refresh of an unchanged 30-million-entry tree finishes in under 2 minutes on a local disk and does not enumerate files inside unchanged directories.
- **SC-003**: 100% of search and children results in tests lie inside declared Roots, including cases that attempt `..`, symlink escape, and sibling-volume paths.
- **SC-004**: Mixed-script and NFC/NFD equivalent names are found in at least the fixtures defined in the quality checklist (CJK, accented Latin, combining marks).
- **SC-005**: A search that matches more than 200 entries still returns at most 200 hits, newest modification time first, and the JSON page stays under 128 KiB.
- **SC-006**: A first-time operator can install the single program, point it at a folder, and complete a successful search in under 10 minutes on a tree of at least 10,000 files.
- **SC-007**: The default distribution is one program. A default build introduces no listening network port and no outbound telemetry.
- **SC-008**: Query-time working set for a 30-million-entry catalog stays under 400 MB RSS on a warm machine while answering SC-001 searches.

## Assumptions

- Operators can already mount SMB/NFS (or equivalent) through the operating system. The product treats those mounts as ordinary directories.
- First release is metadata-only. Content search may be a later feature with its own spec and a stricter threat model.
- "Warm catalog" means the catalog file has been opened once and the operating system may cache its pages.
- Workstation-class means roughly 8 logical CPUs and 16 GB RAM, not a dedicated search cluster.
- Default matching is case-insensitive on Windows Roots and case-sensitive on Unix Roots, always after Unicode normalization. This matches user expectation on each platform.
- A public open-source repository is the destination; defaults favor safety over convenience when those conflict.
- Scheduled refresh (cron / Task Scheduler) is the operator's job. The product provides a refresh command, not a hidden resident daemon, in this feature.
- One catalog file per configuration. Sharing a catalog across machines is unsupported if those machines do not see identical Root paths.
