# Contract: MCP tools

SDK: official `rmcp`, protocol 2026-07-28 (compatible with 2025-11-25).

Authorization: **operator config Roots only**. Client-declared MCP Roots are ignored for access control (MCP Roots deprecated, SEP-2577; also a confused-deputy hazard).

Transport: stdio default. HTTP only with shared token (header or query is defined at implement time; prefer `Authorization: Bearer`).

Return **only** `structuredContent` matching `outputSchema`. `content` text MUST be a one-line summary (`"12 hits, more available"`) — never the hit list again.

Default View is Compact. Default and maximum `limit` is 200. Catalog truncates to 128 KiB. Hits are newest-mtime first.

## `search_files`

Description: Search the catalog by a Unicode-normalized literal fragment of filename or path. Does not read file contents. Does not walk the live disk. Returns Compact hits (relative path, kind, match span).

Input:

| Field | Type | Required | Constraints |
| --- | --- | --- | --- |
| name | string | no | filename contains |
| path | string | no | relative path contains |
| fragment | string | no | name **or** path contains (convenience) |
| ext | string | no | extension, with or without dot |
| owner | string | no | 修改者 / OS owner contains |
| after | string | no | mtime ≥ YYYY-MM-DD or unix seconds |
| before | string | no | mtime ≤ YYYY-MM-DD or unix seconds |
| min_size | string | no | e.g. `10k`, `2m` |
| max_size | string | no | |
| root | string | no | Root id or display name |
| limit | integer | no | default 200, max 200 |
| cursor | string | no | opaque |
| full | boolean | no | include absolute path |

Output (Compact):

```json
{
  "hits": [ { "rel": "docs/發票.xlsx", "root": "nas", "kind": "file", "ext": "xlsx", "size": 12044, "mtime": 1710000000, "owner": "ada" } ],
  "more": false,
  "next_cursor": null,
  "dropped_unsafe": 0
}
```

Errors: invalid params (protocol error). No matches → success with empty hits (tool-level, not protocol error). Jail failure drops that hit and increments `dropped_unsafe`.

## `list_children`

Description: List immediate catalog children of a directory path that jails to a Root.

Input: `path` (string, required; must jail), `limit` (default 200, max 200), `cursor`.

Output: `{ "entries": [ { "name", "kind" } ], "more", "next_cursor" }`. No size/mtime unless `full: true`.

Outside Roots → tool error with message that does not disclose sibling paths.

## `catalog_status`

Input: none. Output: `{ "complete", "entry_count", "built_at", "roots": [ { "name", "path", "state" } ], "build_in_progress" }`.

## `refresh_catalog` (optional, long)

If exposed, MUST run as an MCP Task (SEP-2663) so the agent polls. MUST NOT be available over HTTP unless the token is present. Default feature set MAY omit this tool and keep refresh CLI-only to shrink agent privilege; implement CLI-only first.

## Non-goals

No `read_file`. No `exec`. No `add_root`. No regex tool.
