// Copyright 2026 The Wyvern Authors
//
// Licensed under either of
//
//   * Apache License, Version 2.0 (LICENSE-APACHE or
//     https://www.apache.org/licenses/LICENSE-2.0)
//   * MIT license (LICENSE-MIT or https://opensource.org/licenses/MIT)
//
// at your option.

//! Canonical swarm payload types for the wyvern orchestration engine.
//!
//! These are the *real* versions of the placeholder types that
//! [`drake-agent`](https://github.com/Gilamonster-Foundation/drake-agent)
//! carries as a stub `wyvern-wire` crate (see its `arbiter/scaffold`
//! branch). The names and shapes here are deliberately kept compatible
//! with that stub — [`SortieId`], [`SortieDebrief`], [`Verdict`], and
//! [`WireError`] — so that when drake-agent drops its placeholder and
//! depends on this crate, the swap is mechanical.
//!
//! ## Transport boundary
//!
//! These are *payload* types only. wyvern does not invent its own
//! envelope, signing, or wire-crypto: every value defined here is
//! intended to travel **inside an `agent-mesh` `SignedEnvelope`**. The
//! mesh owns identity, signatures, and the airspace; wyvern owns the
//! swarm-specific cargo. We intentionally do not depend on
//! `agent-mesh-protocol` from the scaffold (to keep it lean); the seam
//! is documented at [`wyvern_dispatch`](https://docs.rs/wyvern-dispatch)
//! where the real transport lives.
//!
//! ## Charter principles enforced at this boundary
//!
//! - **Patch, not prose.** A [`SortieDebrief`] carries the *diff* the
//!   worker produced. The arbiter grades the patch, never a summary.
//! - **An empty diff is a crash, not a candidate.** Constructing a
//!   [`SortieDebrief`] with an empty/whitespace-only diff is an error
//!   ([`WireError::EmptyDiff`]) — the swarm never grades vapor.
//! - **Capabilities, not vendor identities.** Nothing here names a
//!   model or provider. A [`Verdict`] says only pass/fail/bingo.

use std::fmt;

/// Errors produced while constructing wyvern wire values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    /// The debrief carried an empty (or whitespace-only) diff. Per the
    /// charter, "an empty diff is a crash, not a candidate" — the
    /// arbiter never grades vapor. Empty debriefs are scrubbed
    /// pre-arbiter; this is the last-line structural guard.
    EmptyDiff,
    /// A sortie request carried an empty goal. A worker cannot fly
    /// toward nothing.
    EmptyGoal,
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WireError::EmptyDiff => {
                write!(f, "empty diff: an empty diff is a crash, not a candidate")
            }
            WireError::EmptyGoal => {
                write!(f, "empty goal: a worker cannot fly toward nothing")
            }
        }
    }
}

impl std::error::Error for WireError {}

/// Identifies a single sortie (one worker's attempt at a goal).
///
/// Compatible with drake-agent's stub (an opaque string wrapper with
/// [`SortieId::new`] / [`SortieId::as_str`]). Extended here with
/// [`SortieId::content_addressed`], which derives a stable
/// content-addressed id (a BLAKE3 CID-style digest) from the goal and
/// round, so the same sortie always names itself the same way — no
/// wall-clock, no counters of ambient state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SortieId(String);

impl SortieId {
    /// Wrap a string as a sortie id (drake-stub-compatible escape hatch).
    pub fn new(id: impl Into<String>) -> Self {
        SortieId(id.into())
    }

    /// Derive a content-addressed sortie id from its defining inputs.
    ///
    /// Deterministic: the same `(goal, round)` always yields the same
    /// id. Uses BLAKE3 over a domain-separated framing of the inputs
    /// and renders the first 16 bytes as hex, prefixed `b3:`. This is
    /// the no-wall-clock identity primitive — ids come from content,
    /// not from `SystemTime`.
    pub fn content_addressed(goal: &str, round: u32) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"wyvern-sortie-v1\0");
        hasher.update(goal.as_bytes());
        hasher.update(b"\0");
        hasher.update(&round.to_le_bytes());
        let digest = hasher.finalize();
        let hex: String = digest.as_bytes()[..16]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        SortieId(format!("b3:{hex}"))
    }

    /// Borrow the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SortieId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An order to fly a single sortie: do this goal, this round.
///
/// New in the real schema (the drake stub had no request type — drake
/// only consumes debriefs). The Dragon-Rider builds these to scramble
/// workers; the [`SortieId`] is content-addressed from the goal and
/// round so requests are stable and repl-replayable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortieRequest {
    sortie: SortieId,
    goal: String,
    round: u32,
}

impl SortieRequest {
    /// Build a request for `goal` on `round`, minting a content-addressed
    /// [`SortieId`]. Returns [`WireError::EmptyGoal`] if the goal is blank.
    pub fn new(goal: impl Into<String>, round: u32) -> Result<Self, WireError> {
        let goal = goal.into();
        if goal.trim().is_empty() {
            return Err(WireError::EmptyGoal);
        }
        let sortie = SortieId::content_addressed(&goal, round);
        Ok(SortieRequest {
            sortie,
            goal,
            round,
        })
    }

    /// The content-addressed id of this sortie.
    pub fn sortie(&self) -> &SortieId {
        &self.sortie
    }

    /// The goal the worker is ordered to fly toward.
    pub fn goal(&self) -> &str {
        &self.goal
    }

    /// Which round (attempt) of the goal this is.
    pub fn round(&self) -> u32 {
        self.round
    }
}

/// What a worker flew back from a sortie for the arbiter to grade.
///
/// Shape-compatible with drake-agent's stub. The load-bearing field is
/// `diff`: the arbiter grades the patch directly (patch, not prose).
/// `goal` and `context` are extra grading context, not the thing graded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortieDebrief {
    sortie: SortieId,
    goal: String,
    diff: String,
    context: String,
}

impl SortieDebrief {
    /// Construct a debrief, enforcing "an empty diff is a crash".
    ///
    /// Returns [`WireError::EmptyDiff`] if `diff` is empty or contains
    /// only whitespace. This is the structural guarantee that the
    /// arbiter never receives vapor to grade.
    pub fn new(
        sortie: SortieId,
        goal: impl Into<String>,
        diff: impl Into<String>,
        context: impl Into<String>,
    ) -> Result<Self, WireError> {
        let diff = diff.into();
        if diff.trim().is_empty() {
            return Err(WireError::EmptyDiff);
        }
        Ok(SortieDebrief {
            sortie,
            goal: goal.into(),
            diff,
            context: context.into(),
        })
    }

    /// The sortie this debrief belongs to.
    pub fn sortie(&self) -> &SortieId {
        &self.sortie
    }

    /// The goal the worker was flying toward (grading context).
    pub fn goal(&self) -> &str {
        &self.goal
    }

    /// The patch the worker produced — the thing the arbiter grades.
    /// Guaranteed non-empty by construction.
    pub fn diff(&self) -> &str {
        &self.diff
    }

    /// Extra grading context (not the thing graded).
    pub fn context(&self) -> &str {
        &self.context
    }
}

/// A single voter's verdict on a debrief.
///
/// Identical in shape to drake-agent's stub `Verdict`. The arbiter's
/// quorum algebra tallies these. Kept VENDOR-FREE: a verdict says
/// nothing about which model or provider produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The candidate patch meets the goal — splash it in.
    Pass,
    /// The candidate patch does not meet the goal — RTB.
    Fail,
    /// Abort the whole flight: the voter saw something disqualifying
    /// (e.g. an unsafe change) that should short-circuit the quorum. A
    /// "bingo fuel" call — break off immediately.
    BingoAbort,
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Verdict::Pass => "pass",
            Verdict::Fail => "fail",
            Verdict::BingoAbort => "bingo-abort",
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debrief_with_real_diff_constructs() {
        let d = SortieDebrief::new(
            SortieId::new("s1"),
            "add feature",
            "--- a/x\n+++ b/x\n@@\n+line\n",
            "ctx",
        )
        .expect("non-empty diff should construct");
        assert_eq!(d.sortie().as_str(), "s1");
        assert_eq!(d.goal(), "add feature");
        assert!(d.diff().contains("+line"));
        assert_eq!(d.context(), "ctx");
    }

    #[test]
    fn empty_diff_is_a_crash() {
        let err = SortieDebrief::new(SortieId::new("s1"), "goal", "", "ctx")
            .expect_err("empty diff must be rejected");
        assert_eq!(err, WireError::EmptyDiff);
    }

    #[test]
    fn whitespace_only_diff_is_a_crash() {
        let err = SortieDebrief::new(SortieId::new("s1"), "goal", "  \n\t \n", "ctx")
            .expect_err("whitespace-only diff must be rejected");
        assert_eq!(err, WireError::EmptyDiff);
    }

    #[test]
    fn verdict_displays_kebab() {
        // Shape parity with drake-agent's stub Verdict.
        assert_eq!(Verdict::Pass.to_string(), "pass");
        assert_eq!(Verdict::Fail.to_string(), "fail");
        assert_eq!(Verdict::BingoAbort.to_string(), "bingo-abort");
    }

    #[test]
    fn sortie_request_rejects_empty_goal() {
        assert_eq!(
            SortieRequest::new("   ", 0).expect_err("blank goal rejected"),
            WireError::EmptyGoal
        );
    }

    #[test]
    fn sortie_request_mints_content_addressed_id() {
        let r = SortieRequest::new("add feature", 2).expect("valid request");
        assert_eq!(r.goal(), "add feature");
        assert_eq!(r.round(), 2);
        assert!(r.sortie().as_str().starts_with("b3:"));
    }

    #[test]
    fn content_addressed_id_is_deterministic() {
        // Same inputs => same id (no wall-clock, no ambient counter).
        let a = SortieId::content_addressed("goal", 1);
        let b = SortieId::content_addressed("goal", 1);
        assert_eq!(a, b);
    }

    #[test]
    fn content_addressed_id_separates_round_and_goal() {
        let g1 = SortieId::content_addressed("goal", 1);
        let g2 = SortieId::content_addressed("goal", 2);
        let h1 = SortieId::content_addressed("other", 1);
        assert_ne!(g1, g2, "round must change the id");
        assert_ne!(g1, h1, "goal must change the id");
    }
}
