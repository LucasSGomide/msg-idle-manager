//! Not a test of anything: an ignored entry point that keeps a server up on a
//! fixed loopback port with a phone record in memory, so the hand-run
//! `test-script.md` can `curl` the three routes before the desktop window
//! learns to start the server itself (task 06).
//!
//! `cargo nextest run -p idle-manager-remote --run-ignored only --no-capture`
//! prints the enrolment address and holds the server for
//! `IDLE_MANAGER_REMOTE_HOLD_SECS` (default 120) on `IDLE_MANAGER_REMOTE_BIND`
//! (default `127.0.0.1:7466`).

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{ManualClock, MemoryPhoneRecord};
use idle_manager_core::{PhoneLink as _, PhoneRecordStore};
use idle_manager_remote::{Clock, RemoteConfig, RemoteServer, bind_address};

const DEFAULT_BIND: &str = "127.0.0.1:7466";
const DEFAULT_HOLD_SECS: u64 = 120;

#[test]
#[ignore = "keeps a server up for the hand-run test script; run with --run-ignored only"]
fn a_server_stays_up_on_a_fixed_port_for_the_hand_run() {
    let bind =
        std::env::var("IDLE_MANAGER_REMOTE_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_owned());
    let hold_secs = std::env::var("IDLE_MANAGER_REMOTE_HOLD_SECS")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(DEFAULT_HOLD_SECS);
    let config = RemoteConfig {
        bind: bind_address(Some(&bind)).expect("the bind override parses"),
        clock: Arc::new(ManualClock::default()) as Arc<dyn Clock>,
    };

    let (handle, _intents) = RemoteServer::start(
        config,
        Arc::new(MemoryPhoneRecord::default()) as Arc<dyn PhoneRecordStore>,
    )
    .expect("the server binds the port");

    let offer = handle.begin_enrolment();
    eprintln!("listening on {}", handle.local_addr());
    eprintln!(
        "enrol at {} (valid {} s)",
        offer.address, offer.expires_in_secs
    );
    eprintln!("holding for {hold_secs} s");
    std::thread::sleep(Duration::from_secs(hold_secs));
}
