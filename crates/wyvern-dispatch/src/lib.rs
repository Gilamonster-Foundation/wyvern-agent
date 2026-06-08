// Copyright 2026 The Wyvern Authors
//
// Licensed under either of
//
//   * Apache License, Version 2.0 (LICENSE-APACHE or
//     https://www.apache.org/licenses/LICENSE-2.0)
//   * MIT license (LICENSE-MIT or https://opensource.org/licenses/MIT)
//
// at your option.

//! The dispatch seam: how the Dragon-Rider scrambles a sortie to a
//! worker and collects its debrief.
//!
//! ## What's real vs. stubbed
//!
//! The [`SortieDispatcher`] trait is the load-bearing seam. The real
//! implementation will:
//!
//! - reach workers as **newt-agent instances over `agent-mesh`** — the
//!   [`SortieRequest`]/[`SortieDebrief`] payloads ride inside an
//!   `agent-mesh` `SignedEnvelope` (wyvern invents no wire-crypto);
//! - leash each worker with **`agent-bridle` `Caveats`** — a worker is
//!   handed only the capabilities its sortie needs, never ambient
//!   authority. Pilots are selected by the `Caveats` they can satisfy,
//!   never by API provider.
//!
//! Neither dependency is wired in the scaffold (no network in tests,
//! lean build). [`StubDispatcher`] stands in: it synthesizes a
//! deterministic debrief from the request so the engine can be driven
//! end-to-end offline.

use async_trait::async_trait;
use wyvern_wire::{SortieDebrief, SortieRequest, WireError};

/// Errors a dispatcher can return.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    /// The worker flew back nothing usable (e.g. an empty diff). Per the
    /// charter this is a crash, not a candidate — surfaced here so the
    /// Dragon-Rider can count it against the worker's scorecard.
    Empty(WireError),
    /// The transport seam is not implemented in this build. The real
    /// agent-mesh transport replaces this.
    TransportUnavailable,
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatchError::Empty(e) => write!(f, "worker returned no candidate: {e}"),
            DispatchError::TransportUnavailable => {
                write!(
                    f,
                    "no transport: real dispatch rides agent-mesh (not in scaffold)"
                )
            }
        }
    }
}

impl std::error::Error for DispatchError {}

/// Sends a [`SortieRequest`] to a worker and collects a
/// [`SortieDebrief`].
///
/// Async because the real transport is networked (agent-mesh). The
/// trait is the seam; the scaffold only ships [`StubDispatcher`].
#[async_trait]
pub trait SortieDispatcher {
    /// Scramble one sortie and await its debrief.
    async fn scramble(&self, request: &SortieRequest) -> Result<SortieDebrief, DispatchError>;
}

/// A deterministic, offline dispatcher for tests and the reference
/// dragon-rider.
///
/// It does not touch the network, a clock, or any worker. It fabricates
/// a debrief whose diff is derived from the request's goal and round,
/// so the same request always yields the same debrief. Configurable to
/// emit an empty diff (to exercise the empty-diff-is-a-crash path).
#[derive(Debug, Clone, Default)]
pub struct StubDispatcher {
    /// When true, the stub returns an empty diff, which
    /// [`SortieDebrief::new`] rejects — surfaced as
    /// [`DispatchError::Empty`].
    emit_empty: bool,
}

impl StubDispatcher {
    /// A stub that always produces a real (non-empty) diff.
    pub fn new() -> Self {
        StubDispatcher { emit_empty: false }
    }

    /// A stub that always produces an empty diff (to test the crash path).
    pub fn empty() -> Self {
        StubDispatcher { emit_empty: true }
    }

    /// Synthesize a deterministic diff body for a request.
    fn synth_diff(request: &SortieRequest) -> String {
        if request.goal().is_empty() {
            return String::new();
        }
        format!(
            "--- a/sortie\n+++ b/sortie\n@@ round {} @@\n+{}\n",
            request.round(),
            request.goal()
        )
    }
}

#[async_trait]
impl SortieDispatcher for StubDispatcher {
    async fn scramble(&self, request: &SortieRequest) -> Result<SortieDebrief, DispatchError> {
        let diff = if self.emit_empty {
            String::new()
        } else {
            Self::synth_diff(request)
        };
        SortieDebrief::new(
            request.sortie().clone(),
            request.goal(),
            diff,
            format!("stub-dispatch round={}", request.round()),
        )
        .map_err(DispatchError::Empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stub_round_trips_request_to_debrief() {
        let dispatcher = StubDispatcher::new();
        let req = SortieRequest::new("add feature", 0).unwrap();
        let debrief = dispatcher.scramble(&req).await.unwrap();
        assert_eq!(debrief.sortie(), req.sortie());
        assert_eq!(debrief.goal(), "add feature");
        assert!(debrief.diff().contains("add feature"));
    }

    #[tokio::test]
    async fn stub_is_deterministic() {
        let dispatcher = StubDispatcher::new();
        let req = SortieRequest::new("goal", 2).unwrap();
        let a = dispatcher.scramble(&req).await.unwrap();
        let b = dispatcher.scramble(&req).await.unwrap();
        assert_eq!(a, b);
    }

    #[tokio::test]
    async fn empty_stub_surfaces_crash() {
        let dispatcher = StubDispatcher::empty();
        let req = SortieRequest::new("goal", 0).unwrap();
        let err = dispatcher.scramble(&req).await.unwrap_err();
        assert_eq!(err, DispatchError::Empty(WireError::EmptyDiff));
    }
}
