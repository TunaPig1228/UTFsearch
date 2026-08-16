# Quickstart validation: UTFsearch Catalog

This is a validation script for implementers, not user marketing. Commands assume the workspace binary `utfsearch` is on PATH after `cargo build -p utfsearch`.

## Prerequisites

- Rust toolchain from `rust-toolchain.toml`
- A writable temp directory
- No extra privileges (USN/eBPF features off)

## Fixture

Create a tree (names must be exact):

```
<tmp>/root/
  docs/
    發票.xlsx
    cafe.txt          (NFC é if possible)
  deep/a/b/c/d/e/f/g/h/i/j/
    needle.txt
  skipme/
    secret.bin
```

Config:

```toml
catalog = "<tmp>/catalog.uts"
[[roots]]
name = "demo"
path = "<tmp>/root"
excludes = ["skipme"]
```

## Scenarios

1. **Build and search (US1)**  
   `utfsearch --config <cfg> index`  
   `utfsearch --config <cfg> --format json search 發票`  
   Expect one hit whose path ends with `docs/發票.xlsx`.  
   `search needle` finds the deep file.  
   `search secret` returns empty hits.

2. **Unicode (US1 / SC-004)**  
   Search both NFC and NFD forms of `café` if the fixture encoded both; at least one form must hit `cafe.txt` after normalization.

3. **Incremental prune (US2)**  
   `utfsearch --config <cfg> --format json refresh` on an unchanged tree.  
   Expect `pruned_dirs` > 0 and `visited_dirs` far smaller than the full tree.  
   Add `docs/new.txt`, refresh, `search new` hits.

4. **Jail (US3 / SC-003)**  
   `utfsearch --config <cfg> tree <tmp>/root/../..` exits 2.  
   MCP `list_children` with that path returns a tool error.

5. **Agent page cap (US3 / SC-005)**  
   Generate 80 files named `bulk-NN.txt`.  
   Generate 250 files named `bulk-NN.txt` with increasing mtimes.  
   MCP `search_files` `{ "fragment": "bulk" }` returns 200 hits, newest first, `more: true`.  
   `search --ext txt --name bulk` from CLI matches the same set.  
   Second call with cursor returns the rest.

6. **HTTP refused (FR-018)**  
   `utfsearch mcp --http 127.0.0.1:0` without token exits 2.

7. **Status**  
   `utfsearch --format json status` shows `complete: true` and `entry_count` matching the fixture (excluding `skipme`).

Do not treat this file as an implementation dump. Implementation lives in `tasks.md`.
