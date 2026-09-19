//! `ProcPssProbe` walks the whole descendant tree from the application's own
//! pid — sandboxes and all — reads each process's proportional set size from
//! `smaps_rollup` or a `smaps` fallback, counts only what it actually read,
//! keeps a refusal distinct from a malformed read, and lets a descendant it
//! cannot read — gone, a zombie, refused, malformed — cost the figure one
//! process rather than the whole sample. Only the own process failing fails it.
//!
//! `ProcPssProbe` reads `/proc`, so this whole file is Linux-only, the same
//! way `idle-manager-store`'s `#[cfg(unix)]`-gated tests are (roadmap item 12
//! task 07).
#![cfg(target_os = "linux")]

mod common;

use common::{FakeProc, fixture};
use idle_manager_core::{MemoryProbe, MemoryProbeError};
use idle_manager_metrics::{ProcPssError, ProcPssProbe, ProcessKind};

#[test]
fn the_rendering_grandchild_through_two_sandboxes_is_among_the_descendants() {
    let proc = FakeProc::new("grandchild");
    proc.process(100, "idle-manager", 1, 130_000)
        .process(101, "WebKitNetworkProcess", 100, 40_000)
        .process(102, "bwrap", 100, 900)
        .process(103, "bwrap", 102, 900)
        .process(104, "WebKitWebProcess", 103, 751_000);

    let tree = ProcPssProbe::under(proc.root(), 100)
        .read_tree()
        .expect("the fake tree reads");

    let rendering = tree
        .descendants
        .iter()
        .find(|process| process.kind == ProcessKind::Rendering);
    assert_eq!(rendering.map(|process| process.pid), Some(104));
}

#[test]
fn an_application_with_no_descendants_reads_a_descendant_figure_of_zero() {
    let proc = FakeProc::new("lonely");
    proc.process(100, "idle-manager", 1, 130_000);

    let reading = ProcPssProbe::under(proc.root(), 100)
        .sample()
        .expect("a childless tree is not an error");

    assert_eq!(
        (
            reading.own_kib,
            reading.descendants_kib,
            reading.process_count
        ),
        (130_000, 0, 1)
    );
}

#[test]
fn an_unrelated_process_tree_is_not_counted() {
    let proc = FakeProc::new("unrelated");
    proc.process(100, "idle-manager", 1, 130_000)
        .process(101, "WebKitWebProcess", 100, 500_000)
        .process(900, "firefox", 1, 2_000_000)
        .process(901, "Isolated Web Co", 900, 3_000_000);

    let reading = ProcPssProbe::under(proc.root(), 100)
        .sample()
        .expect("the fake tree reads");

    assert_eq!(reading.total_kib(), 630_000);
}

#[test]
fn a_captured_rollup_and_the_matching_smaps_parse_to_the_same_figure() {
    let from_rollup = FakeProc::new("rollup");
    from_rollup
        .process(100, "idle-manager", 1, 1_000)
        .process_with_rollup(
            101,
            "WebKitWebProcess",
            100,
            &fixture("webprocess-smaps-rollup.txt"),
        );

    let from_smaps = FakeProc::new("smaps");
    from_smaps
        .process(100, "idle-manager", 1, 1_000)
        .process_smaps_only(
            101,
            "WebKitWebProcess",
            100,
            &fixture("webprocess-smaps.txt"),
        );

    let rollup_reading = ProcPssProbe::under(from_rollup.root(), 100)
        .sample()
        .expect("the rollup fixture samples");
    let smaps_reading = ProcPssProbe::under(from_smaps.root(), 100)
        .sample()
        .expect("the smaps fixture samples");

    assert_eq!(
        rollup_reading.descendants_kib,
        smaps_reading.descendants_kib
    );
}

#[test]
fn a_process_whose_rollup_is_absent_falls_back_to_smaps_and_produces_a_figure() {
    let proc = FakeProc::new("fallback");
    proc.process(100, "idle-manager", 1, 1_000)
        .process_smaps_only(
            101,
            "WebKitWebProcess",
            100,
            &fixture("webprocess-smaps.txt"),
        );

    let reading = ProcPssProbe::under(proc.root(), 100)
        .sample()
        .expect("the fake tree samples");

    assert_eq!(reading.descendants_kib, 751_002);
}

#[test]
fn the_own_rollup_with_no_pss_line_is_a_malformed_read_carrying_the_path() {
    let proc = FakeProc::new("nopss");
    proc.process_with_rollup(100, "idle-manager", 1, &fixture("rollup-without-pss.txt"));

    let error = ProcPssProbe::under(proc.root(), 100)
        .read_tree()
        .expect_err("a rollup with no Pss line does not parse");

    let ProcPssError::Malformed { path, .. } = error else {
        panic!("expected Malformed, got {error:?}");
    };
    assert!(path.ends_with("100/smaps_rollup"));
}

#[test]
fn a_malformed_read_is_distinguishable_by_matching_from_a_file_that_was_not_there() {
    let malformed = FakeProc::new("malformed");
    malformed.process_with_rollup(100, "idle-manager", 1, &fixture("rollup-without-pss.txt"));

    let gone = FakeProc::new("gone");
    gone.process(100, "idle-manager", 1, 1_000)
        .process_gone(101, "WebKitWebProcess", 100);

    let malformed_error = ProcPssProbe::under(malformed.root(), 100).read_tree();
    let gone_reading = ProcPssProbe::under(gone.root(), 100).read_tree();

    let told_apart = matches!(malformed_error, Err(ProcPssError::Malformed { .. }))
        && matches!(gone_reading, Ok(tree) if tree.descendants.is_empty());
    assert!(told_apart);
}

#[test]
fn a_malformed_read_maps_to_the_ports_unreadable_error() {
    let proc = FakeProc::new("port-error");
    proc.process_with_rollup(100, "idle-manager", 1, &fixture("rollup-without-pss.txt"));

    let error = ProcPssProbe::under(proc.root(), 100)
        .sample()
        .expect_err("a malformed read reaches the port as an error");

    assert!(matches!(error, MemoryProbeError::Unreadable { .. }));
}

#[test]
fn a_descendant_of_no_known_kind_is_still_counted_and_labelled_unrecognised() {
    let proc = FakeProc::new("unknown-kind");
    proc.process(100, "idle-manager", 1, 1_000)
        .process(101, "WebKitGPUProcess", 100, 42_000);

    let tree = ProcPssProbe::under(proc.root(), 100)
        .read_tree()
        .expect("the fake tree reads");

    assert_eq!(
        tree.descendants
            .first()
            .map(|process| (process.kind, process.pss_kib)),
        Some((ProcessKind::Unrecognised, 42_000))
    );
}

#[test]
fn the_process_count_is_the_number_read_not_the_number_found_in_the_walk() {
    let proc = FakeProc::new("thin-sample");
    proc.process(100, "idle-manager", 1, 130_000)
        .process(101, "WebKitNetworkProcess", 100, 40_000)
        .process_gone(102, "WebKitWebProcess", 100);

    let reading = ProcPssProbe::under(proc.root(), 100)
        .sample()
        .expect("the fake tree samples");

    assert_eq!(reading.process_count, 2);
}

#[test]
fn a_zombie_descendant_with_an_empty_smaps_is_skipped_and_the_sample_still_stands() {
    let proc = FakeProc::new("zombie");
    proc.process(100, "idle-manager", 1, 130_000)
        .process(101, "WebKitWebProcess", 100, 500_000)
        .process_zombie(102, "xdg-terminal-ex", 100);

    let reading = ProcPssProbe::under(proc.root(), 100)
        .sample()
        .expect("one unreaped descendant does not blank the figure");

    assert_eq!(
        (reading.descendants_kib, reading.process_count),
        (500_000, 2)
    );
}

#[test]
fn a_descendant_whose_rollup_is_empty_is_skipped_the_same_way() {
    let proc = FakeProc::new("empty-rollup");
    proc.process(100, "idle-manager", 1, 130_000)
        .process_with_rollup(101, "bwrap", 100, "");

    let reading = ProcPssProbe::under(proc.root(), 100)
        .sample()
        .expect("an empty rollup is a process with nothing left to read");

    assert_eq!(reading.process_count, 1);
}

#[test]
fn a_non_empty_smaps_with_no_pss_line_is_still_a_malformed_read_carrying_the_path() {
    let proc = FakeProc::new("smaps-nopss");
    proc.process_smaps_only(100, "idle-manager", 1, &fixture("smaps-without-pss.txt"));

    let error = ProcPssProbe::under(proc.root(), 100)
        .read_tree()
        .expect_err("content with no Pss line does not parse, empty or not");

    let ProcPssError::Malformed { path, .. } = error else {
        panic!("expected Malformed, got {error:?}");
    };
    assert!(path.ends_with("100/smaps"));
}

#[test]
fn a_descendant_whose_memory_file_does_not_parse_is_skipped_rather_than_failing_the_sample() {
    let proc = FakeProc::new("malformed-descendant");
    proc.process(100, "idle-manager", 1, 1_000)
        .process_with_rollup(
            101,
            "WebKitWebProcess",
            100,
            &fixture("rollup-without-pss.txt"),
        )
        .process(102, "WebKitNetworkProcess", 100, 40_000);

    let reading = ProcPssProbe::under(proc.root(), 100)
        .sample()
        .expect("a descendant that will not parse costs the figure one process");

    assert_eq!(
        (reading.descendants_kib, reading.process_count),
        (40_000, 2)
    );
}

#[test]
fn a_descendant_whose_memory_file_is_refused_is_skipped_rather_than_failing_the_sample() {
    let proc = FakeProc::new("refused-descendant");
    proc.process(100, "idle-manager", 1, 1_000)
        .process_with_unreadable_smaps(101, "bwrap", 100)
        .process(102, "WebKitNetworkProcess", 100, 40_000);

    let reading = ProcPssProbe::under(proc.root(), 100)
        .sample()
        .expect("a descendant that refuses to be read costs the figure one process");

    assert_eq!(
        (reading.descendants_kib, reading.process_count),
        (40_000, 2)
    );
}

#[test]
fn the_own_process_refusing_to_be_read_fails_the_whole_sample() {
    let proc = FakeProc::new("own-refused");
    proc.process_with_unreadable_smaps(100, "idle-manager", 1)
        .process(101, "WebKitWebProcess", 100, 500_000);

    let error = ProcPssProbe::under(proc.root(), 100)
        .read_tree()
        .expect_err("no own figure, no sample");

    assert!(matches!(error, ProcPssError::Refused { .. }));
}
