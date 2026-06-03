# wyvern-agent

> A public, clean-room re-statement of the **drake swarm** orchestration
> lessons — decomposed into reusable crates rather than one entrenched binary.

**Status:** bootstrapping (2026-06-03)

## What this is

`drake` proved out an agentic swarm: a desk dispatches sorties to worker models,
captures their patches, and a wing-commander arbiter grades the work. Those
lessons are real, but they live in a private codebase whose central crate
(`drake-foreman`) is too entrenched to evolve cleanly.

**wyvern-agent** is the rewrite: the same hard-won flight-ops lessons, restated
in the open and **decomposed into small, reusable crates** so each idea can be
adopted, tested, and replaced independently.

## Positioning in the Gilamonster constellation

- **Supersedes** the private `drake` swarm (the `drake-arbiter` vendor slots are
  obsolete; there are no vendor pilots).
- **Dispatches** local-model [`newt-agent`](https://github.com/Gilamonster-Foundation/newt-agent)
  instances as workers — dogfood our own agents, no third-party pilots.
- **Rides** [`agent-mesh`](https://github.com/Gilamonster-Foundation/agent-mesh)
  for coordination/airspace (cryptographic, broker-less).
- **Leashes** workers through [`agent-bridle`](https://github.com/Gilamonster-Foundation/agent-bridle)
  — capabilities enforced as `Caveats`, not ambient authority.

## Load-bearing principles (carried forward from drake)

- **Prefer decomposition into reusable crates.** Each lesson is a crate, not a
  module of a monolith.
- **Patch, not prose.** A worker only edits files; orchestration plumbing is the
  desk's job, and the arbiter grades the *diff*.
- **An empty diff is a crash, not a candidate.** Scrubbed pre-arbiter, counted
  against that model's scorecard.
- **Swarm-internal storage is a bare git repo, not a forge.** Forges are export
  targets the operator picks per-candidate, never collaborators in the bake-off.
- **Workers are persistent actors, not request handlers.** Context is the
  substrate; don't tear down between tasks.
- **Flight-ops vocabulary** (sortie / scramble / splash / RTB / debrief) for new
  surfaces, docstrings, and UI strings.

## Source lineage

The drake-era design docs that feed this rewrite are catalogued in
[`docs/MIGRATION_BACKLOG.md`](docs/MIGRATION_BACKLOG.md). They are **inputs to be
revised here**, not files to copy verbatim — and each must pass a go-public
scrub before this repo is made public.
