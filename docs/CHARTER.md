# wyvern-agent Charter

> A minimalist, **headless** airframe for agent-to-agent coordinated **sorties**.

**Status:** Charter ratified 2026-06-04

## Core thesis — identity is preserved context (non-negotiable)

Won through this work since October 2025: **an identity is a context and a
record of decisions and perceptions.** That record of *being* is what makes an
agent more than a bare LLM or a bare harness — it is continuity, and the
continuity is itself the value.

Therefore **preservation of context is not negotiable.** It is a load-bearing
component of the design, not a feature added later. wyvern's persistence layer
(ADR-0001) exists *because* of this thesis: a context's journal **is** the
worker. The Fly gate is a build-order decision about *when* durability lands —
never a statement that context is lower priority. A sortie that cannot preserve
its record of being is not done.

## Why wyvern exists

`drake` proved out an agentic swarm: a desk dispatches sorties to worker models,
captures their patches, and a wing-commander arbiter grades the work. The
lessons are real, but they live in a private monolith (`drake-foreman`) that is
too entrenched to evolve cleanly.

`newt-agent` is a sibling agent that is thriving — but its weight is
concentrated in a rich UX layer (a ratatui TUI). We do **not** want to slow newt
down; we want a *separate*, deliberately light agent for one job: **coordinated
sorties**. That is wyvern.

## What "lightweight" means here

Wyvern is decomposed into many small crates *and* carries a full persistent-actor
lifecycle. That is not a contradiction with "lightweight" — it is what
lightweight means here:

- **Light ≠ few crates.** Light = each crate is small, sharp, single-purpose,
  and independently useful. Fine-grained crates mean a consumer depends only on
  what it needs — the opposite of a monolith.
- **Headless.** No TUI, ever. A ratatui dependency is exactly the weight wyvern
  refuses. Wyvern is a CLI + libraries; observability is structured logs /
  ndjson. Any rich dashboard belongs to `newt pilot`, not wyvern.
- **Reuse, don't rebuild.** Wyvern's own line count stays thin by standing on
  published substrate — `agent-mesh-bus`, `agent-bridle-core`, and dogfooded
  `newt worker` processes. Wyvern writes orchestration glue, not transport,
  crypto, capability enforcement, or inference.
- **No vendor code.** `grep -ri "claude\|openai\|anthropic" wyvern-*/src/`
  (non-test) returns zero. A pilot is chosen by capability + scorecard, never by
  API provider.

## Mission

Dispatch coordinated sorties — one worker × one round — to local-model agents,
capture their work as **patches**, grade the patches, and return winners to the
operator. The hard-won drake lessons, decomposed into small reusable crates on
an open substrate.

## Roles (privilege = context scope, never capability)

| Role | Flight-ops term | Scope | Limit |
|------|-----------------|-------|-------|
| Dispatcher | **The Desk** | current flight + rung state | no final authority |
| Worker (a `newt worker`) | **Pilot / Bird** | one clean worktree | no git, no push, no memory, no external APIs |
| Grader | **Wing Commander / Debrief** | the patch (never prose) | no merge to main |
| Human | **Operator** | full system + intent | irreducible authority; decides RTB |

Each layer can only promote work *upward* (draft → graded → operator), never
inject trust *downward*.

## Seven load-bearing tenets

1. **Capabilities, not vendor identities.** A pilot is selected by `Caveats`
   match + scorecard, never by provider.
2. **Patch, not prose.** Workers only edit files; the Desk captures
   `git diff base..work`; the arbiter grades the diff. Prose is never input to
   judgment.
3. **Empty diff is a crash, not a candidate.** Disqualified pre-arbiter and
   counted against that pilot's scorecard (`EmptyDiff`, distinct from errors).
   The arbiter never grades vapor.
4. **Bare repo, not forge.** Swarm-internal storage is a bare git repo on shared
   storage (`refs/heads/wyvern/rung/<round>/<pilot>`). Forges are
   operator-chosen export targets, post-review — never in the bake-off critical
   path.
5. **No wall-clock in decisions.** Use `Caveats.valid_for_generation:
   Scope<u64>` as the causal primitive. No `Instant::now()` in dispatch-decision
   or public-API paths. Time is observed, never compared.
6. **Workers are persistent actors; context preservation is non-negotiable.**
   Per the core thesis, a context's journal *is* the worker — its identity is
   its record of decisions and perceptions. Preserving that continuity is
   load-bearing, not optional. (See ADR-0001; durability lands at the Fly gate,
   but is never treated as lower priority.)
7. **Flight-ops vocabulary** in docstrings, logs, and UI strings: sortie,
   scramble, splash, RTB, debrief, wing-commander, bingo, scrubbed, hull, bird.

## Crate map (12, two layers)

The target decomposition. Built incrementally through the gates below — not all
at once. See [`docs/decisions/0002-crate-map.md`](decisions/0002-crate-map.md).

**Dispatch / orchestration**

- `wyvern-wire` — `TaskRequest` / `TaskReply` types + payload codec, carried
  inside agent-mesh `SignedEnvelope`s.
- `wyvern-airspace` — agent-mesh integration: bind a `Bus`, route by
  `Recipient::{Direct, Anycast{capability}}`, verify certs.
- `wyvern-hangar` — pilot/aircraft management: launch + supervise `newt worker`
  processes, match pilots to a sortie's required `Caveats`.
- `wyvern-flight` — a flight = one dispatch cycle; sortie types; the `Arbiter`
  trait (debrief seam; the concrete arbiter is a `newt` instance).
- `wyvern-scramble` — the Desk's dispatch engine: fan out sorties, capture
  patches from the bare repo, enforce empty-diff-is-a-crash.
- `wyvern-scorecard` — `model_id`-keyed durable stats
  (dispatches / crashes / splashes / cost).
- `wyvern-cli` — the `wyvern` binary (scramble, debrief, status). Headless.

**Worker lifecycle (persistence — the Fly layer)**

- `wyvern-context` — `ContextId` + content-addressable (BLAKE3) journal.
- `wyvern-supervisor` — embodies a context; pause/resume; hosts the inner
  `newt worker`; journals before acting.
- `wyvern-mailbox` — asynchronous surface over agent-mesh (`Bus` topics).
- `wyvern-attach` — synchronous "tmux for agents" surface (headless multiplex).
- `wyvern-registry` — index of live contexts, keyed by `ContextId` (never host).

## Reuse contract (depend on, do NOT rebuild)

- **agent-mesh** (`"0.6"`, published): `Bus` (request / handle_requests /
  publish / subscribe), `SignedEnvelope`, `Recipient`, `UserKey` / `AgentKey` /
  `CertChain` / `AgentMetadata`, `Caveats` / `Scope` / `CountBound`.
- **agent-bridle-core** (`0.1`, git until crates.io): `Tool` / `Gate` /
  `ToolContext` / `Registry` — the capability leash around worker tooling.
- **newt-agent** (dogfood): the worker is `newt worker` (ACP / stdio). Reuse
  `newt-eval` for scorecard plumbing and `newt-core` where it fits.
- **gix / git2**: bare-repo patch capture.

## Build gates (definition of done = the Fly gate)

The airframe flies early; persistence lands last.

- **Crawl** — charter + crate-map ADR + workspace scaffold; `cargo check` green.
- **Walk** — wire/airspace/hangar/flight scaffolds + `Arbiter` trait; one
  `TaskRequest` over agent-mesh → one `newt worker` → one `TaskReply` → a single
  voter grades the patch.
- **Run** — scramble/scorecard/cli + multi-voter quorum; three pilots compete,
  quorum reaches consensus, scorecard records it; empty-diff scrubbed; bare repo
  holds all rungs.
- **Fly** — the lifecycle layer (context/supervisor/mailbox/attach/registry),
  multi-round refinement, splash policy, durable scorecard.

## Relationship to the Gilamonster agent line

wyvern is the base **chassis** for coordinated sorties — the smallest thing that
flies. It is a floor others build on, not an end state. "Lightweight" means
*sharp and headless*, never *simple*:

- **newt-agent ⊇ wyvern-agent.** newt is a *superset* of wyvern by default: as
  newt grows its agentic-coding capability it builds the wyvern airframe *into
  itself*. Most of wyvern's own implementation is expected to be flown by
  `newt worker` sorties — wyvern building wyvern, dogfood all the way down.
- **gilamonster-agent** is the full monster chassis — larger and more complex
  than hermes-agent — assembled from the published crates of newt + wyvern +
  agent-mesh + agent-bridle.
- **Other agents are specializations** of the monster chassis, trimmed and tuned
  for one job (e.g. monitor-agent).

Because the implementation is meant to be flown by smaller local models, the
work is decomposed into **small, single-goal issues** — ideally one focused Rust
source file per issue — each linked back to this charter.

## Decision record

- **ADR-0001** — [worker as persistent conversational context](decisions/0001-worker-as-persistent-conversational-context.md)
  (the persistence lifecycle this charter's Fly layer realizes).
- **ADR-0002** — [crate map](decisions/0002-crate-map.md).
