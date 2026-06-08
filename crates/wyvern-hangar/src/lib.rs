// Copyright 2026 The Wyvern Authors
//
// Licensed under either of
//
//   * Apache License, Version 2.0 (LICENSE-APACHE or
//     https://www.apache.org/licenses/LICENSE-2.0)
//   * MIT license (LICENSE-MIT or https://opensource.org/licenses/MIT)
//
// at your option.

//! The hangar: where candidate patches are parked.
//!
//! ## Swarm-internal storage is a bare git repo, not a forge
//!
//! Candidates live in a **bare git repository** owned by the swarm —
//! never a forge (GitHub/GitLab/Gitea). Forges are *export targets* the
//! operator picks per-candidate after the bake-off; they are never
//! collaborators in it. The [`CandidateStore`] trait is the seam for
//! that bare-repo backend.
//!
//! The scaffold ships only [`InMemoryHangar`], a deterministic stub.
//! The real bare-git-repo backend (each candidate = a ref/commit in the
//! swarm's bare repo) is a TODO behind this trait.

use std::collections::BTreeMap;
use wyvern_wire::{SortieDebrief, SortieId};

/// Errors from the candidate store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HangarError {
    /// No candidate is parked under the given sortie id.
    NotFound(SortieId),
}

impl std::fmt::Display for HangarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HangarError::NotFound(id) => write!(f, "no candidate parked for sortie {id}"),
        }
    }
}

impl std::error::Error for HangarError {}

/// Parks and retrieves candidate patches keyed by [`SortieId`].
///
/// The real backend is a bare git repo (a candidate becomes a
/// commit/ref); this trait keeps the engine agnostic to that.
pub trait CandidateStore {
    /// Park a debrief (a candidate patch) under its sortie id. Parking
    /// the same sortie again overwrites the prior candidate.
    fn park(&mut self, debrief: SortieDebrief) -> Result<(), HangarError>;

    /// Retrieve the candidate parked under `id`.
    fn candidate(&self, id: &SortieId) -> Result<&SortieDebrief, HangarError>;

    /// How many candidates are currently parked.
    fn len(&self) -> usize;

    /// Whether the hangar is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An in-memory candidate store for tests and the reference rider.
///
/// `BTreeMap` for deterministic iteration order (no hashing
/// nondeterminism). Stands in for the real bare-git-repo backend.
#[derive(Debug, Clone, Default)]
pub struct InMemoryHangar {
    candidates: BTreeMap<String, SortieDebrief>,
}

impl InMemoryHangar {
    /// A fresh, empty hangar.
    pub fn new() -> Self {
        InMemoryHangar::default()
    }
}

impl CandidateStore for InMemoryHangar {
    fn park(&mut self, debrief: SortieDebrief) -> Result<(), HangarError> {
        self.candidates
            .insert(debrief.sortie().as_str().to_string(), debrief);
        Ok(())
    }

    fn candidate(&self, id: &SortieId) -> Result<&SortieDebrief, HangarError> {
        self.candidates
            .get(id.as_str())
            .ok_or_else(|| HangarError::NotFound(id.clone()))
    }

    fn len(&self) -> usize {
        self.candidates.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn debrief(id: &str) -> SortieDebrief {
        SortieDebrief::new(SortieId::new(id), "goal", "--- a\n+++ b\n+x\n", "ctx").unwrap()
    }

    #[test]
    fn park_then_retrieve() {
        let mut hangar = InMemoryHangar::new();
        hangar.park(debrief("s1")).unwrap();
        let got = hangar.candidate(&SortieId::new("s1")).unwrap();
        assert_eq!(got.sortie().as_str(), "s1");
        assert_eq!(hangar.len(), 1);
    }

    #[test]
    fn missing_candidate_is_not_found() {
        let hangar = InMemoryHangar::new();
        let err = hangar.candidate(&SortieId::new("nope")).unwrap_err();
        assert_eq!(err, HangarError::NotFound(SortieId::new("nope")));
    }

    #[test]
    fn parking_same_sortie_overwrites() {
        let mut hangar = InMemoryHangar::new();
        hangar.park(debrief("s1")).unwrap();
        let updated =
            SortieDebrief::new(SortieId::new("s1"), "goal2", "--- a\n+++ b\n+y\n", "ctx2").unwrap();
        hangar.park(updated).unwrap();
        assert_eq!(hangar.len(), 1);
        assert_eq!(
            hangar.candidate(&SortieId::new("s1")).unwrap().goal(),
            "goal2"
        );
    }
}
