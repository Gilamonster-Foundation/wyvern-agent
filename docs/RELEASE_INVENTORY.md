# R0 release inventory and merge train

**Inventory date:** 2026-08-15
**Release status:** blocked before R1; no current artifact is a Wyvern resident
release.

This document records inspected source, not aspiration. A mock, a completed
response replay, or a correct final string does not close any real-process
gate.

## Exact source evidence

| Repository / artifact | Exact inspected evidence | Release-relevant finding |
|---|---|---|
| `wyvern-agent` | `2fb7107d6c8c37e11246a25f19189a88ac989c0f` (`origin/main` and R0 branch base), workspace `0.6.20260608` | Five local crates; no Newt, Agent Bridle, or Agent Mesh dependency. The binary runs an in-memory demo and exits. |
| `newt-agent` | `origin/main` `4de556542abf22a4ef35c0b88e9d7588f5827744`, workspace `0.8.0` | Has reusable in-process session/output types and operator-rooted Mesh identity. Its stable `install-nonce` logic is private to the conversation store and is not a typed resident identity. |
| `agent-bridle` | `origin/main` `5fe291ab38a04913dd5d8bc16134aaceb2f369af`, workspace `0.8.0-rc.2` (not published at inspection) | Bounded shell output observation exists, but the backend-neutral execution handle/events and `RemoteFence` are still RFC work, not public code. |
| published Agent Bridle | `agent-bridle-core =0.8.0-rc.1` and `agent-bridle-tool-shell =0.8.0-rc.1`, crate VCS SHA `d1cb545081336d0f4530f8da8e6cb11bc7552cd3` | Does not provide the required full execution lifecycle/remote handle contract. Do not build a Wyvern-only substitute around it. |
| `agent-mesh` | `origin/main` `d3b2263ac0f932a10bfc209078885d32ec9e35b4`, workspace `0.6.4` | Authenticated envelopes, request/reply, and publish/subscribe exist. The bus `open_session` / `handle_sessions` API is proposed in an ADR but absent from code. |
| published Agent Mesh | `agent-mesh-protocol =0.6.4`, `agent-mesh-bus =0.6.4`, crate VCS SHA `6dfef2c2079e2d46afd4107237287d524875230a` | Canonical key fingerprints and cert chains exist. `AgentKey` is explicitly per-process and non-persistent; `AgentMetadata` has no resident-id/extension claim. One-shot request/reply is not an event stream. |
| upstream `brush` | `origin/main` `69499f0dbff11ae24200b785f95e03c0ee965a36`, `brush-core 0.5.0` | Shell semantics only; it is not an execution backend. |
| Bridle Brush fork | `brush-ocap-core =0.5.1`, crate VCS SHA `c9000846079ff4178317a695db3c80b647c30eeb` | Command interception exists. Bridle's current Brush adapter emits stdout/stderr from the final worker response, so that path is buffered replay, not live streaming. |

The Mesh `0.6.4` pins above are exact dependency evidence, not permission to
claim a complete resident identity or streaming contract. Any dependency bump
used by a later milestone must record both the exact version and its crate VCS
SHA here.

## Current Wyvern inventory

### What is shared with Newt

Nothing is linked today. `cargo tree --workspace` contains only the five Wyvern
crates plus `async-trait`, `blake3`, and `tokio`. There is no compiled reuse of
`newt-core`, `newt-identity`, Agent Bridle, or Agent Mesh.

The current tree does not contain a substantial copied Newt runtime, but it does
contain an obsolete parallel orchestration model. That code must remain
isolated rather than becoming the resident API by inertia.

### Unmerged worker branch

[PR #43](https://github.com/Gilamonster-Foundation/wyvern-agent/pull/43),
head `135305715b983d70a6231bdce4e767c6869d08c2` at inspection, is also not the
resident implementation. It carries a separate chat/drive/tool stack and code
derived from Newt, invokes `bash -lc` directly with `Command::output`, and
records only the completed solve result. It has no Agent Bridle or Agent Mesh
dependency, no authority/provenance chain, no execution handle, and no
pre-completion output. GitHub reported the PR as conflicting with main.

Useful tests or adapters may be extracted into focused PRs only after their
owning shared seams exist. Merging this branch wholesale would duplicate Newt
runtime code and make buffered, Bridle-bypassing execution the resident
foundation.

### Legacy code retained outside the release path

| Crate / component | What it currently does | Why it is not release evidence |
|---|---|---|
| `wyvern-wire` | In-process `SortieRequest`, `SortieDebrief`, and verdict types | No serialization, authenticated transport, lifecycle events, control handle, or authority/provenance binding. It must not become the remote execution protocol. |
| `wyvern-flight` | Pure phase/retry/grading state machine | Orchestration is outside the resident product boundary. |
| `wyvern-dispatch` | Async trait plus `StubDispatcher` that synthesizes a diff | It starts no worker or process and uses no Mesh or Bridle path. |
| `wyvern-hangar` | In-memory candidate map | It is unrelated to resident task/session state. |
| `wyvern-agent` library | `DragonRider` combines the legacy stubs | It is an orchestrator experiment, not shared Newt runtime. |
| `wyvern-agent` binary | Runs a two-phase stub flight and exits | It is not resident, has no stable identity/workspace binding, accepts no remote task, and streams nothing. |

Deleting these components is intentionally outside R0. The future composition
root must not invoke them by default, and new resident code must not depend on
their protocol shapes.

## Missing contracts and owners

| Contract gap | Owning repository | Required closure |
|---|---|---|
| Stable resident identity | `newt-agent` shared runtime [#1701](https://github.com/Gilamonster-Foundation/newt-agent/issues/1701) | A small typed opaque `ResidentId` with atomic load/create, race convergence, corruption refusal, explicit storage root, and stable identity across process restarts. Newt's private store nonce is useful prior art but not a public contract and must not be copied into Wyvern. |
| Resident claim on process identity | `agent-mesh` protocol [#85](https://github.com/Gilamonster-Foundation/agent-mesh/issues/85) | Each newly issued ephemeral `AgentKey` carries the stable resident id as an explicit signed metadata claim. Preserve key rotation; do not persist `AgentKey` secret bytes or substitute the operator `UserKey` fingerprint. Define old/new cert verification compatibility. |
| Minimal resident bootstrap | `wyvern-agent` | Headless start, explicit canonical workspace, exactly one readiness JSON record, stable resident identity, and controlled lifetime/EOF. No task protocol or execution in this step. |
| Execution lifecycle and control | `agent-bridle` [#370](https://github.com/Gilamonster-Foundation/agent-bridle/issues/370) | Backend-neutral request, event stream, and handle with `cancel` / `kill` / `wait`; ordered stdout/stderr; one terminal outcome; explicit drop/finalization; bounded buffering. Implement local first. |
| Incremental Brush worker protocol | Bridle's Brush fork / upstream `brush`, then `agent-bridle` | Framed stdout/stderr events emitted while the child runs. A final response may summarize but cannot be the source of live output. |
| Authenticated full-duplex session transport | `agent-mesh` bus [#84](https://github.com/Gilamonster-Foundation/agent-mesh/issues/84) | Implement and test the proposed long-lived session primitive with ordered signed messages, bounded flow control, clean vs failed close, and no polling fiction. |
| Shared headless task/operator seam | `newt-agent` [#1702](https://github.com/Gilamonster-Foundation/newt-agent/issues/1702) | Submit one structured task under explicit authority, drive it through the shared Bridle handle, and consume live lifecycle/terminal evidence. Reuse shared runtime/event types rather than adding a Wyvern-specific driver or operator client. |
| Remote execution / OpenShell | `agent-bridle` (`RemoteFence`), with OpenShell as a leaf implementation | Land only after the shared lifecycle and provenance types. OpenShell supplies enforcement evidence, never authority, and is not required by the first release. |

The complete release is tracked in Wyvern
[#51](https://github.com/Gilamonster-Foundation/wyvern-agent/issues/51).
Wyvern remains blocked on the linked owning contracts rather than embedding
local copies.

## Dependency-ordered merge train

Each row is a milestone report: blocker, owner, invariant, proof, and next
smallest mergeable step.

| Milestone / current blocker | Owner | Exact invariant being closed | Evidence that closes it | Next smallest mergeable step |
|---|---|---|---|---|
| **R0 — scope is false in current docs/code** | `wyvern-agent` | The release target is the resident/headless runtime; legacy orchestration is explicitly non-release-path. | Product charter and this source-pinned inventory; existing build gate green. | Merge the documentation/CI correction without runtime claims. |
| **R1 — no stable resident contract** | `newt-agent` shared runtime -> `agent-mesh` -> `wyvern-agent` | Stable resident identity is distinct from the operator root and from rotating per-process `AgentKey`s; workspace binding is explicit and canonical. | Unit tests for atomic identity persistence/corruption; cert verification test for signed resident claim; real binary BAT proves same resident id over two starts, one readiness JSON line, canonical workspace, alive-before-EOF, clean EOF exit. | First land the tiny shared `ResidentId` API in Newt-owned code; no Wyvern crypto or persistence format. |
| **R2 — Bridle has observation, not a complete execution contract** | `agent-bridle` | One backend-neutral lifecycle owns request, events, terminal result, cancellation, kill, wait, ordering, finalization, and bounded buffering. | Local real-child tests for `Accepted -> Started -> output* -> exactly one terminal`; denial/failure cases; cancellation reaches actual child; stalled consumer stays within a measured cap. | Land the type/trait seam and local adapter with zero `RemoteFence` or Wyvern dependency. |
| **R3 — no authenticated end-to-end live stream** | `agent-mesh`, then `newt-agent` and `wyvern-agent`; Brush subtrack owned upstream/Bridle | An authenticated long-lived channel carries Bridle events in order; `line 1` reaches the operator before child completion. Brush is never advertised while it still replays a final response. | Real operator/resident process test with delayed stdout; separate stderr and terminal-order tests; packet/session close tests. | Implement Mesh bus sessions and prove them independently before wiring one Wyvern task. Safe-subset semantics only for the initial slice. |
| **R4 — cancellation/disconnect ownership undefined** | `agent-bridle` + `agent-mesh` + `wyvern-agent` | No orphan-by-accident: operator disconnect, resident death, backend death, handle drop, cancel/kill races, and stalled consumers each have a documented owner and terminal behavior. | Real PID/process-group cancellation proof; forced operator disconnect; killed resident/backend; bounded-stall test; reconnect behavior matches the declared policy. | Specify a small disconnect matrix, then add one real-process case per row. |
| **R5 — result attribution is incomplete** | `agent-mesh` + `agent-bridle`, consumed by `newt-agent`/`wyvern-agent` | `request identity -> delegated authority -> resident id -> process key -> backend -> result evidence` is verifiable; unknown or unsupported claims refuse. | Tamper, wrong-resident, widened-grant, stale-generation, backend-mismatch, and missing-evidence tests; accepted result exposes exact evidence IDs. | Land signed resident claim before adding result binding; do not infer provenance from transport or hostname. |
| **R6 — no reproducible artifact** | `wyvern-agent` release lead | The narrow supported matrix works from a clean checkout with exact cross-repo pins and no mock substitutions. | Clean build; unit/integration suites; resident/client task; pre-completion stdout; stderr; cancel; denial/failure; bounded output; authority audit; documented unsupported combinations. | Cut a release candidate only after the complete scripted proof is rerunnable. |

Strict dependency order for the initial path is:

```text
Newt shared ResidentId
  -> Mesh signed resident claim
  -> Wyvern bootstrap
  -> Bridle execution lifecycle + local backend
  -> Mesh full-duplex sessions
  -> Newt operator + Wyvern one-task integration
  -> cancellation/provenance hardening
  -> release candidate
```

The Brush incremental-frame work can proceed after the Bridle lifecycle shape
is stable, but it does not block a release that explicitly supports only
safe-subset semantics. `RemoteFence`/OpenShell follows the same lifecycle later
and does not block local execution.

## Execution combination matrix

There are no supported production combinations in the current R0 tree.
The intended first-release support statement is narrow:

| Shell semantics | Backend | Transport | First-release status |
|---|---|---|---|
| safe-subset | Bridle local | Agent Mesh session | target release slice; blocked on R1-R5 |
| safe-subset | Bridle local | local/in-process | test harness only; not the remote release proof |
| Brush | Bridle local | Agent Mesh session | unsupported until framed incremental worker events pass the same gates |
| host shell | Bridle local | Agent Mesh session | outside first release; no implicit fallback from safe-subset |
| any | Bridle `RemoteFence` / OpenShell | any | unsupported until Bridle lands the backend, authority projection, provenance, and lifecycle contract |

Unsupported combinations fail explicitly; they never silently switch shell
semantics, backend, transport, or authority.

## Release acceptance script requirements

The eventual clean-environment script must record exact binary versions and git
SHAs, then prove all of the following without mocks:

1. launch a resident against an explicit workspace and observe one readiness
   JSON record;
2. restart it and verify stable resident identity with a fresh process
   `AgentKey` bound to that resident;
3. send one authenticated task from a real Newt/operator client;
4. observe `Accepted` and `Started`;
5. observe delayed `line 1` stdout before process completion, then `line 2`;
6. independently observe stderr and ordered terminal status;
7. cancel a second execution and prove the actual child/process group is gone;
8. provoke a Bridle denial and an execution failure and preserve their distinct
   evidence/error outcomes;
9. stall the event consumer and measure the documented buffer/backpressure
   bound; and
10. print the full identity/authority/backend/result evidence chain.

Until that script passes from a clean checkout, Wyvern is not release-ready.
