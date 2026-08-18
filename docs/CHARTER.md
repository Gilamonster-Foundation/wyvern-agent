# wyvern-agent Charter

> A minimalist, **headless** airframe for agent-to-agent coordinated **sorties**.

**Status:** Charter ratified 2026-06-04. **Amended 2026-08-17**: the Roles
table and the newt absorption direction are superseded by
`newt-agent:docs/decisions/agent_line_architecture.md`. See "Supersession
notice" below. Everything else stands.

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

> **SUPERSEDED 2026-08-17.** The dispatcher/worker assignment below is
> inverted: wyvern is the dispatched worker, and newt or gilamonster dispatch
> to it. See the Supersession notice. The *principle* (privilege = context
> scope, never capability) is unchanged.

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

## Security stance — yolo-in-a-box, leash as a seam

Privilege is context scope, not capability (the drake principle). wyvern takes
that literally: a worker's blast radius is bounded by its **environment**, not
by an in-process leash.

- **v0 flies yolo-in-a-box.** Sorties run with NO in-process ocap leash — but
  ONLY inside a **hermetic** sandbox: an ephemeral worktree over the bare repo,
  with zero ambient authority (no mounted secrets or kubeconfig, no network
  egress, no forge credentials). Because workers are patch-not-prose, the worst a
  yolo pilot can do is write a garbage diff to a throwaway worktree — which
  empty-diff-is-a-crash and the arbiter already catch.
- **Hard invariant: `yolo ⇒ hermetic`.** Yolo is permitted *only* when the
  sandbox is provably hermetic. A sortie that needs real authority (network,
  credentials, a non-throwaway filesystem) is **not** eligible for yolo.
- **The leash is a compiled seam, not absent.** The `agent-bridle` `Gate` /
  `ToolContext` plumbing is wired in feature-gated and no-ops in yolo mode. It
  **defaults ON** for any sortie carrying real authority, and flips on fleet-wide
  the moment the brush `CommandInterceptor` hook (track C1) lands — no
  re-architecture.

This keeps wyvern light and able to fly today, unblocked from the external C1
dependency, without abandoning the object-capability thesis: the leash is
relocated to the edge and kept one flag away.

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
- **agent-bridle-core** (`0.1.0`, **published**): `Tool` / `Gate` /
  `ToolContext` / `Registry` — the capability leash around worker tooling. The
  confined *shell* tool (`agent-bridle-tool-shell`) is blocked on the brush
  `CommandInterceptor` upstream hook; wyvern's worker-shell leash rides that same
  unlock chain (knowledge `BRIDLE_NEWT_ROADMAP.md`, track C1). Wired as a
  feature-gated **seam** (see Security stance): no-op under yolo-in-a-box,
  default-on for any sortie with real authority.
- ~~**newt-agent** (dogfood): the worker is `newt worker` (ACP / stdio). Reuse
  `newt-eval` for scorecard plumbing and `newt-core` where it fits.~~
  **SUPERSEDED 2026-08-17.** wyvern must not depend on newt or gilamonster; a
  lower layer never depends on a richer one. The rest of this reuse contract
  (agent-mesh, agent-bridle-core, gix/git2) stands.
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

## Supersession notice (2026-08-17)

Two claims in this charter are superseded by
`newt-agent:docs/decisions/agent_line_architecture.md`
(newt-agent PR #1753, open at the time of writing; the path resolves once it
merges), which is the canonical statement of how the three agents relate. Read that
document first where the two disagree.

| superseded here | replaced by |
|---|---|
| The Roles table below: the Desk (dispatcher) is wyvern and the Pilot (worker) is a `newt worker` process. | The roles invert. wyvern becomes the dispatched worker: a small headless containerized agent, deployed under OpenShell, that newt or gilamonster dispatch OCAP caveats to for headless or scheduled work. |
| "newt is a superset of wyvern by default: as newt grows it builds the wyvern airframe *into itself*." | The capability ordering `wyvern ≤ newt ≤ gilamonster` stands. The direction of absorption does not: wyvern is rewritten as the smaller base and newt is rebuilt on it. |
| Reuse contract: the worker is `newt worker`; reuse `newt-eval` and `newt-core`. | Superseded. wyvern takes no dependency on newt or gilamonster. The rest of the reuse contract stands. |
| Crate map: `wyvern-hangar` launches and supervises `newt worker` processes; `wyvern-scramble` is "the Desk's dispatch engine". | Superseded with the Roles table. Replacements are for a future wyvern ADR to specify. |
| "we do **not** want to slow newt down; we want a *separate*, deliberately light agent." | Partly superseded. wyvern stays light; it is no longer separate, since newt is rebuilt on it. |

Unchanged and still binding: headless-only, patch-not-prose, no vendor code,
`yolo ⇒ hermetic`, capabilities over vendor identity, and the reuse contract.

Accurate as of this amendment, and worth stating plainly because the charter
reads as though it were already true: wyvern currently depends on none of
agent-mesh, agent-bridle, newt or gilamonster. Five crates ship, but only three
of them (`wire`, `flight`, `hangar`) appear in ADR-0002's twelve-crate map;
`wyvern-dispatch` and `wyvern-agent` do not, and ADR-0002's claim that the
scaffold "creates all 12 members as stubs" is not the case. The `agent-bridle`
`Gate` seam described under "Security stance" is not wired. The reuse contract
below names `agent-bridle-core 0.1.0` as published, while every real consumer
in the line is on 0.7.x. Those are gaps to close, not facts to cite.

## Relationship to the Gilamonster agent line

wyvern is the base **chassis** for coordinated sorties — the smallest thing that
flies. It is a floor others build on, not an end state. "Lightweight" means
*sharp and headless*, never *simple*:

- **newt-agent ⊇ wyvern-agent.** The capability ordering stands. ~~as newt grows
  its agentic-coding capability it builds the wyvern airframe *into itself*~~
  **SUPERSEDED 2026-08-17**: the absorption runs the other way. wyvern is
  rewritten as the smaller base and newt is rebuilt on it. Most of wyvern's own implementation is expected to be flown by
  `newt worker` sorties — wyvern building wyvern, dogfood all the way down.
- **gilamonster-agent** is the full monster chassis — larger and more complex
  than hermes-agent — assembled from the published crates of newt + wyvern +
  agent-mesh + agent-bridle.
- **Other agents are specializations** of the monster chassis, trimmed and tuned
  for one job (e.g. monitor-agent).

Because the implementation is meant to be flown by smaller local models, the
work is decomposed into **small, single-goal issues** — ideally one focused Rust
source file per issue — each linked back to this charter.

## The strange loop — our unique contribution

`SOUL.md → system prompt → prompt → action` is just enough to be useful. The
real value appears when we close it into a **strange loop** (a term of art): the
action feeds back into an artifact — the journal, and ultimately the SOUL — and
that altered artifact changes the next action. The agent's record of being edits
the agent. Recursive self-alteration, grounded in preserved context, is the
selfhood we are after.

This is our distinct spin. Most open-source agents are stateless harnesses over
an LLM — they do not accumulate a self. wyvern's persistence layer exists to
*close this loop*: the journal is not merely durability, it is the substrate of
a self that changes over time. Follows directly from the core thesis.

## How we'll know we're on the right track

North star: stand up **k8s wings and flights** of wyvern whose pilots carry
**roles and mission histories**, where the Desk selects a pilot for a sortie
**based on what that pilot has proven good at** — never its provider, never a
static config (tenet 1). When the scorecard (proven competence) and the mission
history (the record of being) together drive selection, the *fleet* is running
the strange loop: histories shape selection, selection shapes outcomes, outcomes
update histories. That is the signal the core thesis has become operational at
fleet scale.

## Decision record

- **ADR-0001** — [worker as persistent conversational context](decisions/0001-worker-as-persistent-conversational-context.md)
  (the persistence lifecycle this charter's Fly layer realizes).
- **ADR-0002** — [crate map](decisions/0002-crate-map.md).
