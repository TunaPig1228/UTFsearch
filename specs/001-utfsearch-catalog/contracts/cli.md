# Contract: CLI

Binary: `utfsearch`

Global flags (all subcommands):

| Flag | Default | Notes |
| --- | --- | --- |
| `--config <path>` | `./utfsearch.toml` then user config dir | Missing file is an error except `utfsearch init`. |
| `--catalog <path>` | from config | Overrides catalog path. |
| `--format text\|json` | `text` | JSON is the machine surface (Constitution II). |
| `--quiet` | off | Suppress progress on stderr. |

Exit codes: `0` success, `1` usage, `2` config/jail, `3` catalog missing/corrupt, `4` IO, `5` query rejected.

## `utfsearch init`

Writes a starter config. Does not walk.

## `utfsearch index [--root <path>]`

First run: `--root` required (repeatable). Later runs omit `--root` and rebuild the remembered Roots.

Default catalog is `catalog.uts` next to the executable. `--catalog` is remembered in `utfsearch.last` beside the exe.

## `utfsearch refresh`

Incremental update. Roots come from the catalog unless `--root` is passed again.

Stderr: progress counts. Stdout (json): `{ "entries": N, "catalog": "<path>" }`.

## `utfsearch refresh`

Incremental update. Same output shape as `index`, plus `{ "pruned_dirs": N, "visited_dirs": N }`.

## `utfsearch search <fragment>`

Flags: `--name`, `--path`, `--ext`, `--owner`, `--after`, `--before`, `--min-size`, `--max-size`, `--root`, `--limit` (default 200, max 5000), `--cursor`, `--full`.

Positional fragment is name-or-path. Default order is mtime newest-first.

Default View is Full. JSON stdout (Full):

```json
{
  "hits": [
    {
      "rel": "docs/發票.xlsx",
      "path": "\\\\nas\\share\\發票.xlsx",
      "root": "nas",
      "kind": "file",
      "size": 12044,
      "mtime": 1710000000,
      "score": 1.0
    }
  ],
  "more": false,
  "next_cursor": null,
  "dropped_unsafe": 0
}
```

Empty fragment → exit 5.

## `utfsearch tree <path>`

Immediate children from the catalog. `--limit`, `--cursor`. Path must jail.

## `utfsearch status`

JSON: completeness, entry_count, built_at, roots, last_error.

## `utfsearch mcp`

Runs the MCP server on stdio. `--http <addr>` requires `--token` (or `UTFSEARCH_MCP_TOKEN`). Missing token with `--http` → exit 2.
