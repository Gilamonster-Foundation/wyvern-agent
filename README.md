# wyvern-agent

> A minimalist, **headless** airframe for agent-to-agent coordinated **sorties** —
> the drake-swarm lessons, decomposed into small reusable crates on an open
> substrate.

**Status:** chartered (2026-06-04) — see [`docs/CHARTER.md`](docs/CHARTER.md)

## What this is

`drake` proved out an agentic swarm: a desk dispatches sorties to worker models,
captures their patches, and a wing-commander arbiter grades the work. Those
lessons are real, but they live in a private monolith (`drake-foreman`) that is
too entrenched to evolve cleanly. Sibling agent `newt-agent` is thriving but
carries a rich TUI; wyvern is the *separate, deliberately light* agent for one
job — coordinated sorties.

**wyvern-agent** is the rewrite: the same flight-ops lessons, restated in the
open.

## What "lightweight" means here

Many small crates *and* a full persistent-actor lifecycle is not a contradiction
with lightweight — it is what lightweight means here:

- **Light ≠ few crates.** Each crate is small, sharp, single-purpose, and
  independently useful; a consumer depends only on what it needs.
- **Headless.** No TUI, ever — a ratatui dependency is exactly the weight wyvern
  refuses. Wyvern is a CLI + libraries; rich dashboards belong to `newt pilot`.
- **Reuse, don't rebuild.** Stand on `agent-mesh-bus`, `agent-bridle-core`, and
  dogfooded `newt worker` processes; wyvern writes orchestration glue only.
- **No vendor code.** Pilots are chosen by capability + scorecard, never by
  provider; `grep` for provider names in non-test source returns zero.

## Positioning in the Gilamonster constellation

- **Supersedes** the private `drake` swarm (no vendor pilots; the
  `drake-arbiter` vendor slots are obsolete).
- **Dispatches** local-model [`newt-agent`](https://github.com/Gilamonster-Foundation/newt-agent)
  instances as workers — dogfood, no third-party pilots.
- **Rides** [`agent-mesh`](https://github.com/Gilamonster-Foundation/agent-mesh)
  for coordination/airspace (cryptographic, broker-less).
- **Leashes** workers through [`agent-bridle`](https://github.com/Gilamonster-Foundation/agent-bridle)
  — capabilities enforced as `Caveats`, not ambient authority.

## Build gates

The airframe flies early; persistence lands last.

- **Crawl** — charter + crate-map ADR + workspace scaffold; `cargo check` green.
- **Walk** — one `TaskRequest` over agent-mesh → one `newt worker` → one
  `TaskReply` → a single voter grades the patch.
- **Run** — three pilots compete; quorum reaches consensus; scorecard records it.
- **Fly** — the persistent-actor lifecycle (context/supervisor/mailbox/attach/
  registry), multi-round refinement, splash policy.

## Documentation

- [`docs/CHARTER.md`](docs/CHARTER.md) — mission, roles, the seven tenets, the
  12-crate map, the reuse contract, and the build gates.
- [`docs/decisions/`](docs/decisions/) — ADRs (ADR-0001 persistent context,
  ADR-0002 crate map).
- [`docs/MIGRATION_BACKLOG.md`](docs/MIGRATION_BACKLOG.md) — drake-era design
  docs feeding the rewrite (revise-don't-copy; go-public scrub before public).
