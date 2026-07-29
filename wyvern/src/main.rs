//! wyvern — the lightest headless worker. Reads an instruction, flies one sortie
//! against a chat-completions endpoint, emits a trace line. No TUI, no config
//! search, no async runtime — just the flight.
//!
//! Usage:
//!   wyvern --endpoint URL --model M --instruction-file F [--cwd DIR]
//!          [--events JSONL] [--max-rounds N] [--context-window N]
//!          [--api-key-env VAR]
//!
//! Exit code: 0 on a clean flight, 1 on an inference/infra failure. Task
//! pass/fail is the benchmark's job (its own verifier), never wyvern's.

use std::io::Write;
use std::path::PathBuf;
use std::process::exit;

use serde_json::json;
use wyvern_runtime::{drive, Backend};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!("{}", USAGE);
        exit(0);
    }
    let get = |flag: &str| -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
    };

    let Some(endpoint) = get("--endpoint").or_else(|| std::env::var("WYVERN_ENDPOINT").ok()) else {
        die("missing --endpoint (or WYVERN_ENDPOINT)");
    };
    let Some(model) = get("--model").or_else(|| std::env::var("WYVERN_MODEL").ok()) else {
        die("missing --model (or WYVERN_MODEL)");
    };
    let Some(instruction_file) = get("--instruction-file") else {
        die("missing --instruction-file");
    };
    let cwd = get("--cwd")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let events = get("--events").map(PathBuf::from);
    let max_rounds = get("--max-rounds")
        .and_then(|s| s.parse().ok())
        .unwrap_or(40);
    let num_ctx = get("--context-window").and_then(|s| s.parse::<u32>().ok());
    let api_key = get("--api-key-env").and_then(|v| std::env::var(v).ok());

    let instruction = match std::fs::read_to_string(&instruction_file) {
        Ok(s) => s,
        Err(e) => die(&format!(
            "reading --instruction-file {instruction_file}: {e}"
        )),
    };
    let workspace = cwd.canonicalize().unwrap_or(cwd);

    let backend = Backend {
        endpoint,
        model: model.clone(),
        api_key,
    };
    let started = std::time::Instant::now();
    let outcome = drive(
        &backend,
        instruction.trim(),
        &workspace,
        max_rounds,
        num_ctx,
    );
    let wall = started.elapsed().as_secs_f64();

    let record = json!({
        "kind": "solve_result",
        "agent": "wyvern",
        "task_file": instruction_file,
        "cwd": workspace.to_string_lossy(),
        "model": model,
        "endpoint": backend.endpoint,
        "status": if outcome.error.is_none() { "completed" } else { "failed" },
        "reply_chars": outcome.reply.len(),
        "rounds": outcome.rounds,
        "tool_calls": outcome.tool_calls,
        "write_calls": outcome.write_calls,
        "usage_total_tokens": outcome.total_tokens,
        "wall_secs": wall,
        "trajectory": outcome.trace,
        "error": outcome.error,
    });

    if let Some(path) = &events {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(f, "{record}");
        }
    }
    println!("{record}");
    exit(if outcome.error.is_none() { 0 } else { 1 });
}

fn die(msg: &str) -> ! {
    eprintln!("wyvern: {msg}\n\n{USAGE}");
    exit(2);
}

const USAGE: &str = "\
wyvern --endpoint URL --model M --instruction-file F [--cwd DIR]
       [--events JSONL] [--max-rounds N] [--context-window N] [--api-key-env VAR]";
