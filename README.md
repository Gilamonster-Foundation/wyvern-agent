# wyvern-agent

> A lightweight resident agent that lives on a workstation, server, VM, or
> Kubernetes node/pod; receives delegated work; executes through Agent Bridle;
> streams lifecycle events to an operator; and retains enough agent and
> workspace state to do useful autonomous work.

**Status:** R0 architecture correction ([release tracker #51]). This repository
does not yet contain a releasable resident runtime. The current binary is a
legacy, stub-backed flight demo and is not evidence of task dispatch, Bridle
execution, or streaming. See the
[release inventory and merge train](docs/RELEASE_INVENTORY.md).

## Product boundary

Wyvern is the small, headless resident in the Newt agent line. Newt remains the
full operator-facing runtime; Wyvern is the remote execution endpoint that can
stay alive near a workspace and accept narrowly delegated work.

Wyvern is not a second Newt implementation, a TUI or web UI, an orchestrator or
control plane, an OpenShell replacement, or an Agent Bridle replacement. It
does not pursue Newt feature parity. Shared runtime behavior belongs in a small
Newt-owned crate rather than being copied here.

Four concerns stay independent:

| Concern | Initial release choice | Other choices |
|---|---|---|
| Agent runtime | Wyvern resident | Newt operator runtime |
| Shell semantics | safe-subset | Brush, host shell, future engines |
| Execution backend | Bridle local | Bridle `RemoteFence`; OpenShell may implement one |
| Transport / dispatch | Agent Mesh | local test transport; future transports |

Brush and OpenShell are not alternatives on one enum. Brush determines shell
semantics. OpenShell is an optional remote execution/enforcement backend.
Wyvern remains useful without OpenShell.

## First release proof

The first supported path is deliberately narrow:

```text
Newt/operator
    -> authenticated remote task over Agent Mesh
    -> long-lived Wyvern resident
    -> Agent Bridle execution (local backend, safe-subset semantics)
    -> ordered live lifecycle events
    -> operator
```

The proof must run a real process that prints `line 1`, waits long enough to
show temporal separation, prints `line 2`, and exits. The operator must observe
`line 1` before process completion. Replaying a completed response, polling a
one-shot Mesh request/reply endpoint, or returning the right final text does
not count as streaming.

## Security floor

Every execution goes through Agent Bridle. There is no yolo mode, hermetic or
otherwise, that bypasses the Bridle authority model for a release path.
Unsupported authority or evidence fails closed; remote execution never inherits
trust merely from local kernel identity; and OpenShell is never an independent
source of authority.

The target attribution chain is explicit:

```text
request identity
  -> delegated authority
  -> stable Wyvern resident identity
  -> per-process Mesh identity
  -> execution backend
  -> execution/result evidence
```

Agent Mesh process `AgentKey`s remain ephemeral. A stable resident identifier
and its signed metadata binding are shared-runtime gaps, not a reason to persist
an ephemeral key or relabel the operator's `UserKey` as the resident.

## Repository state

`wyvern-wire`, `wyvern-flight`, `wyvern-dispatch`, `wyvern-hangar`, and the
current `DragonRider` library are retained as legacy experimentation so this
correction does not become a rewrite. They are outside the resident release
path. In particular, `SortieRequest` / `SortieDebrief` are not the remote task
or execution protocol, and `StubDispatcher` / `InMemoryHangar` are not release
backends.

## Build gate

```bash
just check
```

This runs formatting, zero-warning clippy, the workspace tests, and the
vendor-identity source guard. CI and the pre-push hook mirror the same gate.

The full clean-environment and real-process release gates are listed in
[`docs/RELEASE_INVENTORY.md`](docs/RELEASE_INVENTORY.md).

[release tracker #51]: https://github.com/Gilamonster-Foundation/wyvern-agent/issues/51
