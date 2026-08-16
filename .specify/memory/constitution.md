<!--
Sync Impact Report
- Version change: (none) → 1.0.0
- Modified principles: (initial ratification)
- Added sections: Core Principles I–IX; Security & Generality Constraints; Development Workflow; Governance
- Removed sections: none
- Follow-up TODOs: none
-->

# UTFsearch Constitution

## Core Principles

### I. Library-First
Every capability MUST exist first as a library module with a small
interface. CLI and MCP are adapters at that interface. No feature MAY
be implemented only inside a protocol handler. A module that fails the
deletion test (complexity merely moves to callers) MUST be collapsed.

### II. Dual Surface
Every library capability MUST be reachable from a CLI: arguments or
stdin in, human text or JSON out, errors on stderr. The MCP adapter
MUST call the same interface. A behavior that exists only on one
surface is a defect.

### III. Test-First (NON-NEGOTIABLE)
Implementation MUST follow tests at the module interface: write the
test, confirm it fails, then implement. Contract tests for CLI and MCP
MUST exist before those adapters grow behavior. Tests exercise the
same interface callers use. Reaching past the interface to assert on
private types is forbidden except for documented internal seams.

### IV. Capability Security
Access is deny-by-default. The operator MUST name every Root before
any walk or search proceeds. Every path that leaves the catalog MUST
pass canonicalize-and-jail against those Roots. The default product
indexes metadata only — never file contents. Results, query length,
and tree expansion MUST be hard-capped. Follow-symlink and
follow-junction default to off. HTTP transport is opt-in and MUST
require a shared secret; stdio is the default.

### V. Compactness
The default binary MUST stay a single static-enough executable with
no C/C++ engine, no out-of-process database, and no default network
listener. Default cargo features MUST exclude optional OS journals
and HTTP. A dependency is admitted only when it reduces interface
size or working-set size. Binary growth and query RSS are first-class
regressions.

### VI. Incremental Truth
The Catalog is the searchable truth. A full walk is the last resort.
Directory mtime prune is the portable incremental baseline and MUST
work on local disks and on OS-mounted SMB/NFS. Optional watchers
(notify, USN, FSEvents, inotify) are adapters behind one Watcher
seam. A search MUST never require a live walk of the tree.

### VII. Simplicity
The initial tree MUST contain at most three crates. Prefer one library
crate plus one binary crate. A fourth crate, a new cargo feature, or a
new protocol requires a written justification in the plan's Complexity
Tracking table. Do not add a seam until a second adapter exists.
YAGNI applies to query languages, content search, and remote protocols.

### VIII. Unicode Fidelity
Paths are first-class Unicode values, not "probably UTF-8 strings."
The catalog MUST retain an OS-faithful form and a search-normalized
form (NFC, compatible case-fold). Invalid-UTF-8 Unix paths MUST remain
addressable. Cross-platform comparison MUST document Windows
case-insensitivity and macOS NFD as invariants, not accidents.

### IX. Integration-First Testing
Tests MUST use real temporary filesystems, including deep nesting,
non-ASCII names, and permission failures. Fakes are allowed only at
declared seams that already have two adapters. Contract tests for
published CLI flags and MCP tools are mandatory before those surfaces
ship.

## Security & Generality Constraints

The product is an open-source, local-first catalog. It MUST run on
Windows, Linux, and macOS against whatever the OS already mounts.
It MUST NOT implement SMB or NFS itself.

Threat model (non-negotiable):
- An LLM agent is a confused deputy. Tools MUST NOT widen Roots.
- Path traversal, symlink escape, and UNC/junction tricks are in scope.
- Query input is untrusted: default to literal matching. Unbounded
  regular expressions are forbidden.
- Index files MAY contain sensitive path strings. They MUST be created
  with user-only permissions on Unix and an equivalent ACL intent on
  Windows.
- Telemetry that leaves the machine is forbidden unless the operator
  explicitly enables it. Default is none.

Performance budget (workstation-class, warm catalog, 30 million
entries):
- Interactive search p95 under 50 ms for a typical filename fragment.
- Incremental update of an unchanged tree finishes without visiting
  pruned descendants.
- Query-time RSS stays in the low hundreds of megabytes by mapping
  the catalog rather than copying it.

## Development Workflow

1. Change the spec when intent changes. Change the plan when the
   how changes. Do not "just patch the code."
2. New public interface surface requires a contract test.
3. Security-sensitive changes (Roots, jail, transports, query
   parsers) require an extra review pass against this constitution.
4. Dependency additions MUST record binary-size and license impact.
5. Public crates follow semver. Removing or tightening a Root
   invariant is a MAJOR change.

## Governance

This constitution supersedes informal practice, README claims, and
generated plans. Amendments require:
- A written rationale and migration note in this file.
- A semantic version bump (MAJOR for removed or redefined principles,
  MINOR for added principles, PATCH for wording).
- `LAST_AMENDED_DATE` set to the amendment day.

Compliance review: every `/speckit-plan` run MUST fill Constitution
Check. Unjustified violations halt planning. Maintainers MAY refuse
code that enlarges the default binary, adds an implicit network
surface, or searches outside configured Roots.

**Version**: 1.0.0 | **Ratified**: 2026-08-16 | **Last Amended**: 2026-08-16
