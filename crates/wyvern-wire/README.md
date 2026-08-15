# wyvern-wire

> **Legacy / non-release-path:** these sortie payloads are retained for
> compatibility experiments. They are not Wyvern's remote task, execution, or
> event protocol. See the [R0 inventory](../../docs/RELEASE_INVENTORY.md).

Canonical swarm **payload** types for the wyvern orchestration engine.

These are the real versions of the placeholder types that
[`drake-agent`](https://github.com/Gilamonster-Foundation/drake-agent)
currently carries as a stub `wyvern-wire` crate. The names and shapes
here are kept compatible with that stub so the future swap is mechanical:

| Type | drake stub | wyvern (here) |
|------|-----------|---------------|
| `WireError` | `EmptyDiff` | `EmptyDiff` (+ `EmptyGoal`) |
| `SortieId` | opaque string (`new`/`as_str`) | same, **plus** `content_addressed(goal, round)` (BLAKE3) |
| `SortieDebrief` | `new` rejects empty diff; `sortie/goal/diff/context` | identical |
| `Verdict` | `Pass`/`Fail`/`BingoAbort` (kebab `Display`) | identical |
| `SortieRequest` | — | **new**: content-addressed order to fly a sortie |

## Transport boundary

Payload types only. wyvern invents **no** envelope, signing, or
wire-crypto — every value here is carried inside an `agent-mesh`
`SignedEnvelope`. The mesh owns identity and the airspace; wyvern owns
the cargo. (`agent-mesh-protocol` is intentionally not a dependency in
the scaffold to keep it lean; the transport seam lives in
`wyvern-dispatch`.)

## Charter principles enforced here

- **Patch, not prose** — a debrief carries the *diff*.
- **An empty diff is a crash, not a candidate** — `SortieDebrief::new`
  rejects empty/whitespace diffs.
- **Capabilities, not vendor identities** — nothing names a model or
  provider; machine-checked by `tests/no_vendor.rs`.
