//! The sortie: submit an instruction, then round-trip chat ↔ tools until the
//! model concludes (a reply with no tool calls) or the round cap is reached.
//! One flat loop — the whole flight tier.

use std::path::Path;

use serde_json::{json, Value};

use crate::chat::{chat, Backend, Reply};
use crate::tools::{dispatch, is_write, tool_definitions};

/// The flight-tier system prompt — the lessons distilled to a few lines. Patch,
/// not prose; verify before RTB.
pub const SYSTEM_PROMPT: &str = "\
You are a headless coding agent working in a fixed workspace directory. Solve the \
task by editing files and running commands with the provided tools — do not just \
describe a solution, produce it. Before you conclude, RUN the task's tests or an \
obvious check (e.g. run the program, run pytest / make test) and fix any failure; \
declaring done on an unverified solution is a failure. Keep replies terse.";

/// One executed tool call, for the trace / failure taxonomy.
#[derive(Clone, Debug, serde::Serialize)]
pub struct ToolEvent {
    pub tool: String,
    pub ok: bool,
}

/// The result of a sortie — the material a trace line and the taxonomy need.
#[derive(Clone, Debug, Default)]
pub struct Outcome {
    /// The model's final free-text reply.
    pub reply: String,
    pub rounds: usize,
    pub tool_calls: usize,
    /// Mutating tool calls (`write_file`/`edit_file`) — zero means it never acted.
    pub write_calls: usize,
    pub trace: Vec<ToolEvent>,
    /// An infra/inference failure that ended the sortie early (`None` == clean).
    pub error: Option<String>,
    pub total_tokens: Option<u64>,
}

/// Fly one sortie to completion (or the round cap). `num_ctx` is passed to the
/// backend for servers that honor it.
pub fn drive(
    backend: &Backend,
    instruction: &str,
    workspace: &Path,
    max_rounds: usize,
    num_ctx: Option<u32>,
) -> Outcome {
    let tools = tool_definitions();
    let mut messages: Vec<Value> = vec![
        json!({ "role": "system", "content": SYSTEM_PROMPT }),
        json!({ "role": "user", "content": instruction }),
    ];
    let mut out = Outcome::default();

    for round in 0..max_rounds {
        out.rounds = round + 1;
        let reply: Reply = match chat(backend, &messages, &tools, num_ctx) {
            Ok(r) => r,
            Err(e) => {
                out.error = Some(e);
                return out;
            }
        };
        if let Some(t) = reply.total_tokens {
            out.total_tokens = Some(t);
        }
        if !reply.wants_tools() {
            // Concluded: the free text is the answer.
            out.reply = reply.content;
            return out;
        }
        // Record the assistant turn (must carry the tool_calls it asked for).
        messages.push(assistant_message(&reply));
        for call in &reply.tool_calls {
            let result = dispatch(call, workspace);
            let ok = !result.starts_with("error:");
            out.tool_calls += 1;
            if is_write(&call.name) && ok {
                out.write_calls += 1;
            }
            out.trace.push(ToolEvent {
                tool: call.name.clone(),
                ok,
            });
            messages.push(json!({
                "role": "tool",
                "tool_call_id": call.id,
                "content": result,
            }));
        }
    }
    out
}

/// Rebuild the assistant message carrying the model's tool calls, for the next
/// request's history.
fn assistant_message(reply: &Reply) -> Value {
    let calls: Vec<Value> = reply
        .tool_calls
        .iter()
        .map(|c| {
            json!({
                "id": c.id,
                "type": "function",
                "function": { "name": c.name, "arguments": c.arguments }
            })
        })
        .collect();
    json!({
        "role": "assistant",
        "content": reply.content,
        "tool_calls": calls,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::{Reply, ToolCall};

    #[test]
    fn assistant_message_carries_the_tool_calls() {
        let reply = Reply {
            content: "working".into(),
            tool_calls: vec![ToolCall {
                id: "c1".into(),
                name: "run_command".into(),
                arguments: "{\"command\":\"ls\"}".into(),
            }],
            total_tokens: None,
        };
        let m = assistant_message(&reply);
        assert_eq!(m["role"], "assistant");
        assert_eq!(m["tool_calls"][0]["function"]["name"], "run_command");
        assert_eq!(m["tool_calls"][0]["id"], "c1");
    }

    #[test]
    fn system_prompt_demands_verification() {
        // The self-verify lesson is baked into the prompt, not bolted on.
        assert!(SYSTEM_PROMPT.contains("RUN the task's tests"));
        assert!(SYSTEM_PROMPT.to_lowercase().contains("unverified"));
    }
}
