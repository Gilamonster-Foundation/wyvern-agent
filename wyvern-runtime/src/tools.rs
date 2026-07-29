//! The flight-tier tool set — the four primitives a worker needs to change a
//! codebase: run a shell command, read a file, write a file, edit a file. That
//! is the whole surface. No skills, no MCP, no plan ledger — those are cockpit
//! comforts the flight tier strips.
//!
//! Every path is fenced to the workspace root: a `..` escape or an absolute path
//! that resolves outside the tree is refused, so a worker cannot reach beyond
//! its sortie. Output is capped head+tail so one verbose command can't blow the
//! context window (the overflow lesson, carried over).

use std::path::{Component, Path, PathBuf};

use serde_json::{json, Value};

use crate::chat::ToolCall;

/// Head+tail char budget for a tool result's model-facing payload. Conservative
/// on purpose: dense output tokenizes far denser than prose, and a single result
/// must never overrun a served window on its own.
const MAX_RESULT_CHARS: usize = 24_000;

/// The tools array to advertise on the wire — the four primitives.
pub fn tool_definitions() -> Value {
    json!([
        tool("run_command", "Run a shell command in the workspace and return its combined stdout+stderr. Use this to build, run tests, and inspect. Prefer running the task's tests before concluding.", json!({
            "type": "object",
            "properties": { "command": { "type": "string", "description": "the shell command" } },
            "required": ["command"]
        })),
        tool("read_file", "Read a UTF-8 text file from the workspace.", json!({
            "type": "object",
            "properties": { "path": { "type": "string", "description": "workspace-relative path" } },
            "required": ["path"]
        })),
        tool("write_file", "Write (create or overwrite) a UTF-8 text file in the workspace.", json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["path", "content"]
        })),
        tool("edit_file", "Replace the first occurrence of old_string with new_string in a workspace file. old_string must be unique and match exactly.", json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "old_string": { "type": "string" },
                "new_string": { "type": "string" }
            },
            "required": ["path", "old_string", "new_string"]
        })),
    ])
}

fn tool(name: &str, description: &str, parameters: Value) -> Value {
    json!({ "type": "function", "function": {
        "name": name, "description": description, "parameters": parameters
    }})
}

/// Whether a tool name is one that mutates the workspace — the `write_calls`
/// signal the trace reports (a solve with zero writes never acted).
pub fn is_write(name: &str) -> bool {
    matches!(name, "write_file" | "edit_file")
}

/// Resolve a workspace-relative `rel` under `root`, refusing any path that would
/// escape the tree (a leading `/`, a `..` that climbs above root, …).
fn confine(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let mut out = PathBuf::from(root);
    for comp in Path::new(rel).components() {
        match comp {
            Component::Normal(c) => out.push(c),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() || !out.starts_with(root) {
                    return Err(format!("path `{rel}` escapes the workspace"));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!("absolute path `{rel}` is not allowed"));
            }
        }
    }
    if out.starts_with(root) {
        Ok(out)
    } else {
        Err(format!("path `{rel}` escapes the workspace"))
    }
}

/// Cap `s` to [`MAX_RESULT_CHARS`] as head + tail with an elision marker.
fn cap(s: String) -> String {
    let n = s.chars().count();
    if n <= MAX_RESULT_CHARS {
        return s;
    }
    let head: String = s.chars().take(MAX_RESULT_CHARS * 2 / 3).collect();
    let tail: String = s.chars().skip(n - MAX_RESULT_CHARS / 3).collect::<String>();
    let elided = n - MAX_RESULT_CHARS;
    format!("{head}\n\n[… {elided} chars elided (head+tail shown) …]\n\n{tail}")
}

/// Execute one tool call in `workspace`, returning the model-facing result
/// string. Never panics; every failure is a returned `error: …` string the model
/// can read and react to.
pub fn dispatch(call: &ToolCall, workspace: &Path) -> String {
    let args: Value = serde_json::from_str(&call.arguments).unwrap_or(Value::Null);
    let get = |k: &str| args.get(k).and_then(Value::as_str).unwrap_or("");
    match call.name.as_str() {
        "run_command" => run_command(get("command"), workspace),
        "read_file" => match confine(workspace, get("path")) {
            Ok(p) => match std::fs::read_to_string(&p) {
                Ok(s) => cap(s),
                Err(e) => format!("error: read_file `{}`: {e}", get("path")),
            },
            Err(e) => format!("error: {e}"),
        },
        "write_file" => match confine(workspace, get("path")) {
            Ok(p) => {
                if let Some(parent) = p.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match std::fs::write(&p, get("content")) {
                    Ok(()) => format!("wrote {}", get("path")),
                    Err(e) => format!("error: write_file `{}`: {e}", get("path")),
                }
            }
            Err(e) => format!("error: {e}"),
        },
        "edit_file" => edit_file(get("path"), get("old_string"), get("new_string"), workspace),
        other => format!("error: unknown tool `{other}`"),
    }
}

fn run_command(command: &str, workspace: &Path) -> String {
    if command.trim().is_empty() {
        return "error: run_command needs a non-empty command".into();
    }
    match std::process::Command::new("bash")
        .arg("-lc")
        .arg(command)
        .current_dir(workspace)
        .output()
    {
        Ok(out) => {
            let mut s = String::new();
            s.push_str(&String::from_utf8_lossy(&out.stdout));
            s.push_str(&String::from_utf8_lossy(&out.stderr));
            if s.trim().is_empty() {
                s = format!("(exit {})", out.status.code().unwrap_or(-1));
            }
            cap(s)
        }
        Err(e) => format!("error: run_command: {e}"),
    }
}

fn edit_file(path: &str, old: &str, new: &str, workspace: &Path) -> String {
    let p = match confine(workspace, path) {
        Ok(p) => p,
        Err(e) => return format!("error: {e}"),
    };
    let contents = match std::fs::read_to_string(&p) {
        Ok(c) => c,
        Err(e) => return format!("error: edit_file `{path}`: {e}"),
    };
    let count = contents.matches(old).count();
    if count == 0 {
        return format!("error: edit_file `{path}`: old_string not found");
    }
    if count > 1 {
        return format!("error: edit_file `{path}`: old_string is not unique ({count} matches)");
    }
    let edited = contents.replacen(old, new, 1);
    match std::fs::write(&p, edited) {
        Ok(()) => format!("edited {path}"),
        Err(e) => format!("error: edit_file `{path}`: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_are_the_four_primitives() {
        let defs = tool_definitions();
        let names: Vec<&str> = defs
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["function"]["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            ["run_command", "read_file", "write_file", "edit_file"]
        );
    }

    #[test]
    fn is_write_only_for_mutations() {
        assert!(is_write("write_file") && is_write("edit_file"));
        assert!(!is_write("run_command") && !is_write("read_file"));
    }

    #[test]
    fn confine_allows_within_and_refuses_escape() {
        let root = Path::new("/work/space");
        assert_eq!(
            confine(root, "src/a.rs").unwrap(),
            PathBuf::from("/work/space/src/a.rs")
        );
        assert_eq!(
            confine(root, "./a").unwrap(),
            PathBuf::from("/work/space/a")
        );
        assert!(confine(root, "../secret").is_err());
        assert!(confine(root, "/etc/passwd").is_err());
        assert!(confine(root, "a/../../b").is_err());
    }

    #[test]
    fn cap_leaves_small_output_untouched_and_elides_large() {
        assert_eq!(cap("short".into()), "short");
        let big = "x".repeat(MAX_RESULT_CHARS + 5_000);
        let out = cap(big);
        assert!(out.contains("chars elided"));
        assert!(out.chars().count() < MAX_RESULT_CHARS + 200);
    }

    #[test]
    fn unknown_tool_is_a_readable_error_not_a_panic() {
        let call = ToolCall {
            id: "x".into(),
            name: "nope".into(),
            arguments: "{}".into(),
        };
        assert!(dispatch(&call, Path::new("/tmp")).contains("unknown tool"));
    }
}
