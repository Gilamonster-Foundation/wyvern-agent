# wyvern-agent crate

This crate currently contains the legacy `DragonRider` orchestration scaffold
and a stub-backed demonstration binary. It is retained at R0 to avoid mixing a
product correction with a rewrite, but it is not the resident release path and
does not prove remote dispatch, Agent Bridle execution, or live streaming.

The next composition root will be a lightweight resident/headless Newt runtime.
See the top-level [product README](../../README.md),
[charter](../../docs/CHARTER.md), and
[release inventory](../../docs/RELEASE_INVENTORY.md).
