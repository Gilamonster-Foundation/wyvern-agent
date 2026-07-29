//! The chat-completions HTTP wire — the de-facto `/v1/chat/completions` contract
//! that local inference servers speak (llama.cpp, vLLM, ollama's compat shim).
//!
//! No vendor identity, by principle: a [`Backend`] is a URL + a model name +
//! (optionally) a bearer token. Nothing here knows or cares which family the
//! model belongs to — a pilot is selected by capability, never by provider.

use serde::Deserialize;
use serde_json::{json, Value};

/// Where and what to fly: a base endpoint, a served model, an optional token.
#[derive(Clone, Debug)]
pub struct Backend {
    /// Base URL, e.g. `http://host:8080` (the `/v1/chat/completions` path is
    /// appended). Trailing slashes are tolerated.
    pub endpoint: String,
    /// The served model name the endpoint expects.
    pub model: String,
    /// Optional bearer token for authenticated endpoints.
    pub api_key: Option<String>,
}

/// One tool the model asked to call: an id, a name, and its raw JSON arguments
/// (kept as the wire string so the dispatcher parses exactly what was sent).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// The assistant's turn: free text and/or tool calls.
#[derive(Clone, Debug, Default)]
pub struct Reply {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    /// Total tokens the server reported for this request, if any.
    pub total_tokens: Option<u64>,
}

impl Reply {
    pub fn wants_tools(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

// ── wire response shapes (only the fields we read) ──────────────────────────
#[derive(Deserialize)]
struct WireResponse {
    #[serde(default)]
    choices: Vec<WireChoice>,
    #[serde(default)]
    usage: Option<WireUsage>,
}
#[derive(Deserialize)]
struct WireChoice {
    message: WireMessage,
}
#[derive(Deserialize, Default)]
struct WireMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<WireToolCall>,
}
#[derive(Deserialize)]
struct WireToolCall {
    #[serde(default)]
    id: String,
    function: WireFunction,
}
#[derive(Deserialize)]
struct WireFunction {
    #[serde(default)]
    name: String,
    #[serde(default)]
    arguments: String,
}
#[derive(Deserialize)]
struct WireUsage {
    #[serde(default)]
    total_tokens: Option<u64>,
}

/// Parse a `/v1/chat/completions` response body into a [`Reply`]. Split out from
/// the HTTP call so it is pure and unit-tested against captured wire bodies.
pub fn parse_reply(body: &str) -> Result<Reply, String> {
    let wire: WireResponse =
        serde_json::from_str(body).map_err(|e| format!("bad chat response JSON: {e}"))?;
    let msg = wire
        .choices
        .into_iter()
        .next()
        .map(|c| c.message)
        .unwrap_or_default();
    Ok(Reply {
        content: msg.content.unwrap_or_default(),
        tool_calls: msg
            .tool_calls
            .into_iter()
            .map(|tc| ToolCall {
                id: tc.id,
                name: tc.function.name,
                arguments: tc.function.arguments,
            })
            .collect(),
        total_tokens: wire.usage.and_then(|u| u.total_tokens),
    })
}

/// One blocking request to the backend. `messages` and `tools` are the wire
/// arrays the caller assembled; `num_ctx` (when set) is passed through for
/// servers that honor it. Errors carry the status + body so overflow/limit
/// conditions are legible to the caller.
pub fn chat(
    backend: &Backend,
    messages: &[Value],
    tools: &Value,
    num_ctx: Option<u32>,
) -> Result<Reply, String> {
    let url = format!(
        "{}/v1/chat/completions",
        backend.endpoint.trim_end_matches('/')
    );
    let mut body = json!({
        "model": backend.model,
        "messages": messages,
        "tools": tools,
        "stream": false,
    });
    if let Some(n) = num_ctx {
        body["num_ctx"] = json!(n);
    }
    let mut req = ureq::post(&url).set("content-type", "application/json");
    if let Some(key) = &backend.api_key {
        req = req.set("authorization", &format!("Bearer {key}"));
    }
    match req.send_json(body) {
        Ok(resp) => {
            let text = resp
                .into_string()
                .map_err(|e| format!("reading chat response: {e}"))?;
            parse_reply(&text)
        }
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            Err(format!("inference endpoint {code}: {text}"))
        }
        Err(e) => Err(format!("request failed: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_text_reply() {
        let body = r#"{"choices":[{"message":{"content":"hello"}}],
                       "usage":{"total_tokens":42}}"#;
        let r = parse_reply(body).unwrap();
        assert_eq!(r.content, "hello");
        assert!(!r.wants_tools());
        assert_eq!(r.total_tokens, Some(42));
    }

    #[test]
    fn parses_tool_calls() {
        let body = r#"{"choices":[{"message":{"content":null,"tool_calls":[
            {"id":"c1","function":{"name":"run_command","arguments":"{\"command\":\"ls\"}"}}
        ]}}]}"#;
        let r = parse_reply(body).unwrap();
        assert!(r.wants_tools());
        assert_eq!(r.tool_calls[0].name, "run_command");
        assert_eq!(r.tool_calls[0].arguments, "{\"command\":\"ls\"}");
        assert_eq!(r.tool_calls[0].id, "c1");
    }

    #[test]
    fn empty_choices_is_an_empty_reply_not_an_error() {
        let r = parse_reply(r#"{"choices":[]}"#).unwrap();
        assert_eq!(r.content, "");
        assert!(!r.wants_tools());
    }

    #[test]
    fn malformed_json_errors() {
        assert!(parse_reply("not json").is_err());
    }
}
