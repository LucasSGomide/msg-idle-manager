//! The one numeric readout the application shows: what it costs right now,
//! pinned to the foot of the sidebar and refreshed without being asked
//! (`FR.7.1`, wireframe `memory-footer.md`).
//!
//! Four figures — the application's own process, the running-account count, the
//! aggregate those accounts cost, and the total — joined from two sources: the
//! [`MemoryProbe`] port for the figures and the session book for the count. The
//! widget renders and decides nothing (architecture rule 8).
//!
//! [`MemoryProbe`]: idle_manager_core::MemoryProbe

mod imp;

use std::sync::Arc;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{MemoryProbe, MemoryReading};

/// How often the footer takes a fresh sample. A named constant carrying its
/// unit (code standards rule 5); the roadmap item lists choosing the value as
/// an open blocker, and the runbook records what one sample was measured to
/// cost at this cadence.
const MEMORY_SAMPLE_INTERVAL_SECS: u64 = 5;

/// The unit every figure the footer draws is rounded to for display. Kibibytes
/// are what the probe returns and what is summed; the conversion happens once,
/// here, at the last moment before a label (`FR.7.1`).
const DISPLAY_UNIT: &str = "MiB";

/// The budget the total is judged against, in mebibytes (code standards
/// rule 5). `None` until task 08 measures three games in this application and in
/// the browser it replaces and records the figure in `docs/memory-budget.md`;
/// while it is `None` the footer never takes its warning appearance
/// (`FR.19.2`, `FR.20.1`).
// TODO(05): task 08 replaces this with the measured figure from
// docs/memory-budget.md's budget section.
const MEMORY_BUDGET_MIB: Option<u64> = None;

/// The three sentences the block's tooltip carries, so the reader knows what the
/// figures mean and why there is no per-account one (`FR.7.4`).
const TOOLTIP: &str = "Figures are proportional set size: memory shared between \
processes is divided among the processes sharing it. Per-account figures are \
not available — the engine offers no way to link a process to an account.";

glib::wrapper! {
    /// The sidebar's memory readout, built from `ui/memory-footer.ui`. Build one
    /// with [`MemoryFooter::new`], start it sampling with
    /// [`MemoryFooter::start_sampling`], and tell it the running-account count
    /// with [`MemoryFooter::set_running_count`] on every sidebar sync.
    pub struct MemoryFooter(ObjectSubclass<imp::MemoryFooter>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl MemoryFooter {
    /// Builds a footer showing dashes for every figure — no sample has been
    /// taken yet, and a zero would be a claim that one had (`FR.7.1`).
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Starts sampling `probe` now and every [`MEMORY_SAMPLE_INTERVAL_SECS`]
    /// after. Each sample runs off the GTK main context and only the finished
    /// reading crosses back to touch a label (architecture rule 10). A sample
    /// that fails switches the block to its unavailable line and stops the
    /// timer, rather than leaving stale figures on screen (`FR.18.2`).
    pub fn start_sampling(&self, probe: Arc<dyn MemoryProbe>) {
        self.imp().start_sampling(probe);
    }

    /// Tells the footer how many accounts are currently running, for the label
    /// on the aggregate row. Comes from the session book, never the kernel
    /// (`FR.7.1`).
    pub fn set_running_count(&self, count: usize) {
        self.imp().set_running_count(count);
    }
}

impl Default for MemoryFooter {
    fn default() -> Self {
        Self::new()
    }
}

/// Renders a kibibyte figure in [`DISPLAY_UNIT`], or an en dash when the figure
/// is absent — no sample yet, or a sample that failed (`FR.7.1`).
///
/// Rounds to the nearest whole unit: a footer is a glance, not a ledger, and a
/// figure to the kibibyte would shift under its neighbours on every sample.
fn format_figure(kib: Option<u64>) -> String {
    match kib {
        None => "\u{2013}".to_owned(),
        Some(kib) => {
            let mib = (kib + 512) / 1024;
            format!("{mib} {DISPLAY_UNIT}")
        }
    }
}

/// Whether `reading` is over `budget_mib`, asking the core's verdict rather
/// than comparing figures here (architecture rule 8, `FR.20.1`). Always `false`
/// while no budget has been measured.
fn is_over_budget(reading: MemoryReading, budget_mib: Option<u64>) -> bool {
    let Some(budget_mib) = budget_mib else {
        return false;
    };
    reading
        .budget_verdict(budget_mib.saturating_mul(1024))
        .is_over_budget()
}

/// The line drawn beneath the figures while over budget: names what was
/// exceeded and nothing else (wireframe `## Over budget`).
fn budget_line(budget_mib: u64) -> String {
    format!("over budget ({budget_mib} {DISPLAY_UNIT})")
}

/// The label on the aggregate row: the running-account count folded into the
/// word, so dividing the aggregate figure by it is the obvious next thought
/// (`FR.7.4`, wireframe).
fn running_label(count: usize) -> String {
    match count {
        1 => "1 running".to_owned(),
        other => format!("{other} running"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_absent_figure_renders_as_a_dash() {
        assert_eq!(format_figure(None), "\u{2013}");
    }

    #[test]
    fn a_present_figure_renders_as_a_rounded_value_with_its_unit() {
        assert_eq!(format_figure(Some(85_800)), "84 MiB");
    }

    #[test]
    fn a_zero_figure_renders_as_zero_not_a_dash() {
        assert_eq!(format_figure(Some(0)), "0 MiB");
    }

    #[test]
    fn the_running_label_folds_the_count_into_the_word() {
        assert_eq!(running_label(3), "3 running");
    }

    fn reading(total_kib: u64) -> MemoryReading {
        MemoryReading {
            own_kib: 0,
            descendants_kib: total_kib,
            process_count: 1,
        }
    }

    #[test]
    fn nothing_is_over_budget_while_no_budget_has_been_measured() {
        assert!(!is_over_budget(reading(9_999_999), None));
    }

    #[test]
    fn a_total_past_the_budget_reads_as_over_it() {
        assert!(is_over_budget(reading(700 * 1024), Some(600)));
    }

    #[test]
    fn a_total_at_the_budget_is_not_over_it() {
        assert!(!is_over_budget(reading(600 * 1024), Some(600)));
    }

    #[test]
    fn the_budget_line_names_the_figure_that_was_exceeded() {
        assert_eq!(budget_line(600), "over budget (600 MiB)");
    }

    /// The design doc owes two rules this widget is the first case of (tasks 04
    /// and 08): how a figure is formatted and shown when unmeasured, and a
    /// readout that changes appearance on a measurement, clears itself, and
    /// offers no action — stating how it differs from rule 9's message strip.
    #[test]
    fn the_design_doc_carries_both_rules_this_footer_is_the_first_case_of() {
        let design = include_str!("../../../docs/design.md");

        let has_figure_rule = design.contains("en dash") && design.contains("font-variant-numeric");
        let has_readout_rule = design.contains("clear itself") || design.contains("clears itself");
        let contrasts_the_strip =
            design.contains("opposite of rule 9") || design.contains("rule 9's message strip");

        assert!(
            has_figure_rule && has_readout_rule && contrasts_the_strip,
            "docs/design.md must carry the figure-formatting rule and the \
             self-clearing readout rule, and the second must contrast rule 9"
        );
    }
}
