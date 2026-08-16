use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use serde_json::{json, Value};
use utfsearch_core::{
    parse_size, parse_time, Catalog, Cursor, Error, Query, Result, View, DEFAULT_LIMIT,
};

pub fn run(catalog: PathBuf, http: Option<String>, token: Option<String>) -> Result<()> {
    if http.is_some() {
        let tok = token
            .or_else(|| std::env::var("UTFSEARCH_MCP_TOKEN").ok())
            .filter(|s| !s.is_empty());
        if tok.is_none() {
            return Err(Error::Msg(
                "HTTP MCP requires --token or UTFSEARCH_MCP_TOKEN".into(),
            ));
        }
        return Err(Error::Msg(
            "HTTP transport is not in the default build; use stdio".into(),
        ));
    }
    let cat = Catalog::open(&catalog)?;
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if req.get("id").is_none() {
            continue; // notification
        }
        let resp = handle(&cat, &req);
        writeln!(
            stdout,
            "{}",
            serde_json::to_string(&resp).map_err(|e| Error::Msg(e.to_string()))?
        )?;
        stdout.flush()?;
    }
    Ok(())
}

fn handle(cat: &Catalog, req: &Value) -> Value {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(json!({}));
    match method {
        "initialize" => ok(
            id,
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "utfsearch", "version": env!("CARGO_PKG_VERSION") }
            }),
        ),
        "ping" => ok(id, json!({})),
        "tools/list" => ok(id, json!({ "tools": tools() })),
        "tools/call" => match call(cat, &params) {
            Ok(v) => ok(id, v),
            Err(e) => fail(id, -32000, &e.to_string()),
        },
        _ => fail(id, -32601, "method not found"),
    }
}

fn tools() -> Value {
    json!([
        {
            "name": "search_files",
            "description": "Search the catalog by name, path, extension, owner, mtime, or size. Newest first. Default 200 hits, max 5000. Does not read file contents.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "fragment": { "type": "string" },
                    "name": { "type": "string" },
                    "path": { "type": "string" },
                    "ext": { "type": "string" },
                    "owner": { "type": "string" },
                    "after": { "type": "string" },
                    "before": { "type": "string" },
                    "min_size": { "type": "string" },
                    "max_size": { "type": "string" },
                    "root": { "type": "string" },
                    "limit": { "type": "integer", "maximum": 5000 },
                    "cursor": { "type": "string" },
                    "full": { "type": "boolean" }
                }
            }
        },
        {
            "name": "list_children",
            "description": "List catalog children of a directory that jails to a Root.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "root": { "type": "string" },
                    "full": { "type": "boolean" }
                },
                "required": ["path"]
            }
        },
        {
            "name": "catalog_status",
            "description": "Catalog completeness and Root list.",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ])
}

fn call(cat: &Catalog, params: &Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Msg("missing tool name".into()))?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    match name {
        "search_files" => {
            let page = cat.search(query_from(&args)?)?;
            Ok(tool_result(&page, page.hits.len()))
        }
        "list_children" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Query("path required"))?;
            let root = args.get("root").and_then(|v| v.as_str());
            let full = args.get("full").and_then(|v| v.as_bool()).unwrap_or(false);
            let view = if full { View::Full } else { View::Compact };
            let page = cat.children_of(&PathBuf::from(path), root, view)?;
            Ok(tool_result(&page, page.hits.len()))
        }
        "catalog_status" => {
            let st = cat.status();
            Ok(tool_result(&st, 0))
        }
        _ => Err(Error::Msg(format!("unknown tool {name}"))),
    }
}

fn query_from(args: &Value) -> Result<Query> {
    let mut q = Query::new();
    q.name = args
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| {
            args.get("fragment")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        });
    q.path = args.get("path").and_then(|v| v.as_str()).map(str::to_string);
    q.ext = args.get("ext").and_then(|v| v.as_str()).map(str::to_string);
    q.owner = args
        .get("owner")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    q.root = args.get("root").and_then(|v| v.as_str()).map(str::to_string);
    if let Some(s) = args.get("after").and_then(|v| v.as_str()) {
        q.mtime_min = Some(parse_time(s)?);
    }
    if let Some(s) = args.get("before").and_then(|v| v.as_str()) {
        q.mtime_max = Some(parse_time(s)?);
    }
    if let Some(s) = args.get("min_size").and_then(|v| v.as_str()) {
        q.size_min = Some(parse_size(s)?);
    }
    if let Some(s) = args.get("max_size").and_then(|v| v.as_str()) {
        q.size_max = Some(parse_size(s)?);
    }
    q.limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n as u16)
        .unwrap_or(DEFAULT_LIMIT);
    if let Some(c) = args.get("cursor").and_then(|v| v.as_str()) {
        q.cursor = Some(Cursor::decode(c)?);
    }
    q.view = if args.get("full").and_then(|v| v.as_bool()).unwrap_or(false) {
        View::Full
    } else {
        View::Compact
    };
    Ok(q)
}

fn tool_result<T: serde::Serialize>(val: &T, n: usize) -> Value {
    let structured = serde_json::to_value(val).unwrap_or(json!({}));
    json!({
        "content": [{ "type": "text", "text": format!("{n} hits") }],
        "structuredContent": structured,
        "isError": false
    })
}

fn ok(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn fail(id: Value, code: i64, msg: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": msg } })
}
