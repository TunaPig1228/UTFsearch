# UTFsearch

A local catalog of file metadata under operator-declared Roots, searchable by humans and AI agents. It exists so tens of millions of deeply nested, multi-script paths can be found without walking the live tree and without widening what an agent is allowed to see.

## Language

**Root**:
An operator-declared directory that the product is allowed to see. Every walk and every emitted path belongs to a Root.
_Avoid_: workspace, mount, share, MCP root, client root

**Catalog**:
The durable, searchable snapshot of metadata under a configuration of Roots. Search answers come from the Catalog, never from a live walk.
_Avoid_: index database, DuckDB, corpus

**Entry**:
One file or directory recorded in the Catalog.
_Avoid_: document, inode, record, row

**Query**:
A bounded request to search the Catalog. Filters: filename, path, extension, owner, mtime range, size range. Default order is newest mtime first. Default page is 200; maximum is 5000.
_Avoid_: SQL, regex, prompt

**Owner**:
The operating-system file owner recorded at catalog time (Windows account or Unix user). This is the product's 修改者. It is not an in-file document author.
_Avoid_: author, last editor, ACL

**Page**:
A budgeted slice of Hits: count-capped and byte-capped. The Catalog, not an adapter, enforces the budget.
_Avoid_: result dump, stream, context, tool dump

**View**:
How much of a Hit is projected into a Page. Compact is the agent default (relative path, kind, match span). Full is the human default (absolute path plus size and time).
_Avoid_: format, projection (when you mean the product concept)

**Jail**:
The decision that a candidate path is or is not inside a Root after canonicalization. Catalog runs Jail before a Hit exists.
_Avoid_: sanitizer, validator, filter, sandbox (the last is too broad)

**Hit**:
One Entry selected by a Query, already through Jail, projected through a View.
_Avoid_: match row, document

**Walker**:
The module that enumerates a Root to (re)build a Catalog.
_Avoid_: scanner, crawler, jwalk

**Watcher**:
An optional adapter that notices disk changes so a refresh can start. Not a source of search answers.
_Avoid_: CDC pipeline, eBPF sensor

**Adapter**:
A concrete thing that satisfies an interface at a seam (CLI, MCP, a particular Walker or Watcher).
_Avoid_: service, plugin, integration
