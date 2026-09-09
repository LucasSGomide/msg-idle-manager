//! What a memory reading *is*, and the one judgement that follows from it:
//! whether the application has grown past the line the project holds itself to.
//!
//! Nothing here reads the kernel — that is `metrics`' job through the
//! [`crate::MemoryProbe`] port (architecture rules 1, 5). The core only knows
//! the shape an answer takes and how to compare a total against a budget.

/// One sample of what the application costs: its own process, everything it
/// started, and how many processes were counted to get there.
///
/// Every figure is kibibytes (`FR.7.1`). Converting to the unit a person reads
/// happens once, at the last moment before a label is drawn, and never on the
/// way in: a figure already rounded for display cannot be added to another one.
///
/// The process count is not decoration. A reading built from one process on a
/// machine that should have shown four is wrong in a way no figure reveals, and
/// the count is what lets a runbook step (`FR.19.1`) notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryReading {
    /// The application's own process, in kibibytes.
    pub own_kib: u64,
    /// Everything the application started, summed, in kibibytes.
    pub descendants_kib: u64,
    /// How many processes contributed to the two figures above — the number
    /// actually read, not the number found while walking the tree.
    pub process_count: usize,
}

impl MemoryReading {
    /// The application's whole footprint: its own figure plus its descendants',
    /// in kibibytes.
    ///
    /// Derived on every call rather than stored beside the parts, so the total
    /// can never disagree with them.
    #[must_use]
    pub fn total_kib(self) -> u64 {
        self.own_kib.saturating_add(self.descendants_kib)
    }

    /// Whether this reading's total sits within `budget_kib` or over it.
    ///
    /// A total equal to the budget is [`BudgetVerdict::WithinBudget`]; one
    /// kibibyte above it is [`BudgetVerdict::OverBudget`]. The budget is an
    /// argument, not a constant read here: the number is not settled until three
    /// games have been measured against a browser (task 08), and a test picks
    /// its own threshold (architecture rule 9).
    #[must_use]
    pub fn budget_verdict(self, budget_kib: u64) -> BudgetVerdict {
        if self.total_kib() > budget_kib {
            BudgetVerdict::OverBudget
        } else {
            BudgetVerdict::WithinBudget
        }
    }
}

/// Whether a reading's total is within the recorded budget or past it.
///
/// Named for the answer rather than the arithmetic (naming rule 11). The shell
/// reads this off the core and toggles one appearance from it (`FR.20.1`); it
/// never compares figures itself (architecture rule 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetVerdict {
    /// The total is at or below the budget.
    WithinBudget,
    /// The total is above the budget. A statement of fact, not an error
    /// (`FR.20.1`): the shell tints the footer and names the budget, and does
    /// nothing else (`FR.20.2`).
    OverBudget,
}

impl BudgetVerdict {
    /// Whether the reading was over budget — shaped as a question for the one
    /// caller that toggles a CSS class from it (naming rule 12).
    #[must_use]
    pub fn is_over_budget(self) -> bool {
        matches!(self, BudgetVerdict::OverBudget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryProbe, MemoryProbeError};

    fn reading(own_kib: u64, descendants_kib: u64) -> MemoryReading {
        MemoryReading {
            own_kib,
            descendants_kib,
            process_count: 4,
        }
    }

    #[test]
    fn a_reading_exposes_its_own_figure_its_descendants_figure_and_a_process_count() {
        let sample = MemoryReading {
            own_kib: 130_000,
            descendants_kib: 754_000,
            process_count: 4,
        };

        assert_eq!(
            (sample.own_kib, sample.descendants_kib, sample.process_count),
            (130_000, 754_000, 4)
        );
    }

    #[test]
    fn a_total_is_the_sum_of_the_own_and_descendant_figures() {
        let sample = reading(130_000, 754_000);

        assert_eq!(sample.total_kib(), 884_000);
    }

    #[test]
    fn a_total_equal_to_the_budget_is_not_over_it() {
        let sample = reading(400_000, 200_000);

        assert_eq!(sample.budget_verdict(600_000), BudgetVerdict::WithinBudget);
    }

    #[test]
    fn a_total_one_kibibyte_above_the_budget_is_over_it() {
        let sample = reading(400_000, 200_001);

        assert_eq!(sample.budget_verdict(600_000), BudgetVerdict::OverBudget);
    }

    #[test]
    fn the_same_reading_gives_two_answers_for_two_thresholds() {
        let sample = reading(500_000, 100_000);

        assert_eq!(
            (
                sample.budget_verdict(500_000),
                sample.budget_verdict(700_000)
            ),
            (BudgetVerdict::OverBudget, BudgetVerdict::WithinBudget)
        );
    }

    #[derive(Debug)]
    struct FakeProbe(Result<MemoryReading, MemoryProbeError>);

    impl MemoryProbe for FakeProbe {
        fn sample(&self) -> Result<MemoryReading, MemoryProbeError> {
            self.0.clone()
        }
    }

    #[test]
    fn a_fake_probe_satisfies_the_port_without_touching_the_filesystem() {
        let probe = FakeProbe(Ok(reading(130_000, 754_000)));

        let sampled = probe.sample().expect("the fake probe always answers");

        assert_eq!(sampled.total_kib(), 884_000);
    }

    #[test]
    fn a_refusal_and_an_unreadable_output_are_distinguishable_by_matching() {
        let refused = FakeProbe(Err(MemoryProbeError::Refused {
            reason: "permission denied".to_owned(),
        }));
        let unreadable = FakeProbe(Err(MemoryProbeError::Unreadable {
            reason: "no Pss line".to_owned(),
        }));

        let both_matched = matches!(
            (refused.sample(), unreadable.sample()),
            (
                Err(MemoryProbeError::Refused { .. }),
                Err(MemoryProbeError::Unreadable { .. })
            )
        );
        assert!(both_matched);
    }
}
