# Purpose-built mmap catalog, not DuckDB / DataFusion / Tantivy

The source architecture used an embedded columnar SQL engine so 30 million path strings could be queried with `contains`. That engine is the wrong depth for this domain: callers would learn SQL, FFI, and scan semantics, while the actual problem is a compressed path dictionary plus substring lookup.

We store interned path components, filename trigrams, and a versioned mmap file. Search is a library call (`Catalog::search`). DuckDB (C++ FFI, large binary), DataFusion+Arrow (huge crate graph), and Tantivy (content-search engine) stay out of the default build.

**Status**: accepted

**Considered Options**: DuckDB-rs, DataFusion+Parquet, Tantivy, custom catalog

**Consequences**: We own a file format (see `contracts/catalog-format.md`). Content search, if it ever happens, is a new spec and may introduce Tantivy behind a feature — not a retrofit of SQL into path search.
