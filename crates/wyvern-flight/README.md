# wyvern-flight

The deterministic **phase state machine** — the heart of the old
`drake-foreman`, restated as a pure, testable algebra.

- `Flight` — an ordered list of `Phase`s (each = a goal + a retry bound).
- `PhaseWalker` — advances a `FlightState` given an `Outcome`:
  - `Pass` → advance to next phase (or **SPLASH** on the last);
  - `Fail` → retry the same phase, incrementing the round, until the
    bound is exhausted, then **FAIL**;
  - `Bingo` → **abort** the whole flight immediately.
- `FlightState { phase, goal, round, status }` — atomically
  saveable/restoreable for resume (`restore(..)` reconstructs position).
- `CalibrationLog` — append-only, ordered, **idempotent per
  `(phase, round)`**; renders to NDJSON.

## Determinism (no-wall-clock rule)

No IO, no clocks, no async. The same `(FlightState, Outcome)` always
yields the same next state; identity/ordering come from content (phase
index, goal, round), never `SystemTime`. Persisting the NDJSON log to a
file or the bare-git hangar is the caller's job, outside this algebra.
