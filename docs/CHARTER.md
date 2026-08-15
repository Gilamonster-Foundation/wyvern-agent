# wyvern-agent charter

> A lightweight resident/headless Newt runtime for delegated work.

**Status:** Corrected at R0 on 2026-08-15. This charter supersedes the
2026-06-04 coordinated-sortie/orchestrator charter. It defines the release
boundary; it does not claim that the current stub binary satisfies it.

## Mission

Wyvern lives close to a workspace on a workstation, server, VM, Kubernetes
node/pod, or other remote host. It accepts authenticated delegated work,
executes only through Agent Bridle, streams ordered lifecycle events back to an
operator, and keeps enough identity, workspace, task/session, authority, model,
transport, execution-handle, and evidence state to remain useful between tasks.

Wyvern is the resident half of this relationship:

```text
newt-core / shared runtime
          ^
          |
     +----+----+
     |         |
   Newt      Wyvern
 operator   resident
 runtime    headless
```

Newt owns the rich operator experience and shared agent runtime abstractions.
Wyvern consumes those abstractions. If a change would substantially duplicate
Newt logic, work stops until the shared-library boundary is agreed and landed.

## Scope

Wyvern does four things:

1. starts as a long-lived, headless resident with a stable resident identity;
2. binds explicitly to a canonical project/workspace;
3. receives delegated tasks with request identity and authority intact; and
4. drives Agent Bridle execution while relaying live lifecycle events and
   terminal evidence to the controlling operator.

The first release proves one real task through that path. It is not a general
fleet manager.

### Non-goals

Wyvern is not:

- a second implementation of Newt;
- a TUI, web UI, or mobile UI;
- an OpenShell or Agent Bridle replacement;
- an orchestration/control-plane product;
- a plugin ecosystem, elaborate discovery system, or speculative transport
  laboratory;
- a place to chase Newt parity; or
- a dumping ground for the legacy flight/sortie experiment.

The legacy `wyvern-wire`, `wyvern-flight`, `wyvern-dispatch`,
`wyvern-hangar`, and `DragonRider` code remains in-tree for now to keep R0
correction-forward. It is not on the release path and must not define the new
task, execution, identity, or transport contracts.

## Orthogonal architecture

The following axes must not collapse into one another:

| Axis | Choices |
|---|---|
| Agent runtime | Newt operator-facing runtime; Wyvern resident/headless runtime |
| Shell semantics | safe-subset; Brush; host shell; future engines |
| Execution backend | local; Agent Bridle `RemoteFence`; future backends |
| Transport / dispatch | local test path; Agent Mesh; future transports |

OpenShell may implement a `RemoteFence`; it does not define shell semantics or
authority. Brush defines shell semantics; it is not an execution backend. An
execution chooses one value on each relevant axis.

The smallest release combination is Agent Mesh transport, Wyvern resident,
Agent Bridle local execution, and safe-subset shell semantics. Brush, host
shell, and `RemoteFence`/OpenShell are unsupported until each independently
passes the same lifecycle, streaming, cancellation, and authority gates.

## Shared execution contract

Wyvern must consume a backend-neutral Agent Bridle execution seam. It must not
define a parallel Wyvern-only protocol. The shared lifecycle needs, at minimum,
semantics equivalent to:

```text
Accepted
Started
Stdout(seq, bytes)
Stderr(seq, bytes)
ToolStarted / ToolFinished (when applicable)
Exited(status)
Denied(reason, evidence)
Failed(error)
```

and control in the other direction:

```text
cancel
kill
wait
```

The contract must specify ordering, exactly one terminal outcome,
finalization, cancellation races, handle drop, operator disconnect, resident or
backend death, and bounded buffering/backpressure. A terminal result cannot
overtake earlier output, and a slow observer cannot create unbounded memory.

## True streaming

Buffered replay is not streaming. Release acceptance uses a real child that:

1. emits `line 1`;
2. waits long enough to prove temporal separation;
3. emits `line 2`; and
4. exits.

The remote observer must receive `line 1` before the child completes. The same
test separately proves stderr and ordering through the complete
operator-to-Wyvern-to-Bridle path. Agent Mesh one-shot request/reply polling is
not a substitute for a long-lived authenticated event stream. Brush support is
blocked until its worker protocol emits framed incremental events rather than a
completed-response replay.

## Security invariants

Agent Bridle is the authority and execution boundary. Wyvern must never:

- bypass Bridle, including in a supposedly hermetic or yolo mode;
- widen grants so remote execution succeeds;
- equate remote execution identity with local kernel identity;
- silently downgrade unsupported authority or evidence;
- accept `unknown` as authorization; or
- treat OpenShell policy as an independent grant of authority.

The required provenance chain is:

```text
request identity
  -> delegated authority
  -> stable Wyvern resident identity
  -> ephemeral process AgentKey
  -> execution backend identity
  -> execution/result identity and evidence
```

Agent Mesh `AgentKey`s are intentionally per-process and rotate on restart.
Wyvern will not persist their secret bytes against the Mesh contract. The
stable resident identifier must come from a small shared Newt runtime component
and be carried as a signed Mesh metadata claim on each process key. The current
Newt/Mesh APIs do not yet provide that complete seam; R1 is blocked on it. An
operator `UserKey` fingerprint is not a resident identifier.

Local and remote backends may use different provenance evidence. That
difference remains explicit, and unsupported claims resolve conservatively.

## Resident lifetime

The resident may outlive many tasks and execution sandboxes. Its durable or
restart-recoverable state may include:

- stable resident identity and current ephemeral Mesh process identity;
- canonical project/workspace association;
- task and session state;
- model/provider configuration;
- delegated Bridle authority state;
- transport connection state;
- active execution handles and cancellation policy; and
- result and provenance evidence.

An OpenShell sandbox or other `RemoteFence` may be short-lived or reused under
its own verified policy. It never defines the resident's lifetime.

## Release discipline

Changes land as a small dependency-ordered merge train, one architectural
concern per repository. When a contract belongs in Newt, Agent Bridle, or Agent
Mesh, Wyvern stays blocked rather than copying it. Mocks can prove local type
logic but cannot close a real-process or remote-streaming gate.

No release is ready without a clean checkout build, unit and integration tests,
a real resident/client task, pre-completion stdout, stderr, cancellation,
failure/denial propagation, bounded output behavior, documented execution
combinations, preserved authority, and exact version/SHA evidence for every
cross-repository contract.

The current inventory, blockers, owners, proofs, and next mergeable steps live
in [`RELEASE_INVENTORY.md`](RELEASE_INVENTORY.md).
