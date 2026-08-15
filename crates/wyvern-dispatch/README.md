# wyvern-dispatch

> **Legacy / non-release-path:** the stub dispatch seam below is not the
> authenticated resident transport or execution lifecycle. See the
> [R0 inventory](../../docs/RELEASE_INVENTORY.md).

The **dispatch seam**: how the Dragon-Rider scrambles a sortie to a
worker and collects its debrief.

- `SortieDispatcher` (async trait) — `scramble(&SortieRequest) ->
  Result<SortieDebrief, DispatchError>`. The load-bearing seam.
- `StubDispatcher` — a deterministic, offline stand-in. Synthesizes a
  debrief from the request (same request → same debrief); `::empty()`
  emits an empty diff to exercise the empty-diff-is-a-crash path.

## The real implementation (deferred)

Behind this trait, the real dispatcher:

- reaches workers as **newt-agent instances over `agent-mesh`** — the
  `SortieRequest`/`SortieDebrief` payloads ride inside an `agent-mesh`
  `SignedEnvelope` (wyvern invents no wire-crypto);
- leashes each worker with **`agent-bridle` `Caveats`** — only the
  capabilities the sortie needs, never ambient authority. Pilots are
  selected by the `Caveats` they satisfy, **never** by API provider.

Neither dependency is wired in the scaffold (no network in tests, lean
build) — trait + stub only.
