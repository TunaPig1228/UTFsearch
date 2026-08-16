---
description: "Task list for UTFsearch Catalog"
---

# Tasks: UTFsearch Catalog

**Input**: Design documents from `/specs/001-utfsearch-catalog/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Included ??Constitution III and IX require interface tests before adapters grow behavior.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Workspace and crate skeletons with default-small dependencies

- [x] T001 Create Cargo workspace at `Cargo.toml` with members `crates/utfsearch-core` and `crates/utfsearch`
- [x] T002 [P] Add `rust-toolchain.toml`, `.gitignore`, and workspace lints in root `Cargo.toml`
- [x] T003 [P] Initialize `crates/utfsearch-core/Cargo.toml` and `crates/utfsearch-core/src/lib.rs` with empty public interface stubs
- [x] T004 [P] Initialize `crates/utfsearch/Cargo.toml` and `crates/utfsearch/src/main.rs` with clap subcommand placeholders
- [x] T005 [P] Add default dependencies listed in `specs/001-utfsearch-catalog/plan.md` with optional `watch`, `usn`, `http` features off

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Types, jail, normalize, config ??MUST complete before any user story

**? ï? CRITICAL**: No user story work can begin until this phase is complete

- [x] T006 Write failing tests for NFC/case-fold rules in `crates/utfsearch-core/src/normalize.rs`
- [x] T007 Implement path normalize (faithful + search key) in `crates/utfsearch-core/src/normalize.rs`
- [x] T008 [P] Define `Root`, `RootSet`, `Entry`, `Query`, `Page`, `Hit` types in `crates/utfsearch-core/src/root.rs` and `crates/utfsearch-core/src/entry.rs` and `crates/utfsearch-core/src/query.rs`
- [x] T009 Write failing jail tests (dot-dot, symlink, nested volume) in `tests/integration/jail_escape.rs`
- [x] T010 Implement canonicalize-and-jail in `crates/utfsearch-core/src/jail.rs`
- [x] T011 [P] Implement TOML config load/validate (nested Root reject, missing Root) in `crates/utfsearch/src/config.rs`
- [x] T012 [P] Implement structured error types and stderr/JSON error mapping in `crates/utfsearch/src/cli.rs`
- [x] T013 Expose the public interface only through `crates/utfsearch-core/src/lib.rs`

**Checkpoint**: Foundation ready ??user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Index a root and find a file by name (Priority: P1) ?Ž¯ MVP

**Goal**: Build a catalog and return paginated, Unicode-aware filename/path hits

**Independent Test**: Fixture tree from `quickstart.md` scenarios 1??

### Tests for User Story 1

- [x] T014 [P] [US1] Write failing Unicode fixture tests in `tests/integration/unicode_names.rs`
- [x] T015 [P] [US1] Write failing index-and-search integration test in `tests/integration/index_and_search.rs`
- [x] T016 [P] [US1] Write failing CLI contract test for `search` JSON in `tests/contract/cli_search.rs`

### Implementation for User Story 1

- [x] T017 [P] [US1] Implement component intern (fst) in `crates/utfsearch-core/src/catalog/intern.rs`
- [x] T018 [P] [US1] Implement filename trigram encode/intersect in `crates/utfsearch-core/src/catalog/trigram.rs`
- [x] T019 [P] [US1] Implement on-disk section layout read/write in `crates/utfsearch-core/src/catalog/format.rs`
- [x] T020 [US1] Implement Walker interface + `ignore` adapter with excludes in `crates/utfsearch-core/src/walk/mod.rs` and `crates/utfsearch-core/src/walk/ignore_walk.rs`
- [x] T021 [US1] Implement catalog writer (full build, atomic rename, 0600 perms) in `crates/utfsearch-core/src/catalog/writer.rs`
- [x] T022 [US1] Implement `Catalog::open` + `Catalog::search` (trigram ??verify ??nucleo rank ??page) in `crates/utfsearch-core/src/catalog/mod.rs`
- [x] T023 [US1] Implement `utfsearch index` and `utfsearch search` in `crates/utfsearch/src/cli.rs` and `crates/utfsearch/src/main.rs`
- [x] T024 [US1] Enforce query length 256, View projection, human limit 50 / agent limit 20 / max 200, and 8 KiB Page budget in `crates/utfsearch-core/src/query.rs` and `crates/utfsearch-core/src/catalog/mod.rs`

**Checkpoint**: US1 is a usable MVP on a single Root

---

## Phase 4: User Story 2 - Refresh without walking unchanged trees (Priority: P2)

**Goal**: Incremental merge using directory mtime prune; atomic swap

**Independent Test**: `quickstart.md` scenario 3

### Tests for User Story 2

- [x] T025 [P] [US2] Write failing prune visit-count test in `tests/integration/incremental_prune.rs`

### Implementation for User Story 2

- [x] T026 [US2] Persist and load `dir_stats` section in `crates/utfsearch-core/src/catalog/format.rs`
- [x] T027 [US2] Implement prune callback in `crates/utfsearch-core/src/watch/prune.rs` wired into the walker
- [x] T028 [US2] Implement incremental merge + atomic replace in `crates/utfsearch-core/src/catalog/writer.rs`
- [x] T029 [US2] Implement `utfsearch refresh` reporting `pruned_dirs` and `visited_dirs` in `crates/utfsearch/src/cli.rs`
- [x] T030 [US2] Keep previous complete map readable during rebuild in `crates/utfsearch-core/src/catalog/mod.rs`

**Checkpoint**: Unchanged trees are not enumerated

---

## Phase 5: User Story 3 - AI agent sandboxed search (Priority: P3)

**Goal**: Official MCP stdio server over the same Catalog interface

**Independent Test**: `quickstart.md` scenarios 4?? via an MCP client or contract harness

### Tests for User Story 3

- [x] T031 [P] [US3] Write failing MCP `search_files` contract test in `tests/contract/mcp_search.rs`
- [x] T032 [P] [US3] Extend jail tests to MCP `list_children` denial in `tests/integration/jail_escape.rs`

### Implementation for User Story 3

- [x] T033 [US3] Implement `search_files`, `list_children`, `catalog_status` with structured pages in `crates/utfsearch/src/mcp.rs`
- [x] T034 [US3] Implement `utfsearch mcp` stdio in `crates/utfsearch/src/main.rs`
- [x] T035 [US3] Reject `--http` without token in `crates/utfsearch/src/mcp.rs`
- [x] T036 [US3] Ignore client-supplied MCP Roots for authorization in `crates/utfsearch/src/mcp.rs`

**Checkpoint**: Agent search is jailed and page-capped

---

## Phase 6: User Story 4 - Inspect a folder from the catalog (Priority: P4)

**Goal**: Children listing with expansion cap

**Independent Test**: `utfsearch tree` on fixture; outside-root denied

- [x] T037 [P] [US4] Write failing children tests in `tests/integration/index_and_search.rs`
- [x] T038 [US4] Implement `Catalog::children` in `crates/utfsearch-core/src/catalog/mod.rs`
- [x] T039 [US4] Implement `utfsearch tree` in `crates/utfsearch/src/cli.rs`

**Checkpoint**: Navigation does not touch the live NAS

---

## Phase 7: User Story 5 - Multi-root config and excludes (Priority: P5)

**Goal**: Portable config, multi-root search, excludes at build time

**Independent Test**: `quickstart.md` fixture plus a second Root

- [x] T040 [P] [US5] Write failing two-root and exclude tests in `tests/integration/index_and_search.rs`
- [x] T041 [US5] Enforce nested-root rejection in `crates/utfsearch-core/src/root.rs`
- [x] T042 [US5] Tag hits with `root` name in search output (`crates/utfsearch-core/src/catalog/mod.rs`, `crates/utfsearch/src/cli.rs`)
- [x] T043 [US5] Implement `utfsearch init` starter TOML in `crates/utfsearch/src/cli.rs`
- [x] T044 [US5] Implement `utfsearch status` in `crates/utfsearch/src/cli.rs`

**Checkpoint**: Config is the single source of Roots for CLI and MCP

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Open-source readiness, optional adapters, budgets

- [x] T045 [P] Add README install/security section in `README.md`
- [x] T046 [P] Add `LICENSE` (MIT OR Apache-2.0) once the maintainer confirms
- [x] T047 [P] Document optional `watch` adapter in `crates/utfsearch-core/src/watch/notify_watch.rs` behind feature `watch`
- [x] T048 [P] Document optional USN adapter stub in `crates/utfsearch-core/src/watch/usn_watch.rs` behind feature `usn`
- [x] T049 Run `quickstart.md` end-to-end and record results in `specs/001-utfsearch-catalog/quickstart.md`
- [x] T050 Measure stripped default binary size and a 100k-file search p95; fail the task if default features pull DuckDB/Arrow/DataFusion
- [x] T051 [P] Add SECURITY.md threat-model short form at repo root

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies
- **Foundational (Phase 2)**: Depends on Setup ??BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational
  - US1 (P1) first ??MVP
  - US2 depends on US1 catalog format
  - US3 depends on US1 search interface (not on US2)
  - US4 depends on US1 entries
  - US5 depends on US1 RootSet
- **Polish**: After desired stories

### User Story Dependencies

- **User Story 1 (P1)**: After Phase 2
- **User Story 2 (P2)**: After US1 writer/format
- **User Story 3 (P3)**: After US1 `Catalog::search`
- **User Story 4 (P4)**: After US1 entries
- **User Story 5 (P5)**: After US1; can proceed in parallel with US3/US4

### Parallel Opportunities

- T002?“T005 in Setup
- T008, T011, T012 in Foundational after T007 types exist
- T014?“T016 tests in parallel
- T017?“T019 intern/trigram/format in parallel
- T031?“T032 MCP tests in parallel
- US3, US4, US5 can proceed in parallel after US1

---

## Parallel Example: User Story 1

```bash
# tests together
Task: "Write failing Unicode fixture tests in tests/integration/unicode_names.rs"
Task: "Write failing index-and-search integration test in tests/integration/index_and_search.rs"
Task: "Write failing CLI contract test for search JSON in tests/contract/cli_search.rs"

# catalog internals together
Task: "Implement component intern in crates/utfsearch-core/src/catalog/intern.rs"
Task: "Implement filename trigram encode/intersect in crates/utfsearch-core/src/catalog/trigram.rs"
Task: "Implement on-disk section layout in crates/utfsearch-core/src/catalog/format.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 Setup
2. Phase 2 Foundational
3. Phase 3 User Story 1
4. STOP and validate quickstart scenarios 1??

### Incremental Delivery

1. Setup + Foundational
2. US1 ??demo human search
3. US2 ??NAS-viable refresh
4. US3 ??agent surface
5. US4 + US5 ??navigation and multi-root

---

## Notes

- Do not add DuckDB, DataFusion, Arrow, Parquet, or Tantivy in any default task
- `refresh_catalog` MCP tool is deferred (CLI-only) to keep agent privilege small
- eBPF/`aya` is out of scope
