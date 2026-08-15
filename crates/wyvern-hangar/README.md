# wyvern-hangar

> **Legacy / non-release-path:** this candidate store belongs to the retired
> orchestration experiment, not resident state. See the
> [R0 inventory](../../docs/RELEASE_INVENTORY.md).

The **hangar**: where candidate patches are parked.

- `CandidateStore` (trait) — `park(SortieDebrief)` / `candidate(&SortieId)`.
- `InMemoryHangar` — a deterministic (`BTreeMap`-backed) stub.

## Swarm-internal storage is a bare git repo, not a forge

Candidates live in a **bare git repository** owned by the swarm — never
a forge (GitHub/GitLab/Gitea). Forges are *export targets* the operator
picks per-candidate after the bake-off; they are never collaborators in
it.

The real backend (each candidate = a commit/ref in the swarm's bare
repo) is a **TODO** behind `CandidateStore`; the scaffold ships only the
in-memory stub.
