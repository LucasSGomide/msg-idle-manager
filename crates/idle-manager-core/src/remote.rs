//! The words a phone and the desktop exchange: what a phone may ask for, the
//! snapshot it renders, the shapes a captured picture takes, the policy that
//! decides when a silent phone counts as gone, and the script text that turns
//! a tap or a scroll into page events.
//!
//! Everything here is a plain value. The server crate serialises these onto
//! the wire and the shell acts on them; neither the wire nor the screen is
//! known here (architecture rule 1), so every policy is tested with numbers
//! and strings alone.

use crate::session::{Liveness, SessionId};
use crate::workspace_book::WorkspaceBook;

/// How often an attached phone is expected to send a heartbeat.
pub const HEARTBEAT_INTERVAL_SECS: u64 = 5;

/// How long a phone may stay silent before it counts as gone — three missed
/// heartbeats, so one dropped packet on mobile data does not unattach it
/// (Remote Access `FR.4.5`).
pub const SILENCE_LIMIT_SECS: u64 = 15;

/// The viewport mobile mode is entered with when no phone is attached: a
/// common phone screen in CSS pixels (`FR.3.2`).
pub const DEFAULT_MOBILE_VIEWPORT: Viewport = Viewport {
    width: 412,
    height: 915,
};

/// A phone screen's size in logical (CSS) pixels — what the page's
/// `window.innerWidth` × `window.innerHeight` report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    /// Width in logical pixels.
    pub width: u32,
    /// Height in logical pixels.
    pub height: u32,
}

/// Everything a phone may ask the desktop to do — and, by omission, everything
/// it may not.
///
/// This closed set *is* the boundary Remote Access `FR.6.3` demands: adding,
/// renaming, regrouping or deleting an account, changing keep-awake, layout or
/// zoom, and loading another address have no variant, so the desktop refuses
/// them by type rather than by a check someone could forget (code standards
/// rule 1). `Tap` and `Scroll` coordinates are in the viewport's logical
/// pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteIntent {
    /// The phone is watching, with this screen size.
    Attach {
        /// The phone screen's size in logical pixels.
        viewport: Viewport,
    },
    /// The phone stopped watching — hidden, closed, or silent too long.
    Leave,
    /// Show this account on the phone.
    ChooseAccount(SessionId),
    /// Stop this account's rendering process.
    Park(SessionId),
    /// Start this account's rendering process.
    Start(SessionId),
    /// Switch the desktop's mobile mode on or off.
    SetMobileMode(bool),
    /// A tap at this point of the watched page.
    Tap {
        /// Horizontal position in logical pixels from the page's left edge.
        x: u32,
        /// Vertical position in logical pixels from the page's top edge.
        y: u32,
    },
    /// A scroll gesture at this point of the watched page.
    Scroll {
        /// Horizontal position in logical pixels from the page's left edge.
        x: u32,
        /// Vertical position in logical pixels from the page's top edge.
        y: u32,
        /// Horizontal distance to scroll, in logical pixels; positive is right.
        dx: i32,
        /// Vertical distance to scroll, in logical pixels; positive is down.
        dy: i32,
    },
}

/// One account as the phone lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteAccount {
    /// The account's identifier, the one every intent names it by.
    pub id: SessionId,
    /// The name the user gave the account.
    pub name: String,
    /// Whether the account's rendering process is running, stopped or on its
    /// way up — the sidebar's own words (design rule 1).
    pub liveness: Liveness,
}

/// One workspace as the phone lists it: its name and its accounts, in
/// sidebar order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteWorkspace {
    /// The workspace's name.
    pub name: String,
    /// Every account the workspace holds, in the order the sidebar shows them.
    pub accounts: Vec<RemoteAccount>,
}

/// The snapshot the phone renders — sent whole on every change, so the phone
/// keeps no state of its own to fall out of step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteState {
    /// Whether the desktop is in mobile mode.
    pub mobile_mode: bool,
    /// The viewport the watched page is laid out for.
    pub viewport: Viewport,
    /// The account in the shown workspace's focused slot, if that slot holds
    /// one.
    pub current: Option<SessionId>,
    /// Every workspace with its accounts, in sidebar order — Ungrouped last.
    pub workspaces: Vec<RemoteWorkspace>,
}

impl RemoteState {
    /// The snapshot of `book` as it stands: every workspace with every account
    /// listed exactly once under it, and `current` the shown workspace's
    /// focused account.
    ///
    /// Mobile mode and its viewport are reported off and
    /// [`DEFAULT_MOBILE_VIEWPORT`] until the book learns them.
    // TODO(13): read `mobile_mode` and `viewport` from the book once task 03
    // adds `is_mobile_mode` and `mobile_viewport` to `WorkspaceBook`.
    #[must_use]
    pub fn from_book(book: &WorkspaceBook) -> Self {
        let workspaces = book
            .workspaces()
            .map(|view| RemoteWorkspace {
                name: view.name().to_owned(),
                accounts: view
                    .book()
                    .sessions()
                    .iter()
                    .map(|session| RemoteAccount {
                        id: session.id().clone(),
                        name: session.display_name().to_owned(),
                        liveness: session.liveness(),
                    })
                    .collect(),
            })
            .collect();
        let current = book
            .active()
            .focused_session()
            .map(|session| session.id().clone());

        Self {
            mobile_mode: false,
            viewport: DEFAULT_MOBILE_VIEWPORT,
            current,
            workspaces,
        }
    }
}

/// One captured picture of the watched page, in whichever shape the engine
/// produced it. `WebKitGTK` hands over raw pixels; `WebView2` hands over a JPEG
/// already encoded.
#[derive(Debug, PartialEq, Eq)]
pub enum Frame {
    /// Raw pixels, four bytes each in red, green, blue, alpha order.
    Rgba {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
        /// Bytes from the start of one row to the start of the next — at
        /// least `width × 4`, more when the engine pads rows.
        stride: u32,
        /// `stride × height` bytes of pixels.
        bytes: Vec<u8>,
    },
    /// A JPEG image, ready to send as it is.
    Jpeg(Vec<u8>),
}

/// What the desktop can say about its phone at a glance — the dialog's and the
/// header menu's one source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhoneStatus {
    /// The server is not accepting connections at all.
    NotListening {
        /// One line saying why, ready to show as-is.
        reason: String,
    },
    /// The server is listening and no phone has been enrolled.
    NotEnrolled,
    /// A phone is enrolled.
    Enrolled {
        /// Whether that phone is watching right now.
        attached: bool,
    },
}

/// The one phone allowed in: what the store keeps between runs.
///
/// `Debug` prints the secret's length and never its bytes, so a log line or a
/// failed assertion cannot leak it.
#[derive(Clone, PartialEq, Eq)]
pub struct EnrolledPhone {
    /// The identifier the phone presents in its cookie.
    pub device_id: String,
    /// The shared secret both sides prove they hold; never sent on the wire
    /// after enrolment.
    pub secret: Vec<u8>,
    /// When the phone was enrolled, as the server wrote it — an RFC 3339
    /// timestamp, shown to the user and never parsed.
    pub enrolled_on: String,
}

impl std::fmt::Debug for EnrolledPhone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnrolledPhone")
            .field("device_id", &self.device_id)
            .field("secret", &format_args!("[{} bytes]", self.secret.len()))
            .field("enrolled_on", &self.enrolled_on)
            .finish()
    }
}

/// What the desktop shows to enrol a phone: the address to open and how long
/// the offer stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrolmentOffer {
    /// The full address to open on the phone, code included.
    pub address: String,
    /// How many seconds the code stays valid from the moment it was minted.
    pub expires_in_secs: u64,
}

/// What one observation of a [`Presence`] changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum PresenceChange {
    /// Nothing moved.
    Unchanged,
    /// The phone is now attached.
    Attached,
    /// The phone is no longer attached — it left, or fell silent too long.
    /// Emitted exactly once per attachment.
    Gone,
}

/// Whether the phone counts as watching, decided from the moments it was
/// last heard from — clocked from outside, so the policy is tested with
/// numbers (architecture rule 9).
///
/// Attached means an `attach` was received and a heartbeat has arrived within
/// [`SILENCE_LIMIT_SECS`]; once that limit passes without one the phone is
/// gone, and [`Presence::observe`] says so exactly once (`FR.4.5`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Presence {
    last_seen_millis: Option<u64>,
}

impl Presence {
    /// A presence with no phone attached.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the phone currently counts as attached.
    #[must_use]
    pub fn is_attached(&self) -> bool {
        self.last_seen_millis.is_some()
    }

    /// The phone said it is watching, at `now_millis`. [`PresenceChange::Attached`]
    /// the first time; a repeated attach only refreshes the moment it was heard.
    pub fn attach(&mut self, now_millis: u64) -> PresenceChange {
        let was_attached = self.is_attached();
        self.last_seen_millis = Some(now_millis);
        if was_attached {
            PresenceChange::Unchanged
        } else {
            PresenceChange::Attached
        }
    }

    /// The phone was heard from at `now_millis`. Ignored while no phone is
    /// attached: a heartbeat alone never attaches.
    pub fn heartbeat(&mut self, now_millis: u64) {
        if self.last_seen_millis.is_some() {
            self.last_seen_millis = Some(now_millis);
        }
    }

    /// The phone said it stopped watching. [`PresenceChange::Gone`] if it was
    /// attached, otherwise nothing.
    pub fn leave(&mut self) -> PresenceChange {
        if self.last_seen_millis.take().is_some() {
            PresenceChange::Gone
        } else {
            PresenceChange::Unchanged
        }
    }

    /// Checks the clock: [`PresenceChange::Gone`] the first time `now_millis`
    /// is [`SILENCE_LIMIT_SECS`] or more past the last moment the phone was
    /// heard, otherwise nothing.
    pub fn observe(&mut self, now_millis: u64) -> PresenceChange {
        let Some(last_seen) = self.last_seen_millis else {
            return PresenceChange::Unchanged;
        };
        if now_millis.saturating_sub(last_seen) < SILENCE_LIMIT_SECS * 1000 {
            return PresenceChange::Unchanged;
        }
        self.last_seen_millis = None;
        PresenceChange::Gone
    }
}

/// The JavaScript that delivers a tap at (`x`, `y`) viewport pixels to the
/// element under that point.
///
/// Dispatches `pointerdown`, `mousedown`, `pointerup`, `mouseup` and `click`
/// in that order rather than `click` alone: script-dispatched events carry
/// `isTrusted = false`, and a game listening on pointer or mouse events rather
/// than `click` would otherwise never see the tap (roadmap item 13, Technical
/// References).
#[must_use]
pub fn tap_script(x: u32, y: u32) -> String {
    format!(
        "(() => {{
  const target = document.elementFromPoint({x}, {y}) || document.body;
  if (!target) {{ return; }}
  const init = {{
    bubbles: true, cancelable: true, composed: true, view: window,
    clientX: {x}, clientY: {y}, button: 0, buttons: 1,
    pointerId: 1, pointerType: 'touch', isPrimary: true
  }};
  const released = Object.assign({{}}, init, {{ buttons: 0 }});
  target.dispatchEvent(new PointerEvent('pointerdown', init));
  target.dispatchEvent(new MouseEvent('mousedown', init));
  target.dispatchEvent(new PointerEvent('pointerup', released));
  target.dispatchEvent(new MouseEvent('mouseup', released));
  target.dispatchEvent(new MouseEvent('click', released));
  if (typeof target.focus === 'function') {{ target.focus(); }}
}})();"
    )
}

/// The JavaScript that scrolls the page under (`x`, `y`) viewport pixels by
/// (`dx`, `dy`) pixels: it walks up from the element under the point to the
/// nearest ancestor that can scroll in the asked direction and adds the deltas
/// there, or to the window when no such ancestor exists.
#[must_use]
pub fn scroll_script(x: u32, y: u32, dx: i32, dy: i32) -> String {
    format!(
        "(() => {{
  const scrolls = (overflow) => overflow === 'auto' || overflow === 'scroll';
  const canScroll = (element) => {{
    const style = getComputedStyle(element);
    return (scrolls(style.overflowY) && element.scrollHeight > element.clientHeight)
      || (scrolls(style.overflowX) && element.scrollWidth > element.clientWidth);
  }};
  const root = document.scrollingElement || document.documentElement;
  let element = document.elementFromPoint({x}, {y});
  while (element && element !== root && element !== document.body && !canScroll(element)) {{
    element = element.parentElement;
  }}
  if (element && element !== root && element !== document.body) {{
    element.scrollBy({dx}, {dy});
  }} else {{
    window.scrollBy({dx}, {dy});
  }}
}})();"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::WorkspaceId;

    fn a_book_with_two_workspaces() -> (WorkspaceBook, SessionId, SessionId, SessionId) {
        let mut book = WorkspaceBook::default();
        let party = book
            .create_workspace("Party", &[])
            .expect("a fresh name is accepted");
        let a = book
            .add(&party, "A", "https://example.test/a")
            .expect("party has room");
        let b = book
            .add(&party, "B", "https://example.test/b")
            .expect("party has room");
        let c = book
            .add(&WorkspaceId::ungrouped(), "C", "https://example.test/c")
            .expect("ungrouped has room");
        book.park(&b);
        (book, a, b, c)
    }

    #[test]
    fn from_book_lists_every_account_once_under_its_workspace_with_its_liveness() {
        let (book, a, b, c) = a_book_with_two_workspaces();

        let state = RemoteState::from_book(&book);

        assert_eq!(
            state.workspaces,
            vec![
                RemoteWorkspace {
                    name: "Party".to_owned(),
                    accounts: vec![
                        RemoteAccount {
                            id: a,
                            name: "A".to_owned(),
                            liveness: Liveness::Live,
                        },
                        RemoteAccount {
                            id: b,
                            name: "B".to_owned(),
                            liveness: Liveness::Parked,
                        },
                    ],
                },
                RemoteWorkspace {
                    name: "Ungrouped".to_owned(),
                    accounts: vec![RemoteAccount {
                        id: c,
                        name: "C".to_owned(),
                        liveness: Liveness::Live,
                    }],
                },
            ]
        );
    }

    #[test]
    fn from_book_reports_the_shown_workspaces_focused_session_as_current() {
        let (book, _a, _b, c) = a_book_with_two_workspaces();

        let state = RemoteState::from_book(&book);

        assert_eq!(state.current, Some(c));
    }

    #[test]
    fn from_book_on_an_empty_book_has_no_current_account() {
        let book = WorkspaceBook::default();

        let state = RemoteState::from_book(&book);

        assert_eq!(
            (state.current, state.mobile_mode, state.viewport),
            (None, false, DEFAULT_MOBILE_VIEWPORT)
        );
    }

    /// A presence attached at `now_millis`, its attach transition checked.
    fn an_attached_presence(now_millis: u64) -> Presence {
        let mut presence = Presence::new();
        let change = presence.attach(now_millis);
        assert_eq!(change, PresenceChange::Attached);
        presence
    }

    #[test]
    fn presence_is_attached_after_attach_and_a_heartbeat() {
        let mut presence = Presence::new();

        let change = presence.attach(0);
        presence.heartbeat(1_000);

        assert_eq!(
            (change, presence.is_attached()),
            (PresenceChange::Attached, true)
        );
    }

    #[test]
    fn presence_stays_attached_at_fourteen_seconds_of_silence() {
        let mut presence = an_attached_presence(0);
        presence.heartbeat(1_000);

        let change = presence.observe(15_000);

        assert_eq!(
            (change, presence.is_attached()),
            (PresenceChange::Unchanged, true)
        );
    }

    #[test]
    fn presence_reports_gone_once_at_fifteen_seconds_of_silence() {
        let mut presence = an_attached_presence(0);
        presence.heartbeat(1_000);

        let change = presence.observe(16_000);

        assert_eq!(
            (change, presence.is_attached()),
            (PresenceChange::Gone, false)
        );
    }

    #[test]
    fn presence_does_not_report_gone_a_second_time_at_thirty_seconds() {
        let mut presence = an_attached_presence(0);

        let at_fifteen = presence.observe(15_000);
        let at_thirty = presence.observe(30_000);

        assert_eq!(
            (at_fifteen, at_thirty),
            (PresenceChange::Gone, PresenceChange::Unchanged)
        );
    }

    #[test]
    fn a_heartbeat_alone_never_attaches() {
        let mut presence = Presence::new();

        presence.heartbeat(1_000);

        assert!(!presence.is_attached());
    }

    #[test]
    fn an_explicit_leave_reports_gone_once_and_a_later_observation_stays_quiet() {
        let mut presence = an_attached_presence(0);

        let left = presence.leave();
        let later = presence.observe(60_000);

        assert_eq!(
            (left, later),
            (PresenceChange::Gone, PresenceChange::Unchanged)
        );
    }

    #[test]
    fn the_silence_limit_is_three_heartbeat_intervals() {
        assert_eq!(SILENCE_LIMIT_SECS, 3 * HEARTBEAT_INTERVAL_SECS);
    }

    #[test]
    fn tap_script_finds_the_element_under_the_point() {
        let script = tap_script(120, 340);

        assert!(script.contains("document.elementFromPoint(120, 340)"));
    }

    #[test]
    fn tap_script_dispatches_the_pointer_mouse_and_click_events_in_order() {
        let script = tap_script(120, 340);

        let positions: Vec<Option<usize>> = [
            "'pointerdown'",
            "'mousedown'",
            "'pointerup'",
            "'mouseup'",
            "'click'",
        ]
        .iter()
        .map(|name| script.find(name))
        .collect();

        assert!(
            positions.iter().all(Option::is_some) && positions.windows(2).all(|w| w[0] < w[1]),
            "events out of order or missing: {positions:?}"
        );
    }

    #[test]
    fn tap_script_places_the_events_at_the_client_coordinates() {
        let script = tap_script(120, 340);

        assert!(script.contains("clientX: 120, clientY: 340"));
    }

    #[test]
    fn scroll_script_adds_the_deltas_to_the_nearest_scrollable_ancestor() {
        let script = scroll_script(50, 60, 30, -40);

        assert!(
            script.contains("document.elementFromPoint(50, 60)")
                && script.contains("element.parentElement")
                && script.contains("element.scrollBy(30, -40)")
        );
    }

    #[test]
    fn scroll_script_falls_back_to_the_window() {
        let script = scroll_script(50, 60, 30, -40);

        assert!(script.contains("window.scrollBy(30, -40)"));
    }

    #[test]
    fn remote_intent_has_exactly_the_eight_variants_and_no_account_management() {
        let intents = [
            RemoteIntent::Attach {
                viewport: DEFAULT_MOBILE_VIEWPORT,
            },
            RemoteIntent::Leave,
            RemoteIntent::ChooseAccount(SessionId::new("session-0001")),
            RemoteIntent::Park(SessionId::new("session-0001")),
            RemoteIntent::Start(SessionId::new("session-0001")),
            RemoteIntent::SetMobileMode(true),
            RemoteIntent::Tap { x: 1, y: 2 },
            RemoteIntent::Scroll {
                x: 1,
                y: 2,
                dx: 3,
                dy: 4,
            },
        ];

        // Exhaustive by construction: a ninth variant — an add, rename,
        // regroup, delete, keep-awake, layout, zoom or load — fails to compile
        // here until it is named, which is the point.
        let words: Vec<&str> = intents
            .iter()
            .map(|intent| match intent {
                RemoteIntent::Attach { .. } => "attach",
                RemoteIntent::Leave => "leave",
                RemoteIntent::ChooseAccount(_) => "choose",
                RemoteIntent::Park(_) => "park",
                RemoteIntent::Start(_) => "start",
                RemoteIntent::SetMobileMode(_) => "mobile",
                RemoteIntent::Tap { .. } => "tap",
                RemoteIntent::Scroll { .. } => "scroll",
            })
            .collect();

        assert_eq!(
            words,
            [
                "attach", "leave", "choose", "park", "start", "mobile", "tap", "scroll"
            ]
        );
    }

    #[test]
    fn the_default_mobile_viewport_is_a_portrait_phone() {
        assert_eq!(
            (
                DEFAULT_MOBILE_VIEWPORT.width,
                DEFAULT_MOBILE_VIEWPORT.height
            ),
            (412, 915)
        );
    }

    #[test]
    fn an_enrolled_phones_debug_output_never_shows_the_secret() {
        let phone = EnrolledPhone {
            device_id: "device-1".to_owned(),
            secret: vec![0xAB; 32],
            enrolled_on: "2026-09-19T10:00:00Z".to_owned(),
        };

        let printed = format!("{phone:?}");

        assert!(printed.contains("[32 bytes]") && !printed.contains("171"));
    }
}
