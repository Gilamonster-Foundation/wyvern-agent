//! wyvern-runtime — the stripped essence of an agentic loop.
//!
//! This is the flight tier: the smallest thing that can fly a coding sortie
//! against a chat-completions endpoint. Three modules, one flat loop, four tools,
//! no cockpit. A [`Backend`] is a URL + model + token — no vendor identity ever
//! enters this crate ("capabilities, not vendor identities").
//!
//! ```no_run
//! use std::path::Path;
//! use wyvern_runtime::{drive, Backend};
//! let backend = Backend { endpoint: "http://host:8080".into(), model: "m".into(), api_key: None };
//! let outcome = drive(&backend, "fix the failing test", Path::new("/app"), 40, Some(65536));
//! println!("{} writes, error={:?}", outcome.write_calls, outcome.error);
//! ```

mod chat;
mod drive;
mod tools;

pub use chat::{chat, parse_reply, Backend, Reply, ToolCall};
pub use drive::{drive, Outcome, ToolEvent, SYSTEM_PROMPT};
pub use tools::{dispatch, is_write, tool_definitions};
