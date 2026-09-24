//! Learning that a newer release exists, and deciding what to do about it
//! (roadmap item 16). Everything here is a plain value and a pure
//! transition: no network, no disk, no clock read directly (architecture
//! rules 1, 9) — `idle-manager-update` is the crate that actually asks
//! GitHub and Velopack for anything.
//!
//! [`UpdatePolicy::apply`] is the one function that decides what a check, a
//! download or a dismissal means; [`UpdateSchedule::next_check_due`] is the
//! other, deciding only whether today's automatic check is due yet.

use std::fmt;
use std::str::FromStr;

/// A release version: three non-negative numbers, ordered the way a person
/// reads them — `0.3.0` is newer than `0.2.9`.
///
/// A newtype over the three numbers rather than a string (code standards
/// rule 2): once parsed, a version can be compared and displayed without
/// ever being re-parsed or mismatched against a differently-shaped string
/// elsewhere in the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(pub u16, pub u16, pub u16);

impl Version {
    /// The major, minor and patch numbers, in that order.
    #[must_use]
    pub fn new(major: u16, minor: u16, patch: u16) -> Self {
        Version(major, minor, patch)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// `s` was not a version in the `major.minor.patch` shape [`Version::from_str`]
/// accepts.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("not a version in the form 0.2.0: {0}")]
pub struct ParseVersionError(String);

impl FromStr for Version {
    type Err = ParseVersionError;

    /// Parses `major.minor.patch`, with or without a leading `v` — `0.2.0`
    /// and `v0.2.0` both parse to the same [`Version`]. Anything with a
    /// missing part, an extra part, or a part that is not a plain number
    /// (`0.2`, `0.2.0-beta`) is rejected rather than guessed at.
    ///
    /// # Errors
    ///
    /// [`ParseVersionError`] carrying the original string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let numbers = s.strip_prefix('v').unwrap_or(s);
        let mut parts = numbers.split('.');

        let (Some(major), Some(minor), Some(patch), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(ParseVersionError(s.to_string()));
        };

        let parse_part = |part: &str| {
            part.parse::<u16>()
                .map_err(|_| ParseVersionError(s.to_string()))
        };

        Ok(Version(
            parse_part(major)?,
            parse_part(minor)?,
            parse_part(patch)?,
        ))
    }
}

/// The closed list of states an update can be in — exactly what the notice
/// widget renders (roadmap item 16, `update-notice.md`'s state table).
///
/// Two booleans never describe this: a version is only ever meaningful
/// alongside the state that carries one, so "downloading, but which version"
/// cannot be asked (code standards rule 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateState {
    /// Nothing to show. Nothing has been checked, or the last check found
    /// nothing worth showing.
    Idle,
    /// A check is in flight. `manual` is true when the user asked for it
    /// from the menu, false for the launch-time or daily check — it decides
    /// what an uneventful answer becomes.
    Checking {
        /// Whether the user asked for this check, rather than the clock.
        manual: bool,
    },
    /// A manual check found nothing newer than `current`.
    UpToDate {
        /// The version already running.
        current: Version,
    },
    /// A newer version exists and nothing has been downloaded yet.
    Available {
        /// The version on offer.
        version: Version,
        /// Where "What's new" opens.
        notes_url: String,
    },
    /// The package is being fetched.
    Downloading {
        /// The version being downloaded.
        version: Version,
        /// How much of it has arrived, `0..=100`.
        percent: u8,
    },
    /// The package finished downloading and its checksum and signature are
    /// being checked.
    Verifying {
        /// The version being verified.
        version: Version,
    },
    /// The package verified and is staged to install once the app quits.
    Ready {
        /// The version staged to install.
        version: Version,
    },
    /// A check, a download or a verification did not succeed.
    Failed {
        /// The version that failed to download or verify, when the failure
        /// happened after one was found; `None` for a check that never named
        /// one.
        version: Option<Version>,
        /// One line ready to show as-is.
        reason: String,
    },
}

/// Something happened that may move an [`UpdateState`] to another one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateEvent {
    /// The clock or the user asked for a check.
    CheckRequested {
        /// Whether the user asked, rather than the clock.
        manual: bool,
    },
    /// The check found a newer version.
    Found {
        /// The version found.
        version: Version,
        /// Where "What's new" opens.
        notes_url: String,
    },
    /// The check found nothing newer than `current`.
    NothingNewer {
        /// The version already running.
        current: Version,
    },
    /// The check itself could not be completed — offline, GitHub down, rate
    /// limited.
    CheckFailed {
        /// One line ready to show as-is.
        reason: String,
    },
    /// The user pressed Update or Try again.
    FetchRequested,
    /// The download reported progress, `0..=100`.
    Progress(u8),
    /// The downloaded package passed its checksum and signature checks.
    Verified,
    /// The downloaded package failed a checksum or signature check.
    Rejected {
        /// One line ready to show as-is.
        reason: String,
    },
    /// The user dismissed the notice.
    Dismissed,
}

/// The one thing a caller must go do outside the pure policy, when
/// [`UpdatePolicy::apply`] answers one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum Effect {
    /// Ask the channel whether a newer version exists, on a worker thread.
    RunCheck,
    /// Fetch the package on a worker thread.
    Download,
}

/// Decides what an [`UpdateEvent`] means for the current [`UpdateState`].
///
/// Pure and synchronous (architecture rule 9): every network call and every
/// clock read happens outside it, and only the one fact the policy must
/// remember across calls — the version a user last dismissed — lives on
/// `self`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdatePolicy {
    /// The version an `Available` notice was last dismissed for, so the same
    /// version does not reopen it within this run.
    dismissed: Option<Version>,
}

impl UpdatePolicy {
    /// A policy that has not seen anything dismissed yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies `event` to `state`, returning the next state and, when the
    /// caller has something to go do, the one [`Effect`] that means.
    ///
    /// An event that means nothing for `state` — a stray `Progress` while
    /// idle, a second `FetchRequested` while already downloading — leaves
    /// `state` unchanged and answers no effect, rather than being an error:
    /// the shell forwards intents as they arrive and the policy is the one
    /// place allowed to say "that changes nothing right now".
    pub fn apply(
        &mut self,
        state: UpdateState,
        event: UpdateEvent,
    ) -> (UpdateState, Option<Effect>) {
        use UpdateEvent as E;
        use UpdateState as S;

        // An automatic check's quiet failure and a generic dismissal both
        // answer `(Idle, None)`, but for unrelated reasons — merging them
        // into one pattern would read as a coincidence instead of two rules
        // (code standards rule 16).
        #[allow(clippy::match_same_arms)]
        match (state, event) {
            (_, E::CheckRequested { manual }) => (S::Checking { manual }, Some(Effect::RunCheck)),

            (S::Checking { manual }, E::Found { version, notes_url }) => {
                if !manual && self.dismissed.as_ref() == Some(&version) {
                    (S::Idle, None)
                } else {
                    (S::Available { version, notes_url }, None)
                }
            }
            (S::Checking { manual: true }, E::NothingNewer { current }) => {
                (S::UpToDate { current }, None)
            }
            (S::Checking { manual: true }, E::CheckFailed { reason }) => (
                S::Failed {
                    version: None,
                    reason,
                },
                None,
            ),
            (S::Checking { manual: false }, E::NothingNewer { .. } | E::CheckFailed { .. }) => {
                (S::Idle, None)
            }

            (
                S::Available { version, .. }
                | S::Failed {
                    version: Some(version),
                    ..
                },
                E::FetchRequested,
            ) => (
                S::Downloading {
                    version,
                    percent: 0,
                },
                Some(Effect::Download),
            ),

            (S::Downloading { version, .. }, E::Progress(percent)) if percent >= 100 => {
                (S::Verifying { version }, None)
            }
            (S::Downloading { version, .. }, E::Progress(percent)) => {
                (S::Downloading { version, percent }, None)
            }

            (S::Downloading { version, .. } | S::Verifying { version }, E::Verified) => {
                (S::Ready { version }, None)
            }
            (S::Downloading { version, .. } | S::Verifying { version }, E::Rejected { reason }) => {
                (
                    S::Failed {
                        version: Some(version),
                        reason,
                    },
                    None,
                )
            }

            (S::Available { version, .. }, E::Dismissed) => {
                self.dismissed = Some(version);
                (S::Idle, None)
            }
            (S::Ready { version }, E::Dismissed) => (S::Ready { version }, None),
            (_, E::Dismissed) => (S::Idle, None),

            (other, _) => (other, None),
        }
    }
}

/// Whether today's automatic check is due yet, clocked from outside so the
/// daily schedule is tested without waiting a day (architecture rule 9).
#[derive(Debug, Clone, Copy)]
pub struct UpdateSchedule;

impl UpdateSchedule {
    /// How long an automatic check waits before asking again — once a day.
    pub const CHECK_INTERVAL_SECS: u64 = 86_400;

    /// True when no check has ever run, or [`Self::CHECK_INTERVAL_SECS`] has
    /// passed since `last_check_millis`.
    #[must_use]
    pub fn next_check_due(last_check_millis: Option<u64>, now_millis: u64) -> bool {
        match last_check_millis {
            None => true,
            Some(last) => now_millis.saturating_sub(last) >= Self::CHECK_INTERVAL_SECS * 1_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_from_str_parses_dotted_numbers_with_or_without_a_leading_v() {
        assert_eq!("0.2.0".parse::<Version>(), Ok(Version(0, 2, 0)));
        assert_eq!("v0.2.0".parse::<Version>(), Ok(Version(0, 2, 0)));
    }

    #[test]
    fn version_from_str_rejects_a_missing_part_and_a_trailing_suffix() {
        assert!("0.2".parse::<Version>().is_err());
        assert!("0.2.0-beta".parse::<Version>().is_err());
    }

    #[test]
    fn a_higher_minor_number_orders_above_a_lower_one() {
        assert!(Version(0, 3, 0) > Version(0, 2, 9));
    }

    #[test]
    fn an_automatic_check_requested_from_idle_runs_a_check() {
        let mut policy = UpdatePolicy::new();

        let (state, effect) = policy.apply(
            UpdateState::Idle,
            UpdateEvent::CheckRequested { manual: false },
        );

        assert_eq!(state, UpdateState::Checking { manual: false });
        assert_eq!(effect, Some(Effect::RunCheck));
    }

    #[test]
    fn an_automatic_check_that_finds_nothing_or_fails_goes_quietly_back_to_idle() {
        let mut nothing_newer = UpdatePolicy::new();
        let (state, _) = nothing_newer.apply(
            UpdateState::Idle,
            UpdateEvent::CheckRequested { manual: false },
        );
        let (state, effect) = nothing_newer.apply(
            state,
            UpdateEvent::NothingNewer {
                current: Version(0, 2, 0),
            },
        );
        assert_eq!((state, effect), (UpdateState::Idle, None));

        let mut check_failed = UpdatePolicy::new();
        let (state, _) = check_failed.apply(
            UpdateState::Idle,
            UpdateEvent::CheckRequested { manual: false },
        );
        let (state, effect) = check_failed.apply(
            state,
            UpdateEvent::CheckFailed {
                reason: "offline".to_string(),
            },
        );
        assert_eq!((state, effect), (UpdateState::Idle, None));
    }

    #[test]
    fn a_manual_check_that_finds_nothing_or_fails_answers_on_screen() {
        let mut nothing_newer = UpdatePolicy::new();
        let (state, _) = nothing_newer.apply(
            UpdateState::Idle,
            UpdateEvent::CheckRequested { manual: true },
        );
        let (state, _) = nothing_newer.apply(
            state,
            UpdateEvent::NothingNewer {
                current: Version(0, 2, 0),
            },
        );
        assert_eq!(
            state,
            UpdateState::UpToDate {
                current: Version(0, 2, 0)
            }
        );

        let mut check_failed = UpdatePolicy::new();
        let (state, _) = check_failed.apply(
            UpdateState::Idle,
            UpdateEvent::CheckRequested { manual: true },
        );
        let (state, _) = check_failed.apply(
            state,
            UpdateEvent::CheckFailed {
                reason: "offline".to_string(),
            },
        );
        assert_eq!(
            state,
            UpdateState::Failed {
                version: None,
                reason: "offline".to_string()
            }
        );
    }

    #[test]
    fn a_found_version_becomes_available_and_dismissing_it_records_the_version() {
        let mut policy = UpdatePolicy::new();
        let (state, _) = policy.apply(
            UpdateState::Idle,
            UpdateEvent::CheckRequested { manual: false },
        );

        let (state, _) = policy.apply(
            state,
            UpdateEvent::Found {
                version: Version(0, 3, 0),
                notes_url: "https://example.test".to_string(),
            },
        );
        assert_eq!(
            state,
            UpdateState::Available {
                version: Version(0, 3, 0),
                notes_url: "https://example.test".to_string()
            }
        );

        let (state, effect) = policy.apply(state, UpdateEvent::Dismissed);
        assert_eq!((state, effect), (UpdateState::Idle, None));
    }

    #[test]
    fn a_dismissed_version_found_again_automatically_stays_idle() {
        let mut policy = UpdatePolicy::new();
        let (state, _) = policy.apply(
            UpdateState::Idle,
            UpdateEvent::CheckRequested { manual: false },
        );
        let (state, _) = policy.apply(
            state,
            UpdateEvent::Found {
                version: Version(0, 3, 0),
                notes_url: "https://example.test".to_string(),
            },
        );
        policy.apply(state, UpdateEvent::Dismissed);

        let (state, _) = policy.apply(
            UpdateState::Idle,
            UpdateEvent::CheckRequested { manual: false },
        );
        let (state, _) = policy.apply(
            state,
            UpdateEvent::Found {
                version: Version(0, 3, 0),
                notes_url: "https://example.test".to_string(),
            },
        );

        assert_eq!(state, UpdateState::Idle);
    }

    #[test]
    fn a_newer_version_reopens_the_notice_even_after_a_dismissal() {
        let mut policy = UpdatePolicy::new();
        let (state, _) = policy.apply(
            UpdateState::Idle,
            UpdateEvent::CheckRequested { manual: false },
        );
        let (state, _) = policy.apply(
            state,
            UpdateEvent::Found {
                version: Version(0, 3, 0),
                notes_url: "https://example.test".to_string(),
            },
        );
        policy.apply(state, UpdateEvent::Dismissed);

        let (state, _) = policy.apply(
            UpdateState::Idle,
            UpdateEvent::CheckRequested { manual: false },
        );
        let (state, _) = policy.apply(
            state,
            UpdateEvent::Found {
                version: Version(0, 4, 0),
                notes_url: "https://example.test".to_string(),
            },
        );

        assert_eq!(
            state,
            UpdateState::Available {
                version: Version(0, 4, 0),
                notes_url: "https://example.test".to_string()
            }
        );
    }

    #[test]
    fn a_manual_check_reopens_a_dismissed_version() {
        let mut policy = UpdatePolicy::new();
        let (state, _) = policy.apply(
            UpdateState::Idle,
            UpdateEvent::CheckRequested { manual: false },
        );
        let (state, _) = policy.apply(
            state,
            UpdateEvent::Found {
                version: Version(0, 3, 0),
                notes_url: "https://example.test".to_string(),
            },
        );
        policy.apply(state, UpdateEvent::Dismissed);

        let (state, _) = policy.apply(
            UpdateState::Idle,
            UpdateEvent::CheckRequested { manual: true },
        );
        let (state, _) = policy.apply(
            state,
            UpdateEvent::Found {
                version: Version(0, 3, 0),
                notes_url: "https://example.test".to_string(),
            },
        );

        assert_eq!(
            state,
            UpdateState::Available {
                version: Version(0, 3, 0),
                notes_url: "https://example.test".to_string()
            }
        );
    }

    #[test]
    fn fetching_from_available_downloads_progresses_verifies_and_can_be_retried_on_rejection() {
        let mut policy = UpdatePolicy::new();
        let available = UpdateState::Available {
            version: Version(0, 3, 0),
            notes_url: "https://example.test".to_string(),
        };

        let (state, effect) = policy.apply(available, UpdateEvent::FetchRequested);
        assert_eq!(
            state,
            UpdateState::Downloading {
                version: Version(0, 3, 0),
                percent: 0
            }
        );
        assert_eq!(effect, Some(Effect::Download));

        let (state, _) = policy.apply(state, UpdateEvent::Progress(42));
        assert_eq!(
            state,
            UpdateState::Downloading {
                version: Version(0, 3, 0),
                percent: 42
            }
        );

        let (ready, _) = policy.apply(state.clone(), UpdateEvent::Verified);
        assert_eq!(
            ready,
            UpdateState::Ready {
                version: Version(0, 3, 0)
            }
        );

        let (failed, _) = policy.apply(
            state,
            UpdateEvent::Rejected {
                reason: "bad signature".to_string(),
            },
        );
        assert_eq!(
            failed,
            UpdateState::Failed {
                version: Some(Version(0, 3, 0)),
                reason: "bad signature".to_string()
            }
        );

        let (state, effect) = policy.apply(failed, UpdateEvent::FetchRequested);
        assert_eq!(
            state,
            UpdateState::Downloading {
                version: Version(0, 3, 0),
                percent: 0
            }
        );
        assert_eq!(effect, Some(Effect::Download));
    }

    #[test]
    fn dismissing_a_ready_update_leaves_it_ready() {
        let mut policy = UpdatePolicy::new();
        let ready = UpdateState::Ready {
            version: Version(0, 3, 0),
        };

        let (state, effect) = policy.apply(ready.clone(), UpdateEvent::Dismissed);

        assert_eq!((state, effect), (ready, None));
    }

    #[test]
    fn fetch_requested_changes_nothing_outside_available_or_a_named_failure() {
        let mut policy = UpdatePolicy::new();

        let (state, effect) = policy.apply(UpdateState::Idle, UpdateEvent::FetchRequested);
        assert_eq!((state, effect), (UpdateState::Idle, None));

        let checking = UpdateState::Checking { manual: false };
        let (state, effect) = policy.apply(checking.clone(), UpdateEvent::FetchRequested);
        assert_eq!((state, effect), (checking, None));

        let ready = UpdateState::Ready {
            version: Version(0, 3, 0),
        };
        let (state, effect) = policy.apply(ready.clone(), UpdateEvent::FetchRequested);
        assert_eq!((state, effect), (ready, None));
    }

    #[test]
    fn next_check_due_is_true_with_no_previous_check() {
        assert!(UpdateSchedule::next_check_due(None, 1_000));
    }

    #[test]
    fn next_check_due_is_false_one_second_short_of_a_day() {
        let last = 1_000;
        assert!(!UpdateSchedule::next_check_due(
            Some(last),
            last + 86_399_000
        ));
    }

    #[test]
    fn next_check_due_is_true_exactly_a_day_later() {
        let last = 1_000;
        assert!(UpdateSchedule::next_check_due(
            Some(last),
            last + 86_400_000
        ));
    }
}
