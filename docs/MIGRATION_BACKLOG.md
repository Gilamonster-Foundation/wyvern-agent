# Migration backlog — drake/gilabot design docs → wyvern-agent

> **Historical:** this swarm-orchestration migration is not part of the
> resident release train. It remains as lineage only; see
> [`CHARTER.md`](CHARTER.md) and [`RELEASE_INVENTORY.md`](RELEASE_INVENTORY.md).

These design docs currently live in the **private** `hartsock/gilabot` repo
(`docs/design/`). They are the source lineage for wyvern's clean-room rewrite.

**Rules:**
- **Revise, don't copy.** Restate each lesson in wyvern's crate-decomposed
  framing; don't paste the gilabot prose verbatim.
- **Scrub before public.** Before this repo flips public, every migrated doc
  must pass the go-public audit: no host paths, no usernames, no private-repo
  links, no DATA720, no secrets.
- **Route by concern.** Docs whose home is another Foundation repo go there, not
  here (see "Routes elsewhere").

## To revise into wyvern (swarm orchestration concern)

| Source (gilabot/docs/design/) | Wyvern target | Notes |
|-------------------------------|---------------|-------|
| `persistent-conversational-contexts.md` | `docs/decisions/` | "worker = persistent context"; the swarm-side root of the [conversation-context architecture](https://github.com/Gilamonster-Foundation/newt-agent/blob/main/docs/decisions/conversation_context_architecture.md) (single-agent projection already landed in newt-agent) |
| `long-running-agent.md` | `docs/decisions/` | persistent-actor lifecycle |
| `UNIFIED_AGENT_SYSTEM.md` | `docs/decisions/` | overall swarm system framing |

## Routes elsewhere (NOT into wyvern)

| Source | Correct home | Reason |
|--------|--------------|--------|
| `swarm-context-security-model.md` | `agent-mesh` | trust-root + capability model the mesh inherits |
| `agent-security-models.md` | `agent-bridle` / `agent-mesh` | Caveats / object-capability layer |
| `monitor-lizard.md` | `monitor-agent` | already has a home |
| `hermetic-vs-lived-agent-environments.md` | newt-agent or wyvern | shared tension; route by primary audience when revised |

## Excluded — do not migrate anywhere public

| Source | Reason |
|--------|--------|
| `docs/design/forensics/` (whole dir) | operational forensics; **contains a NATS nkeys Secret manifest** (`opencode-adapter-secret-nats-nkeys.yaml`). Never propagate. Consider rotating the nkeys and scrubbing gilabot history. |
