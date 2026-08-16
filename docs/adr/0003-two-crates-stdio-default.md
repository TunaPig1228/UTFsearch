# Two crates, one binary, stdio default

Open-source generality wants CLI and MCP. Compactness forbids a daemon and a second engine. The core library is the deep module; `utfsearch` is the binary with `search` / `index` / `mcp` subcommands.

A Watcher seam exists because prune-only, `notify`, and USN are real adapters. eBPF is not an adapter in v1.

**Status**: accepted
