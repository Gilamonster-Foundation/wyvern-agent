# wyvern-agent

The **headless** orchestration daemon and the reference **Dragon-Rider**
orchestrator — the clean-room rewrite of the old `drake-foreman`
mechanism.

The Dragon-Rider (NEVER "foreman", NEVER "the desk") drives a `Flight`
end-to-end:

1. **Scramble** a content-addressed sortie for the current phase/round
   via a `SortieDispatcher`.
2. **Park** the returned debrief (candidate patch) in a `CandidateStore`
   (the hangar).
3. **Request grading**: a `Grader` turns the debrief into an `Outcome`.
   In the scaffold the grader is supplied as input (`ScriptedGrader`, a
   fixed verdict sequence); the real arbiter quorum drops in behind the
   same seam.
4. **Advance / retry / abort** the `FlightState` via the `PhaseWalker`
   and **debrief** the round into the `CalibrationLog`.

Repeat until the flight is terminal: **SPLASH** (all phases pass),
**FAIL** (a phase exhausts retries), or **BINGO** (a voter aborts).

Headless: no UI, no model awareness, no clock. An empty diff from a
worker is surfaced as a crash (`RideError::Dispatch`), never parked as a
candidate.

```bash
cargo run -p wyvern-agent   # flies a 2-phase demo flight against stubs
```
