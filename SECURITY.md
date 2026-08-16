# Security

UTFsearch is a local catalog. An AI agent using the MCP adapter is a confused deputy.

- Only operator-declared Roots are visible.
- Every emitted path is jailed after a lexical join (and canonicalize when the path exists).
- Symlinks and junctions are not followed unless a Root sets `follow_links`.
- The catalog stores metadata only. There is no `read_file` tool.
- Default page size is 200. Hard ceiling is 5000.
- HTTP MCP does not start without `--token` / `UTFSEARCH_MCP_TOKEN`. Stdio is the default.
- Catalog files are created mode `0600` on Unix.
- Client-supplied MCP Roots are ignored for authorization.

Report issues privately to the maintainers before a public advisory.
