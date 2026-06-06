# ADR-0002 — Crate map

> **Status:** Accepted (2026-06-04)
> **Context:** [`../CHARTER.md`](../CHARTER.md)

## Decision

wyvern-agent is a Cargo workspace of **12 small crates** in two layers, built
incrementally through the Crawl → Walk → Run → Fly gates. Many sharp crates is a
deliberate lightness strategy: a consumer depends only on what it needs, and
each crate is independently testable and replaceable.

### Dispatch / orchestration layer

| Crate | Responsibility | Gate |
|-------|----------------|------|
| `wyvern-wire` | `TaskRequest` / `TaskReply` types + payload codec inside agent-mesh `SignedEnvelope`s | Walk |
| `wyvern-airspace` | bind an agent-mesh `Bus`; route by `Recipient::{Direct, Anycast}`; verify certs | Walk |
| `wyvern-hangar` | launch + supervise `newt worker` processes; match pilots to required `Caveats` | Walk |
| `wyvern-flight` | flight/sortie types; the `Arbiter` trait (debrief seam) | Walk |
| `wyvern-scramble` | dispatch engine: fan out sorties, capture patches, enforce empty-diff-is-a-crash | Run |
| `wyvern-scorecard` | `model_id`-keyed durable stats (dispatches/crashes/splashes/cost) | Run |
| `wyvern-cli` | the headless `wyvern` binary | Run |

### Worker-lifecycle layer (persistence)

| Crate | Responsibility | Gate |
|-------|----------------|------|
| `wyvern-context` | `ContextId` + content-addressable (BLAKE3) journal | Fly |
| `wyvern-supervisor` | embody a context; pause/resume; host the inner `newt worker` | Fly |
| `wyvern-mailbox` | asynchronous surface over agent-mesh `Bus` topics | Fly |
| `wyvern-attach` | synchronous "tmux for agents" surface (headless) | Fly |
| `wyvern-registry` | index of live contexts, keyed by `ContextId` | Fly |

## Constraints

- **Extraction over anticipation within a crate.** A crate may start as one
  module and grow; it does not split further until a real second consumer
  appears. The 12 above are the seams we are confident in; we do not add a 13th
  speculatively.
- **No TUI dependency** anywhere in the workspace (no ratatui/crossterm).
- **No vendor SDKs.** The dependency tree must show only the reuse-contract deps
  (agent-mesh, agent-bridle, gix, serde, tokio) — never a provider client.
- **Causal, not wall-clock.** `valid_for_generation: Scope<u64>` is the ordering
  primitive in dispatch decisions; `Instant::now()` is banned from those paths.

## Reuse contract

Depend on, do not rebuild:

- `agent-mesh-protocol = "0.6"`, `agent-mesh-bus = "0.6"` (published) — identity,
  `Caveats`, envelopes, the `Bus`.
- `agent-bridle-core` `0.1` (git until crates.io) — `Tool` / `Gate` /
  `ToolContext` / `Registry`.
- `newt worker` (dogfood) — the pilot; `newt-eval` / `newt-core` where they fit.
- `gix` / `git2` — bare-repo patch capture.

## Consequences

- The Crawl scaffold creates all 12 members as stubs so `cargo check --workspace`
  is green from day one, but only the gate-appropriate crates gain real code.
- This supersedes the prior loose framing (the 5-crate-only lifecycle note and
  any earlier "each lesson is a crate" wording in the README).
