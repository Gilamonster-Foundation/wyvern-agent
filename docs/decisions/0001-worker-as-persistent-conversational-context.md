# A Worker Is a Persistent Conversational Context

> **Status:** Proposed
> **Date:** 2026-06-03
> **Repo:** `Gilamonster-Foundation/wyvern-agent`
> **Lineage:** Revised, clean-room, from the drake-era swarm worker design.
> This is the public, crate-decomposed re-statement of that lesson — the
> single-agent projection lives in
> [`newt-agent/docs/decisions/conversation_context_architecture.md`](https://github.com/Gilamonster-Foundation/newt-agent/blob/main/docs/decisions/conversation_context_architecture.md);
> this is its swarm-side counterpart.

## TL;DR

A swarm worker is not a request handler that spins up, answers, and dies. It is
a **persistent conversational context**: a long-lived identity with an
accumulating journal, an address you can send to, and a session you can attach
to. Tearing it down between tasks throws away the most valuable thing it
accumulated — its context.

The cloud-native reflex (stateless, ephemeral, horizontally scaled) is exactly
wrong for agents. Agents are the inverse: **context is the load-bearing
substrate**, and it must persist, accumulate, and stay addressable.

This document states that lesson as a set of **small, reusable crates** rather
than one orchestrator binary.

## 1. The core model

A **context** is the unit of the swarm. Each context has:

- **An identity** — a stable, content-independent name (a `ContextId`), never a
  hostname, pod name, or IP. You address the *conversation*, not the machine it
  happens to run on.
- **A journal** — an append-only, content-addressable record of everything that
  happened in the conversation. The journal *is* the worker; the running process
  is just its current embodiment.
- **An inner agent** — the thing that actually thinks. In this constellation
  that is a local-model [`newt-agent`](https://github.com/Gilamonster-Foundation/newt-agent)
  instance (dogfood; no vendor pilots), hosted by a supervisor that owns
  durability and lifecycle.

### What this model is NOT

- Not a microservice (no statelessness, no request/response as the primary mode).
- Not a job queue (work is *conversation*, not discrete units flung at a pool).
- Not a chat session bolted onto a daemon (the journal is the source of truth,
  not a UI transcript).

## 2. Two surfaces, not one

A context is reachable two ways, and the distinction is load-bearing:

| Surface | Mode | Metaphor | Crate |
|---------|------|----------|-------|
| **Mailbox** | asynchronous | leave a message; it's read when the context next runs | `wyvern-mailbox` |
| **Attach** | synchronous | "tmux for agents" — join the live session, watch and steer | `wyvern-attach` |

Both ride [`agent-mesh`](https://github.com/Gilamonster-Foundation/agent-mesh)
as transport and identity root (cryptographic, broker-less). The mailbox is the
durable default; attach is the interactive escalation. You do not need a
separate message bus and a separate session protocol — they are two access
patterns over one mesh-addressed context.

## 3. Crate decomposition

The whole point of this rewrite is that each idea is a crate that can be
adopted, tested, and replaced independently — not a module of an entrenched
monolith.

- **`wyvern-context`** — the `Context` model: `ContextId`, lifecycle state, and
  the content-addressable **journal** (BLAKE3-addressed entries; integrity is
  intrinsic, not asserted). Shares the journal concept with newt-agent's
  single-agent conversation store.
- **`wyvern-supervisor`** — the Rust process that embodies a context: hosts the
  pluggable inner agent (a newt worker), owns restart/pause/resume, and writes
  the journal durably before acting. The supervisor is mortal; the journal is not.
- **`wyvern-mailbox`** — the asynchronous surface over agent-mesh. Deliver a
  message to a `ContextId`; it is journaled and read when the context runs.
- **`wyvern-attach`** — the synchronous surface: multiplex a live session so a
  human (or another agent) can attach, observe, and steer, then detach without
  ending the context.
- **`wyvern-registry`** — the index of contexts: which exist, their mesh
  addresses, and their lifecycle state. Keyed by `ContextId`, never by host.

Capabilities are enforced throughout by
[`agent-bridle`](https://github.com/Gilamonster-Foundation/agent-bridle):
a context acts only within its attenuated `Caveats`, never with ambient
authority.

## 4. Lifecycle as state transitions

The context moves through explicit states. **State transitions — not
wall-clock time — are the coordination primitive.** Nothing in the swarm waits
"5 minutes"; it waits for *open → warm*, for a journal generation to advance,
for a message to land.

```
open ──▶ warm ──▶ cold ──▶ closed
          ▲ │
   attach │ │ detach        (attach/detach and message do not change
          │ ▼                lifecycle state — they act on a warm context)
        (live session)
```

- **open** — context created; identity + journal allocated, not yet embodied.
- **warm** — supervisor running, inner agent embodied, reachable on both surfaces.
- **cold (pause)** — embodiment torn down to save resources; journal preserved.
  Re-warming replays/loads the journal — the conversation resumes, it does not
  restart.
- **closed** — retired; journal retained as provenance.

Pause/resume is cheap *because* the journal is the worker. You are not
checkpointing a process; you are putting down and picking up a conversation.

## 5. Durability is a precondition, not an afterthought

A context that loses its journal on restart is not persistent — it is an
ephemeral worker wearing a costume. Durability is therefore a property of the
*context*, owned by the supervisor, and established before the inner agent takes
its first action:

- Every consequential step is journaled **before** it is acted on.
- The journal is content-addressable, so any embodiment can verify it hasn't
  been tampered with (the same trust property that makes the data sovereign).
- Recovery is replay, not reconstruction.

## 6. Security model

- **Identity** is a mesh key, in the `UserKey → AgentKey` attenuation chain — not
  a shared secret, not a hostname you trust by convention.
- **Authority** is capability-based via `agent-bridle` `Caveats`. A context can
  do only what its caveats permit; delegation only attenuates, never amplifies.
- **The attach surface authenticates by mesh identity**, not by a raw transport's
  ambient trust. (A transport like SSH may carry attach traffic, but it is never
  the *root* of trust.)
- **Blast radius** is bounded per context: filesystem isolation, per-context
  caveats, and runaway/inactivity floors enforced by the supervisor, not hoped
  for.
- **Self-modification** by the inner agent is constrained by the same caveats
  that bound everything else — a context cannot widen its own authority.

## 7. What this gives the swarm

- Workers accumulate expertise instead of amnesia — a specialized context gets
  *better* at its beat over its lifetime.
- The operator can leave a message and walk away (mailbox) or sit down and pair
  (attach), with no protocol switch.
- Pausing idle contexts reclaims resources without losing their minds.
- Every crate here is reusable on its own: the journal, the supervisor, the two
  surfaces, the registry — each stands alone and composes through agent-mesh.

## 8. Open questions

- **Granularity** — one context per project? per beat? per persona? The journal
  makes splitting/merging contexts a real operation; the right default is
  unsettled.
- **Journal compaction** — an accumulating journal grows without bound; in-context
  pruning (the agent curating its own history) vs external compaction is open.
- **Registry home** — does the registry live as mesh state, or as a distinct
  durable service? Leaning mesh-native to avoid a new single point of failure.
- **Re-warming cost** — replaying a long journal on every resume may be too slow;
  snapshot-plus-tail is the likely answer but unproven here.
