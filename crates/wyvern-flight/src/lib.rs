// Copyright 2026 The Wyvern Authors
//
// Licensed under either of
//
//   * Apache License, Version 2.0 (LICENSE-APACHE or
//     https://www.apache.org/licenses/LICENSE-2.0)
//   * MIT license (LICENSE-MIT or https://opensource.org/licenses/MIT)
//
// at your option.

//! The deterministic phase state machine — the heart of the old
//! `drake-foreman`, restated as a pure, testable algebra.
//!
//! A [`Flight`] is an ordered list of [`Phase`]s; each phase is a goal
//! plus a bound on how many rounds (retries) a sortie may take before
//! the phase is declared failed. The [`PhaseWalker`] advances a
//! [`FlightState`] in response to an [`Outcome`]:
//!
//! - [`Outcome::Pass`] → splash this phase, advance to the next
//!   (RTB+debrief on the final phase → the flight is complete).
//! - [`Outcome::Fail`] → retry the same phase, incrementing the round,
//!   until the phase's retry bound is exhausted, then the flight fails.
//! - [`Outcome::Bingo`] → bingo fuel: abort the whole flight immediately.
//!
//! ## Determinism (no-wall-clock rule)
//!
//! Nothing in this crate reads a clock, performs IO, or is async. The
//! same `(FlightState, Outcome)` always yields the same next state.
//! Identity and ordering come from content (phase index, goal, round),
//! never from `SystemTime`. The [`CalibrationLog`] is an in-memory,
//! append-only record; serializing it to NDJSON on disk is the caller's
//! job (and lives outside this pure algebra).

use std::fmt;
use wyvern_wire::Verdict;

/// One phase of a flight: a goal and the retry bound for its sorties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phase {
    goal: String,
    /// Maximum number of rounds (attempts) for this phase before it is
    /// declared failed. A bound of `N` allows rounds `0..N`.
    max_rounds: u32,
}

impl Phase {
    /// Build a phase. `max_rounds` of 0 is clamped to 1 (a phase always
    /// gets at least one attempt).
    pub fn new(goal: impl Into<String>, max_rounds: u32) -> Self {
        Phase {
            goal: goal.into(),
            max_rounds: max_rounds.max(1),
        }
    }

    /// The goal a worker flies toward in this phase.
    pub fn goal(&self) -> &str {
        &self.goal
    }

    /// How many rounds this phase allows.
    pub fn max_rounds(&self) -> u32 {
        self.max_rounds
    }
}

/// An ordered sequence of phases — the whole mission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flight {
    phases: Vec<Phase>,
}

impl Flight {
    /// Build a flight from its phases (must be non-empty).
    pub fn new(phases: Vec<Phase>) -> Result<Self, FlightError> {
        if phases.is_empty() {
            return Err(FlightError::NoPhases);
        }
        Ok(Flight { phases })
    }

    /// The phases of this flight.
    pub fn phases(&self) -> &[Phase] {
        &self.phases
    }

    /// The number of phases.
    pub fn len(&self) -> usize {
        self.phases.len()
    }

    /// Whether the flight has no phases. Always false for a constructed
    /// [`Flight`] (the constructor rejects empty), but provided for
    /// lint-clean ergonomics alongside [`Flight::len`].
    pub fn is_empty(&self) -> bool {
        self.phases.is_empty()
    }

    /// The phase at `index`, if any.
    pub fn phase(&self, index: usize) -> Option<&Phase> {
        self.phases.get(index)
    }
}

/// Errors from flight construction or walking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlightError {
    /// A flight must have at least one phase.
    NoPhases,
    /// The walker was advanced after the flight already terminated.
    AlreadyTerminal,
}

impl fmt::Display for FlightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlightError::NoPhases => write!(f, "a flight must have at least one phase"),
            FlightError::AlreadyTerminal => {
                write!(f, "cannot advance a flight that has already terminated")
            }
        }
    }
}

impl std::error::Error for FlightError {}

/// The outcome of grading a phase's sortie(s) for one round.
///
/// This is the *graded* signal fed into the walker. It is derived from
/// one or more [`Verdict`]s upstream (the arbiter's quorum), but the
/// walker only needs the rolled-up decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The phase's goal was met. Advance.
    Pass,
    /// The phase's goal was not met. Retry (within bound) or fail.
    Fail,
    /// Bingo fuel: abort the whole flight now.
    Bingo,
}

impl Outcome {
    /// Roll a single arbiter [`Verdict`] up into a walker [`Outcome`].
    /// A convenience for the common one-voter case; quorum tallying of
    /// many verdicts lives in the arbiter, not here.
    pub fn from_verdict(v: Verdict) -> Self {
        match v {
            Verdict::Pass => Outcome::Pass,
            Verdict::Fail => Outcome::Fail,
            Verdict::BingoAbort => Outcome::Bingo,
        }
    }
}

/// Where a flight currently stands. Atomically saveable/restoreable so
/// a daemon can resume exactly where it left off.
///
/// `phase` indexes into the [`Flight`]'s phases; `round` is the current
/// attempt within that phase (0-based); `goal` is denormalized from the
/// phase for log/debrief convenience.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlightState {
    phase: usize,
    goal: String,
    round: u32,
    status: FlightStatus,
}

/// Terminal vs in-progress status of a flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlightStatus {
    /// Sorties are still being flown.
    InProgress,
    /// Every phase passed — RTB, debrief complete. The good ending.
    Splashed,
    /// A phase exhausted its retry bound. The flight failed.
    Failed,
    /// A voter called bingo — the flight was aborted mid-air.
    BingoAbort,
}

impl FlightStatus {
    /// Whether this status is terminal (no further advance possible).
    pub fn is_terminal(self) -> bool {
        !matches!(self, FlightStatus::InProgress)
    }
}

impl FlightState {
    /// The initial state of a flight: phase 0, round 0, in progress.
    pub fn start(flight: &Flight) -> Self {
        let goal = flight.phases[0].goal().to_string();
        FlightState {
            phase: 0,
            goal,
            round: 0,
            status: FlightStatus::InProgress,
        }
    }

    /// Reconstruct a state from saved fields (for resume). The `goal`
    /// is taken verbatim from the saved record; callers that want to
    /// re-derive it from the flight can use [`FlightState::start`] +
    /// replay instead.
    pub fn restore(
        phase: usize,
        goal: impl Into<String>,
        round: u32,
        status: FlightStatus,
    ) -> Self {
        FlightState {
            phase,
            goal: goal.into(),
            round,
            status,
        }
    }

    /// Current phase index.
    pub fn phase(&self) -> usize {
        self.phase
    }

    /// Current goal (denormalized from the phase).
    pub fn goal(&self) -> &str {
        &self.goal
    }

    /// Current round (attempt) within the phase, 0-based.
    pub fn round(&self) -> u32 {
        self.round
    }

    /// The flight's status.
    pub fn status(&self) -> FlightStatus {
        self.status
    }
}

/// Advances a [`FlightState`] deterministically given a [`Flight`].
///
/// Stateless: the walker borrows the flight definition and transforms
/// states. All state lives in [`FlightState`] so it can be persisted
/// and resumed.
#[derive(Debug, Clone)]
pub struct PhaseWalker<'f> {
    flight: &'f Flight,
}

impl<'f> PhaseWalker<'f> {
    /// Bind a walker to a flight definition.
    pub fn new(flight: &'f Flight) -> Self {
        PhaseWalker { flight }
    }

    /// Apply an [`Outcome`] to `state`, returning the next state.
    ///
    /// - `Pass`: advance to the next phase (or splash on the last).
    /// - `Fail`: retry the same phase if rounds remain, else fail.
    /// - `Bingo`: abort the flight immediately.
    ///
    /// Returns [`FlightError::AlreadyTerminal`] if `state` is already
    /// terminal — terminal states never advance.
    pub fn advance(
        &self,
        state: &FlightState,
        outcome: Outcome,
    ) -> Result<FlightState, FlightError> {
        if state.status.is_terminal() {
            return Err(FlightError::AlreadyTerminal);
        }

        match outcome {
            Outcome::Bingo => Ok(FlightState {
                phase: state.phase,
                goal: state.goal.clone(),
                round: state.round,
                status: FlightStatus::BingoAbort,
            }),
            Outcome::Pass => {
                let next_phase = state.phase + 1;
                if next_phase >= self.flight.len() {
                    // Last phase passed: RTB, splash.
                    Ok(FlightState {
                        phase: state.phase,
                        goal: state.goal.clone(),
                        round: state.round,
                        status: FlightStatus::Splashed,
                    })
                } else {
                    let goal = self.flight.phases[next_phase].goal().to_string();
                    Ok(FlightState {
                        phase: next_phase,
                        goal,
                        round: 0,
                        status: FlightStatus::InProgress,
                    })
                }
            }
            Outcome::Fail => {
                let phase = &self.flight.phases[state.phase];
                let next_round = state.round + 1;
                // Rounds are 0-based; `max_rounds` attempts means valid
                // rounds are 0..max_rounds. If next_round would be out
                // of that range, the phase has exhausted its retries.
                if next_round >= phase.max_rounds() {
                    Ok(FlightState {
                        phase: state.phase,
                        goal: state.goal.clone(),
                        round: state.round,
                        status: FlightStatus::Failed,
                    })
                } else {
                    Ok(FlightState {
                        phase: state.phase,
                        goal: state.goal.clone(),
                        round: next_round,
                        status: FlightStatus::InProgress,
                    })
                }
            }
        }
    }
}

/// One entry in the append-only calibration log: the result of one
/// round of one phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalibrationEntry {
    /// Phase index this round belonged to.
    pub phase: usize,
    /// Round (attempt) number within the phase.
    pub round: u32,
    /// The goal flown.
    pub goal: String,
    /// The outcome that was graded.
    pub outcome: Outcome,
}

impl CalibrationEntry {
    /// Render this entry as a single NDJSON line (one JSON object, no
    /// trailing newline). Hand-rolled to avoid a serde dependency in
    /// the scaffold; fields are simple and string-escaped minimally.
    pub fn to_ndjson(&self) -> String {
        let outcome = match self.outcome {
            Outcome::Pass => "pass",
            Outcome::Fail => "fail",
            Outcome::Bingo => "bingo",
        };
        format!(
            r#"{{"phase":{},"round":{},"goal":{},"outcome":"{}"}}"#,
            self.phase,
            self.round,
            json_string(&self.goal),
            outcome
        )
    }
}

/// Minimal JSON string escaping for the goal field.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// An append-only, ordered record of every graded round.
///
/// Idempotent per `(phase, round)`: appending the same round twice (a
/// retry of a save, a replay) does not duplicate the entry. Insertion
/// order is preserved. Persistence (NDJSON to a file/bare repo) is the
/// caller's concern; this is the pure in-memory record.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CalibrationLog {
    entries: Vec<CalibrationEntry>,
}

impl CalibrationLog {
    /// A fresh, empty log.
    pub fn new() -> Self {
        CalibrationLog::default()
    }

    /// Append an entry, idempotent per `(phase, round)`.
    ///
    /// Returns `true` if the entry was newly recorded, `false` if a
    /// round with the same `(phase, round)` was already present (the
    /// existing entry is left untouched).
    pub fn append(&mut self, entry: CalibrationEntry) -> bool {
        if self
            .entries
            .iter()
            .any(|e| e.phase == entry.phase && e.round == entry.round)
        {
            return false;
        }
        self.entries.push(entry);
        true
    }

    /// All entries in insertion order.
    pub fn entries(&self) -> &[CalibrationEntry] {
        &self.entries
    }

    /// The number of recorded rounds.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Render the whole log as NDJSON (one entry per line).
    pub fn to_ndjson(&self) -> String {
        self.entries
            .iter()
            .map(CalibrationEntry::to_ndjson)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_phase_flight() -> Flight {
        Flight::new(vec![Phase::new("phase-a", 3), Phase::new("phase-b", 3)])
            .expect("non-empty flight")
    }

    #[test]
    fn flight_rejects_empty_phases() {
        assert_eq!(Flight::new(vec![]).unwrap_err(), FlightError::NoPhases);
    }

    #[test]
    fn pass_advances_to_next_phase() {
        let flight = two_phase_flight();
        let walker = PhaseWalker::new(&flight);
        let s0 = FlightState::start(&flight);
        assert_eq!(s0.phase(), 0);
        assert_eq!(s0.goal(), "phase-a");

        let s1 = walker.advance(&s0, Outcome::Pass).unwrap();
        assert_eq!(s1.phase(), 1);
        assert_eq!(s1.round(), 0);
        assert_eq!(s1.goal(), "phase-b");
        assert_eq!(s1.status(), FlightStatus::InProgress);
    }

    #[test]
    fn pass_on_last_phase_splashes() {
        let flight = two_phase_flight();
        let walker = PhaseWalker::new(&flight);
        let s0 = FlightState::start(&flight);
        let s1 = walker.advance(&s0, Outcome::Pass).unwrap();
        let s2 = walker.advance(&s1, Outcome::Pass).unwrap();
        assert_eq!(s2.status(), FlightStatus::Splashed);
        assert!(s2.status().is_terminal());
    }

    #[test]
    fn fail_retries_up_to_bound_then_fails() {
        // max_rounds = 3 => rounds 0,1,2 valid; 3rd fail exhausts.
        let flight = two_phase_flight();
        let walker = PhaseWalker::new(&flight);
        let s0 = FlightState::start(&flight);
        assert_eq!(s0.round(), 0);

        let s1 = walker.advance(&s0, Outcome::Fail).unwrap();
        assert_eq!(s1.round(), 1);
        assert_eq!(s1.status(), FlightStatus::InProgress);

        let s2 = walker.advance(&s1, Outcome::Fail).unwrap();
        assert_eq!(s2.round(), 2);
        assert_eq!(s2.status(), FlightStatus::InProgress);

        let s3 = walker.advance(&s2, Outcome::Fail).unwrap();
        assert_eq!(s3.status(), FlightStatus::Failed);
        assert!(s3.status().is_terminal());
    }

    #[test]
    fn single_round_phase_fails_immediately_on_fail() {
        let flight = Flight::new(vec![Phase::new("only", 1)]).unwrap();
        let walker = PhaseWalker::new(&flight);
        let s0 = FlightState::start(&flight);
        let s1 = walker.advance(&s0, Outcome::Fail).unwrap();
        assert_eq!(s1.status(), FlightStatus::Failed);
    }

    #[test]
    fn bingo_aborts_mid_flight() {
        let flight = two_phase_flight();
        let walker = PhaseWalker::new(&flight);
        let s0 = FlightState::start(&flight);
        let s1 = walker.advance(&s0, Outcome::Bingo).unwrap();
        assert_eq!(s1.status(), FlightStatus::BingoAbort);
        assert_eq!(s1.phase(), 0, "bingo preserves position");
        assert!(s1.status().is_terminal());
    }

    #[test]
    fn terminal_state_cannot_advance() {
        let flight = two_phase_flight();
        let walker = PhaseWalker::new(&flight);
        let s0 = FlightState::start(&flight);
        let aborted = walker.advance(&s0, Outcome::Bingo).unwrap();
        assert_eq!(
            walker.advance(&aborted, Outcome::Pass).unwrap_err(),
            FlightError::AlreadyTerminal
        );
    }

    #[test]
    fn resume_from_saved_state_reproduces_position() {
        // Walk to phase 1 / round 0, "persist", restore, and confirm
        // the next advance behaves identically.
        let flight = two_phase_flight();
        let walker = PhaseWalker::new(&flight);
        let s0 = FlightState::start(&flight);
        let s1 = walker.advance(&s0, Outcome::Pass).unwrap();

        let saved = FlightState::restore(s1.phase(), s1.goal(), s1.round(), s1.status());
        assert_eq!(saved, s1);

        let from_live = walker.advance(&s1, Outcome::Pass).unwrap();
        let from_saved = walker.advance(&saved, Outcome::Pass).unwrap();
        assert_eq!(from_live, from_saved);
        assert_eq!(from_saved.status(), FlightStatus::Splashed);
    }

    #[test]
    fn calibration_log_is_ordered() {
        let mut log = CalibrationLog::new();
        log.append(CalibrationEntry {
            phase: 0,
            round: 0,
            goal: "a".into(),
            outcome: Outcome::Fail,
        });
        log.append(CalibrationEntry {
            phase: 0,
            round: 1,
            goal: "a".into(),
            outcome: Outcome::Pass,
        });
        let entries = log.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].round, 0);
        assert_eq!(entries[1].round, 1);
    }

    #[test]
    fn calibration_log_append_is_idempotent_per_round() {
        let mut log = CalibrationLog::new();
        let entry = CalibrationEntry {
            phase: 1,
            round: 2,
            goal: "g".into(),
            outcome: Outcome::Pass,
        };
        assert!(log.append(entry.clone()), "first append is new");
        assert!(!log.append(entry.clone()), "duplicate round is a no-op");
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn calibration_entry_renders_ndjson() {
        let entry = CalibrationEntry {
            phase: 0,
            round: 3,
            goal: "add \"x\"".into(),
            outcome: Outcome::Bingo,
        };
        assert_eq!(
            entry.to_ndjson(),
            r#"{"phase":0,"round":3,"goal":"add \"x\"","outcome":"bingo"}"#
        );
    }

    #[test]
    fn outcome_rolls_up_from_verdict() {
        assert_eq!(Outcome::from_verdict(Verdict::Pass), Outcome::Pass);
        assert_eq!(Outcome::from_verdict(Verdict::Fail), Outcome::Fail);
        assert_eq!(Outcome::from_verdict(Verdict::BingoAbort), Outcome::Bingo);
    }
}
