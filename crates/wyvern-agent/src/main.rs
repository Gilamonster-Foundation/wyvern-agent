// Copyright 2026 The Wyvern Authors
//
// Licensed under either of
//
//   * Apache License, Version 2.0 (LICENSE-APACHE or
//     https://www.apache.org/licenses/LICENSE-2.0)
//   * MIT license (LICENSE-MIT or https://opensource.org/licenses/MIT)
//
// at your option.

//! Thin headless entrypoint for the wyvern orchestration daemon.
//!
//! This binary is deliberately minimal: the daemon's whole job is to
//! mount the [`DragonRider`](wyvern_agent::DragonRider) on its seams
//! (dispatcher, hangar, grader) and fly a [`Flight`]. There is no UI and
//! no model awareness here. The scaffold wires the deterministic stubs
//! and flies a tiny demonstration flight so `cargo run` shows the engine
//! end-to-end; the real daemon swaps in the agent-mesh dispatcher, the
//! bare-git hangar, and the arbiter-quorum grader behind the same seams.

use wyvern_agent::{DragonRider, ScriptedGrader};
use wyvern_dispatch::StubDispatcher;
use wyvern_flight::{Flight, Outcome, Phase};
use wyvern_hangar::InMemoryHangar;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let flight = Flight::new(vec![
        Phase::new("scaffold the engine", 3),
        Phase::new("debrief the calibration log", 3),
    ])?;

    let dispatcher = StubDispatcher::new();
    let mut hangar = InMemoryHangar::new();
    let grader = ScriptedGrader::new([Outcome::Pass, Outcome::Pass]);

    let mut rider = DragonRider::new(&dispatcher, &mut hangar, &grader);
    let report = rider.fly(&flight).await?;

    println!("[wyvern] flight terminated: {:?}", report.state.status());
    println!("[wyvern] calibration log:");
    println!("{}", report.log.to_ndjson());

    Ok(())
}
