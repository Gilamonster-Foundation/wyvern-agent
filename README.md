# wyvern-agent

> A public, clean-room re-statement of the **drake swarm** orchestration
> lessons — decomposed into reusable crates rather than one entrenched binary.

**Status:** bootstrapping (2026-06-03)

## What this is

`drake` proved out an agentic swarm: a desk dispatches sorties to worker models,
captures their patches, and a wing-commander arbiter grades the work. Those
lessons are real, but they live in a private codebase whose central crate
(`drake-foreman`) is too entrenched to evolve cleanly.

**wyvern-agent** is the rewrite: the same hard-won flight-ops lessons, restated
in the open and **decomposed into small, reusable crates** so each idea can be
adopted, tested, and replaced independently.

## Positioning in the Gilamonster constellation

- **Supersedes** the private `drake` swarm (the `drake-arbiter` vendor slots are
  obsolete; there are no vendor pilots).
- **Dispatches** local-model [`newt-agent`](https://github.com/Gilamonster-Foundation/newt-agent)
  instances as workers — dogfood our own agents, no third-party pilots.
- **Rides** [`agent-mesh`](https://github.com/Gilamonster-Foundation/agent-mesh)
  for coordination/airspace (cryptographic, broker-less).
- **Leashes** workers through [`agent-bridle`](https://github.com/Gilamonster-Foundation/agent-bridle)
  — capabilities enforced as `Caveats`, not ambient authority.

## Load-bearing principles

- **Capabilities, not vendor identities.** A pilot is selected by what
  `Caveats` it can satisfy and what its scorecard says, never by what API
  provider it talks to. There are no vendor adapters in any wyvern crate.
  A clean test: `grep -ri "claude\|openai\|anthropic" wyvern-*/src/` must
  return zero matches in non-test code. *(This is the biggest delta from
  drake and the principle a vendor-API regression would silently violate;
  it leads the list deliberately.)*
- **Prefer decomposition into reusable crates.** Each lesson is a crate, not a
  module of a monolith. Drake was already decomposed into 20 crates — the
  rewrite recuts the boundaries (separating phase orchestration, aggregation,
  bare-repo storage, transport, and dispatch policy) rather than just
  splitting a monolith.
- **Patch, not prose.** A worker only edits files; orchestration plumbing is the
  desk's job, and the arbiter grades the *diff*.
- **An empty diff is a crash, not a candidate.** Scrubbed pre-arbiter, counted
  against that model's scorecard.
- **Swarm-internal storage is a bare git repo, not a forge.** Forges are export
  targets the operator picks per-candidate, never collaborators in the bake-off.
- **Workers are persistent actors, not request handlers.** Context is the
  substrate; don't tear down between tasks.
- **Flight-ops vocabulary** (sortie / scramble / splash / RTB / debrief) for new
  surfaces, docstrings, and UI strings.

## The flight tier — the worker (v0.1)

The first thing to land is the **worker itself**: the lightest thing that can fly
a coding sortie. It is the stripped essence of an agentic loop — a
chat-completions client, four tools (`run_command` / `read_file` / `write_file` /
`edit_file`), one flat loop, a trace. No TUI, no config search, no async runtime.

- **`wyvern-runtime`** — the reusable core (`drive()` + the tools + the wire).
- **`wyvern`** — a thin headless binary.

```bash
cargo build --release          # → target/release/wyvern  (~1.8 MB, opt-level=z, LTO, stripped)
wyvern --endpoint http://host:8080 --model M \
       --instruction-file task.md --cwd /app \
       --events run.jsonl --max-rounds 40 --context-window 65536
```

The system prompt bakes in the lessons: **patch, not prose**, and **run the
task's tests before RTB** (declaring done unverified is a failure). No vendor
identity enters any `wyvern-*` crate —
`grep -ri "claude\|openai\|anthropic" wyvern-*/src` returns zero in non-test code.

## Terminal-Bench scoreboard

Same rule as [`newt-agent`](https://github.com/Gilamonster-Foundation/newt-agent):
wyvern is measured on [Terminal-Bench](https://github.com/harbor-framework/terminal-bench)
(`scripts/eval/harbor/wyvern_agent.py`), and the release gate is a **per-model
monotonic ratchet** — a model's score never goes down. Published every release
from `scripts/eval/bench-results.jsonl` via `scripts/eval/bench_scoreboard.py render`.

<!-- BENCH-SCOREBOARD:START -->
_Per-model **champion** scores — the release gate is a monotonic ratchet: a model's score never goes down. Auto-generated; do not edit by hand._

| Model | Family | Score | Passed | Suite | Window | Version | Date |
|-------|--------|-------|--------|-------|--------|---------|------|
| _(no runs recorded yet)_ | | | | | | | |

<!-- BENCH-SCOREBOARD:END -->

## Source lineage

The drake-era design docs that feed this rewrite are catalogued in
[`docs/MIGRATION_BACKLOG.md`](docs/MIGRATION_BACKLOG.md). They are **inputs to be
revised here**, not files to copy verbatim — and each must pass a go-public
scrub before this repo is made public.
