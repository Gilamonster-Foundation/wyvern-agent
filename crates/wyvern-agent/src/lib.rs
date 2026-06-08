// Copyright 2026 The Wyvern Authors
//
// Licensed under either of
//
//   * Apache License, Version 2.0 (LICENSE-APACHE or
//     https://www.apache.org/licenses/LICENSE-2.0)
//   * MIT license (LICENSE-MIT or https://opensource.org/licenses/MIT)
//
// at your option.

//! The **Dragon-Rider** orchestrator — the clean-room rewrite of the
//! old `drake-foreman` mechanism.
//!
//! The Dragon-Rider (NEVER "foreman", NEVER "the desk") drives a
//! [`Flight`] end-to-end:
//!
//! 1. **Scramble** a sortie for the current phase/round via a
//!    [`SortieDispatcher`].
//! 2. **Park** the returned debrief (the candidate patch) in a
//!    [`CandidateStore`] — the hangar.
//! 3. **Request grading**: a [`Grader`] turns the debrief into an
//!    [`Outcome`]. In the scaffold the grader is supplied as input
//!    (a deterministic stub / fixed verdict sequence); the real arbiter
//!    quorum drops in behind the same seam.
//! 4. **Advance / retry / abort** the [`FlightState`] via the
//!    [`PhaseWalker`] and **debrief** the round into the
//!    [`CalibrationLog`].
//!
//! Repeat until the flight reaches a terminal status (SPLASH, FAIL, or
//! BINGO). Headless: no UI, no model awareness, no clock.

use async_trait::async_trait;
use wyvern_dispatch::{DispatchError, SortieDispatcher};
use wyvern_flight::{
    CalibrationEntry, CalibrationLog, Flight, FlightState, FlightStatus, Outcome, PhaseWalker,
};
use wyvern_hangar::{CandidateStore, HangarError};
use wyvern_wire::{SortieDebrief, SortieRequest, WireError};

/// Decides an [`Outcome`] for a debrief. The real arbiter quorum
/// (tallying many voter [`Verdict`](wyvern_wire::Verdict)s) implements
/// this; the scaffold uses fixed/stub graders.
#[async_trait]
pub trait Grader {
    /// Grade a debrief into an outcome.
    async fn grade(&self, debrief: &SortieDebrief) -> Outcome;
}

/// A deterministic grader that replays a fixed sequence of outcomes,
/// one per call, for tests. Once exhausted it yields `Fail` (so a flight
/// never hangs waiting on a verdict).
#[derive(Debug, Clone)]
pub struct ScriptedGrader {
    script: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<Outcome>>>,
}

impl ScriptedGrader {
    /// Build a grader from an ordered list of outcomes.
    pub fn new(outcomes: impl IntoIterator<Item = Outcome>) -> Self {
        ScriptedGrader {
            script: std::sync::Arc::new(std::sync::Mutex::new(outcomes.into_iter().collect())),
        }
    }
}

#[async_trait]
impl Grader for ScriptedGrader {
    async fn grade(&self, _debrief: &SortieDebrief) -> Outcome {
        self.script
            .lock()
            .expect("grader lock not poisoned")
            .pop_front()
            .unwrap_or(Outcome::Fail)
    }
}

/// Errors the Dragon-Rider can surface while flying a flight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RideError {
    /// A sortie request could not be built (e.g. blank goal).
    Wire(WireError),
    /// A worker returned no usable candidate.
    Dispatch(DispatchError),
    /// The hangar rejected a candidate.
    Hangar(HangarError),
}

impl std::fmt::Display for RideError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RideError::Wire(e) => write!(f, "wire: {e}"),
            RideError::Dispatch(e) => write!(f, "dispatch: {e}"),
            RideError::Hangar(e) => write!(f, "hangar: {e}"),
        }
    }
}

impl std::error::Error for RideError {}

/// The outcome of flying a whole flight: the terminal state plus the
/// full calibration log of every round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RideReport {
    /// The terminal flight state (Splashed / Failed / BingoAbort).
    pub state: FlightState,
    /// The append-only record of every graded round.
    pub log: CalibrationLog,
}

/// The reference Dragon-Rider orchestrator. Generic over the dispatch,
/// hangar, and grading seams so the real agent-mesh transport / bare-git
/// hangar / arbiter quorum drop in without touching this logic.
pub struct DragonRider<'a, D, H, G>
where
    D: SortieDispatcher,
    H: CandidateStore,
    G: Grader,
{
    dispatcher: &'a D,
    hangar: &'a mut H,
    grader: &'a G,
}

impl<'a, D, H, G> DragonRider<'a, D, H, G>
where
    D: SortieDispatcher,
    H: CandidateStore,
    G: Grader,
{
    /// Mount a rider on its seams.
    pub fn new(dispatcher: &'a D, hangar: &'a mut H, grader: &'a G) -> Self {
        DragonRider {
            dispatcher,
            hangar,
            grader,
        }
    }

    /// Fly `flight` to a terminal status, returning the report.
    ///
    /// Each round: build a content-addressed [`SortieRequest`] for the
    /// current phase/round, scramble it, park the debrief, grade it, log
    /// the round, and advance the [`PhaseWalker`]. A bingo splashes the
    /// flight immediately; an empty diff is surfaced as a crash
    /// (`RideError::Dispatch`), not silently treated as a candidate.
    pub async fn fly(&mut self, flight: &Flight) -> Result<RideReport, RideError> {
        let walker = PhaseWalker::new(flight);
        let mut state = FlightState::start(flight);
        let mut log = CalibrationLog::new();

        while !state.status().is_terminal() {
            let request =
                SortieRequest::new(state.goal(), state.round()).map_err(RideError::Wire)?;

            // Scramble: a worker flies the sortie and debriefs.
            let debrief = self
                .dispatcher
                .scramble(&request)
                .await
                .map_err(RideError::Dispatch)?;

            // Park the candidate patch in the hangar (bare repo, later).
            self.hangar
                .park(debrief.clone())
                .map_err(RideError::Hangar)?;

            // Request grading and debrief the round into the log.
            let outcome = self.grader.grade(&debrief).await;
            log.append(CalibrationEntry {
                phase: state.phase(),
                round: state.round(),
                goal: state.goal().to_string(),
                outcome,
            });

            // Advance / retry / abort. Terminal states stop the loop.
            state = walker
                .advance(&state, outcome)
                .expect("walker only advances non-terminal states inside the loop");
        }

        debug_assert!(matches!(
            state.status(),
            FlightStatus::Splashed | FlightStatus::Failed | FlightStatus::BingoAbort
        ));

        Ok(RideReport { state, log })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wyvern_dispatch::StubDispatcher;
    use wyvern_flight::Phase;
    use wyvern_hangar::InMemoryHangar;

    fn two_phase_flight() -> Flight {
        Flight::new(vec![Phase::new("phase-a", 3), Phase::new("phase-b", 3)]).unwrap()
    }

    #[tokio::test]
    async fn rider_splashes_a_two_phase_flight() {
        let flight = two_phase_flight();
        let dispatcher = StubDispatcher::new();
        let mut hangar = InMemoryHangar::new();
        // Pass phase A, then pass phase B => SPLASH.
        let grader = ScriptedGrader::new([Outcome::Pass, Outcome::Pass]);

        let mut rider = DragonRider::new(&dispatcher, &mut hangar, &grader);
        let report = rider.fly(&flight).await.unwrap();

        assert_eq!(report.state.status(), FlightStatus::Splashed);
        assert_eq!(report.log.len(), 2);
        assert_eq!(report.log.entries()[0].phase, 0);
        assert_eq!(report.log.entries()[0].outcome, Outcome::Pass);
        assert_eq!(report.log.entries()[1].phase, 1);
    }

    #[tokio::test]
    async fn rider_retries_then_advances() {
        let flight = two_phase_flight();
        let dispatcher = StubDispatcher::new();
        let mut hangar = InMemoryHangar::new();
        // Phase A fails once (retry), passes; phase B passes => SPLASH.
        let grader = ScriptedGrader::new([Outcome::Fail, Outcome::Pass, Outcome::Pass]);

        let mut rider = DragonRider::new(&dispatcher, &mut hangar, &grader);
        let report = rider.fly(&flight).await.unwrap();

        assert_eq!(report.state.status(), FlightStatus::Splashed);
        assert_eq!(report.log.len(), 3);
        // Phase 0 round 0 failed, phase 0 round 1 passed, phase 1 passed.
        assert_eq!(report.log.entries()[0].round, 0);
        assert_eq!(report.log.entries()[0].outcome, Outcome::Fail);
        assert_eq!(report.log.entries()[1].round, 1);
        assert_eq!(report.log.entries()[1].outcome, Outcome::Pass);
        assert_eq!(report.log.entries()[2].phase, 1);
    }

    #[tokio::test]
    async fn rider_aborts_on_bingo_mid_flight() {
        let flight = two_phase_flight();
        let dispatcher = StubDispatcher::new();
        let mut hangar = InMemoryHangar::new();
        // First graded round bingoes => abort before reaching phase B.
        let grader = ScriptedGrader::new([Outcome::Bingo]);

        let mut rider = DragonRider::new(&dispatcher, &mut hangar, &grader);
        let report = rider.fly(&flight).await.unwrap();

        assert_eq!(report.state.status(), FlightStatus::BingoAbort);
        assert_eq!(report.state.phase(), 0, "bingo preserves position");
        assert_eq!(report.log.len(), 1);
        assert_eq!(report.log.entries()[0].outcome, Outcome::Bingo);
    }

    #[tokio::test]
    async fn rider_fails_when_phase_exhausts_retries() {
        let flight = Flight::new(vec![Phase::new("only", 2)]).unwrap();
        let dispatcher = StubDispatcher::new();
        let mut hangar = InMemoryHangar::new();
        // max_rounds=2 => rounds 0,1; two fails exhaust the phase.
        let grader = ScriptedGrader::new([Outcome::Fail, Outcome::Fail]);

        let mut rider = DragonRider::new(&dispatcher, &mut hangar, &grader);
        let report = rider.fly(&flight).await.unwrap();

        assert_eq!(report.state.status(), FlightStatus::Failed);
        assert_eq!(report.log.len(), 2);
    }

    #[tokio::test]
    async fn empty_diff_worker_crashes_the_ride() {
        let flight = two_phase_flight();
        let dispatcher = StubDispatcher::empty(); // worker returns vapor
        let mut hangar = InMemoryHangar::new();
        let grader = ScriptedGrader::new([Outcome::Pass]);

        let mut rider = DragonRider::new(&dispatcher, &mut hangar, &grader);
        let err = rider.fly(&flight).await.unwrap_err();
        assert_eq!(
            err,
            RideError::Dispatch(DispatchError::Empty(WireError::EmptyDiff))
        );
    }
}
