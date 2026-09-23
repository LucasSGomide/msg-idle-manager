//! The dialog's template children, the poll that keeps its status line true
//! and the countdown a live code runs on (architecture rule 12).

use std::cell::{Cell, RefCell};
use std::sync::Arc;
use std::time::{Duration, Instant};

use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{PhoneLink, PhoneStatus};

use super::qr;

/// How often the status is re-read and the countdown redrawn while the
/// dialog is open. Once a second and no faster: `phone_status()` reads the
/// phone record each time it is asked.
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// The class the not-listening line carries (design rule 8).
const DIM_CLASS: &str = "dim-label";

/// Where the enrolment code stands, as far as the dialog knows.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum CodePhase {
    /// Nothing is on screen: no code was offered yet, or the last one was
    /// spent by a scan.
    #[default]
    Idle,
    /// A code is on screen until `expires_at`.
    Live {
        /// The moment the offer runs out, taken when it was minted.
        expires_at: Instant,
    },
    /// The last code ran out before anything scanned it.
    Expired,
}

/// The status line as drawn: its text, and whether it is dimmed.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StatusLine {
    text: String,
    dim: bool,
}

/// A handler run with the newly observed status.
type StatusHandler = Box<dyn Fn(&PhoneStatus)>;

/// The composite-template backing object for [`super::PhoneDialog`].
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/idlemanager/IdleManager/ui/phone-dialog.ui")]
pub struct PhoneDialog {
    #[template_child]
    status_label: TemplateChild<gtk::Label>,
    #[template_child]
    intro_label: TemplateChild<gtk::Label>,
    #[template_child]
    link_block: TemplateChild<gtk::Box>,
    #[template_child]
    link_label: TemplateChild<gtk::Label>,
    #[template_child]
    code_block: TemplateChild<gtk::Box>,
    #[template_child]
    code_picture: TemplateChild<gtk::Picture>,
    #[template_child]
    address_label: TemplateChild<gtk::Label>,
    #[template_child]
    countdown_label: TemplateChild<gtk::Label>,
    #[template_child]
    enrol_button: TemplateChild<gtk::Button>,
    #[template_child]
    revoke_button: TemplateChild<gtk::Button>,

    link: RefCell<Option<Arc<dyn PhoneLink>>>,
    phase: Cell<CodePhase>,
    /// The status the last poll answered, so a change is reported once and
    /// a scan is recognised as the step from not enrolled to enrolled.
    last_status: RefCell<Option<PhoneStatus>>,
    /// The one-second poll, held so closing the dialog can remove it — the
    /// dialog leaves nothing running behind it (code standards rule 14's
    /// spirit).
    poll: RefCell<Option<glib::SourceId>>,

    pub(super) on_status_changed: RefCell<Option<StatusHandler>>,
}

impl std::fmt::Debug for PhoneDialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhoneDialog")
            .field("phase", &self.phase.get())
            .finish_non_exhaustive()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for PhoneDialog {
    const NAME: &'static str = "IdleManagerPhoneDialog";
    type Type = super::PhoneDialog;
    type ParentType = gtk::Window;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for PhoneDialog {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();

        let dialog = obj.downgrade();
        self.enrol_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog.upgrade() {
                dialog.imp().enrol();
            }
        });

        let dialog = obj.downgrade();
        self.revoke_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog.upgrade() {
                dialog.imp().revoke();
            }
        });

        // Closing is the one way out, whether by the window's own button or
        // Escape below: the poll stops and a code still on screen is
        // withdrawn, since a code nobody can see is only a door left open.
        let dialog = obj.downgrade();
        obj.connect_close_request(move |_| {
            if let Some(dialog) = dialog.upgrade() {
                dialog.imp().shut_down();
            }
            glib::Propagation::Proceed
        });

        let escape = gtk::EventControllerKey::new();
        let dialog = obj.downgrade();
        escape.connect_key_pressed(move |_, key, _, _| {
            if key != gdk::Key::Escape {
                return glib::Propagation::Proceed;
            }
            if let Some(dialog) = dialog.upgrade() {
                dialog.close();
            }
            glib::Propagation::Stop
        });
        obj.add_controller(escape);
    }
}

impl WidgetImpl for PhoneDialog {}
impl WindowImpl for PhoneDialog {}

impl PhoneDialog {
    /// Takes the link, answers the status line at once and starts the poll.
    pub(super) fn configure(&self, link: Arc<dyn PhoneLink>) {
        self.link.replace(Some(link));
        self.refresh();

        let dialog = self.obj().downgrade();
        let source = glib::timeout_add_local(POLL_INTERVAL, move || {
            let Some(dialog) = dialog.upgrade() else {
                return glib::ControlFlow::Break;
            };
            dialog.imp().tick();
            glib::ControlFlow::Continue
        });
        self.poll.replace(Some(source));
    }

    /// One second passed: an expired code is withdrawn and said so, a live
    /// one counts down, and the status line is asked again either way.
    fn tick(&self) {
        if let CodePhase::Live { expires_at } = self.phase.get() {
            match secs_left(expires_at, Instant::now()) {
                0 => {
                    if let Some(link) = self.link() {
                        link.cancel_enrolment();
                    }
                    self.clear_code(CodePhase::Expired);
                }
                secs => self.countdown_label.set_label(&countdown_line(secs)),
            }
        }
        self.refresh();
    }

    /// Asks the link for an offer and shows it two ways. An empty offer —
    /// the link of a desktop that is not listening answers one — shows
    /// nothing: the status line already says why.
    fn enrol(&self) {
        let Some(link) = self.link() else {
            return;
        };
        let offer = link.begin_enrolment();
        if offer.address.is_empty() || offer.expires_in_secs == 0 {
            tracing::warn!("the link offered no enrolment address");
            self.refresh();
            return;
        }
        let Some(texture) = qr::texture(&offer.address) else {
            tracing::warn!(
                address = %offer.address,
                "the enrolment address does not fit a QR code"
            );
            self.refresh();
            return;
        };
        self.code_picture.set_paintable(Some(&texture));
        self.address_label.set_label(&offer.address);
        self.countdown_label
            .set_label(&countdown_line(offer.expires_in_secs));
        self.code_block.set_visible(true);
        self.phase.set(CodePhase::Live {
            expires_at: Instant::now() + Duration::from_secs(offer.expires_in_secs),
        });
        self.refresh();
    }

    /// Cuts the phone off at once (`FR.6.2`) and redraws from what the link
    /// says afterwards.
    fn revoke(&self) {
        if let Some(link) = self.link() {
            link.revoke_phone();
        }
        self.refresh();
    }

    /// Empties the code block and records `phase` as why.
    fn clear_code(&self, phase: CodePhase) {
        self.code_picture.set_paintable(None::<&gdk::Paintable>);
        self.address_label.set_label("");
        self.countdown_label.set_label("");
        self.code_block.set_visible(false);
        self.phase.set(phase);
    }

    /// Re-reads the status, redraws the line and both buttons from it, and
    /// tells the window when it changed. The step from not enrolled to
    /// enrolled while a code shows is the scan: the code has been spent and
    /// leaves the screen (`FR.5.1`).
    fn refresh(&self) {
        let Some(status) = self.link().map(|link| link.phone_status()) else {
            return;
        };
        let changed = self.last_status.borrow().as_ref() != Some(&status);
        if changed
            && matches!(status, PhoneStatus::Enrolled { .. })
            && matches!(self.phase.get(), CodePhase::Live { .. })
        {
            self.clear_code(CodePhase::Idle);
        }

        // The enrolled phone's own address stands in for the code once one
        // is enrolled: what to bookmark, and the way back in for a browser
        // that lost the cookie. Hidden while a code shows, which is the
        // moment a second address would only confuse.
        let address = self
            .link()
            .filter(|_| !matches!(self.phase.get(), CodePhase::Live { .. }))
            .and_then(|link| link.phone_address());
        self.link_label.set_label(address.as_deref().unwrap_or(""));
        self.link_block.set_visible(address.is_some());

        let line = status_line(&status, self.phase.get());
        self.status_label.set_label(&line.text);
        if line.dim {
            self.status_label.add_css_class(DIM_CLASS);
        } else {
            self.status_label.remove_css_class(DIM_CLASS);
        }
        // The explanatory sentence (2.11's wireframe) only where the status
        // line alone would otherwise be the dialog's entire content: nothing
        // enrolled yet and no code on screen.
        self.intro_label.set_visible(matches!(
            (&status, self.phase.get()),
            (PhoneStatus::NotEnrolled, CodePhase::Idle)
        ));
        self.enrol_button
            .set_sensitive(enrol_allowed(&status, self.phase.get()));
        self.revoke_button.set_sensitive(revoke_allowed(&status));

        if changed {
            self.last_status.replace(Some(status.clone()));
            if let Some(handler) = self.on_status_changed.borrow().as_ref() {
                handler(&status);
            }
        }
    }

    /// Stops the poll and withdraws a code still on screen.
    fn shut_down(&self) {
        if let Some(source) = self.poll.take() {
            source.remove();
        }
        if matches!(self.phase.get(), CodePhase::Live { .. }) {
            if let Some(link) = self.link() {
                link.cancel_enrolment();
            }
            self.clear_code(CodePhase::Idle);
        }
    }

    fn link(&self) -> Option<Arc<dyn PhoneLink>> {
        self.link.borrow().clone()
    }
}

/// The status line for `status`, given where the code stands: the four
/// answers the link can give, with the not-listening one dimmed (design
/// rule 8); a live code always reads as waiting, whatever `status` says
/// underneath it; and — only while no phone is enrolled and no code is
/// live — the word that the last code ran out. `Enrolled`/`Un-enrolled` in
/// the domain's own vocabulary read as `Connected`/`Not connected` here
/// (2.11's wireframe: "Enrol" and "Un-enrol" become "Connect" and
/// "Disconnect"). The domain carries no device name to complete the
/// wireframe's `Connected: {device}.` literally, so a plain `Connected.`
/// stands in for it.
fn status_line(status: &PhoneStatus, phase: CodePhase) -> StatusLine {
    let (text, dim) = match status {
        PhoneStatus::NotListening { reason } => (format!("Not listening: {reason}"), true),
        _ if matches!(phase, CodePhase::Live { .. }) => {
            ("Waiting for the phone…".to_owned(), false)
        }
        PhoneStatus::NotEnrolled if phase == CodePhase::Expired => {
            ("The code expired; start again".to_owned(), false)
        }
        PhoneStatus::NotEnrolled => ("Not connected.".to_owned(), false),
        PhoneStatus::Enrolled { attached: false } => {
            ("Paired, but not connected right now.".to_owned(), false)
        }
        PhoneStatus::Enrolled { attached: true } => ("Connected.".to_owned(), false),
    };
    StatusLine { text, dim }
}

/// `Valid for m:ss` for a code with `secs_left` to run.
fn countdown_line(secs_left: u64) -> String {
    format!("Valid for {}:{:02}", secs_left / 60, secs_left % 60)
}

/// Whole seconds until `expires_at`, as seen at `now`; zero once it passed.
fn secs_left(expires_at: Instant, now: Instant) -> u64 {
    expires_at.saturating_duration_since(now).as_secs()
}

/// `Connect phone…` is offered while the desktop listens and no code is live.
fn enrol_allowed(status: &PhoneStatus, phase: CodePhase) -> bool {
    !matches!(status, PhoneStatus::NotListening { .. }) && !matches!(phase, CodePhase::Live { .. })
}

/// `Disconnect` is offered only while there is a phone to cut off.
fn revoke_allowed(status: &PhoneStatus) -> bool {
    matches!(status, PhoneStatus::Enrolled { .. })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn not_listening() -> PhoneStatus {
        PhoneStatus::NotListening {
            reason: "no mesh network address found".to_owned(),
        }
    }

    #[test]
    fn the_status_line_names_each_of_the_four_states() {
        let lines = [
            status_line(&PhoneStatus::NotEnrolled, CodePhase::Idle),
            status_line(&PhoneStatus::Enrolled { attached: false }, CodePhase::Idle),
            status_line(&PhoneStatus::Enrolled { attached: true }, CodePhase::Idle),
            status_line(&not_listening(), CodePhase::Idle),
        ];

        assert_eq!(
            lines.map(|line| line.text),
            [
                "Not connected.",
                "Paired, but not connected right now.",
                "Connected.",
                "Not listening: no mesh network address found",
            ]
        );
    }

    #[test]
    fn only_the_not_listening_line_is_dimmed() {
        let dims = [
            status_line(&PhoneStatus::NotEnrolled, CodePhase::Idle),
            status_line(&PhoneStatus::Enrolled { attached: false }, CodePhase::Idle),
            status_line(&PhoneStatus::Enrolled { attached: true }, CodePhase::Idle),
            status_line(&not_listening(), CodePhase::Idle),
        ];

        assert_eq!(dims.map(|line| line.dim), [false, false, false, true]);
    }

    #[test]
    fn an_expired_code_is_named_only_while_no_phone_is_enrolled() {
        let lines = [
            status_line(&PhoneStatus::NotEnrolled, CodePhase::Expired),
            status_line(
                &PhoneStatus::Enrolled { attached: false },
                CodePhase::Expired,
            ),
            status_line(&not_listening(), CodePhase::Expired),
        ];

        assert_eq!(
            lines.map(|line| line.text),
            [
                "The code expired; start again",
                "Paired, but not connected right now.",
                "Not listening: no mesh network address found",
            ]
        );
    }

    #[test]
    fn a_live_code_always_reads_as_waiting_whatever_status_says_underneath() {
        let live = CodePhase::Live {
            expires_at: Instant::now() + Duration::from_secs(600),
        };
        let lines = [
            status_line(&PhoneStatus::NotEnrolled, live),
            status_line(&PhoneStatus::Enrolled { attached: false }, live),
        ];

        assert_eq!(
            lines.map(|line| line.text),
            ["Waiting for the phone…", "Waiting for the phone…"]
        );
    }

    #[test]
    fn not_listening_wins_over_a_live_code() {
        let live = CodePhase::Live {
            expires_at: Instant::now() + Duration::from_secs(600),
        };

        assert_eq!(
            status_line(&not_listening(), live).text,
            "Not listening: no mesh network address found"
        );
    }

    #[test]
    fn the_countdown_reads_minutes_and_two_digit_seconds() {
        let lines = [countdown_line(600), countdown_line(59), countdown_line(0)];

        assert_eq!(
            lines,
            ["Valid for 10:00", "Valid for 0:59", "Valid for 0:00"]
        );
    }

    #[test]
    fn the_seconds_left_stop_at_zero_once_the_deadline_passed() {
        let now = Instant::now();

        let left = [
            secs_left(now + Duration::from_secs(600), now),
            secs_left(now + Duration::from_millis(1500), now),
            secs_left(now, now + Duration::from_secs(5)),
        ];

        assert_eq!(left, [600, 1, 0]);
    }

    #[test]
    fn enrol_is_refused_while_not_listening_or_while_a_code_is_live() {
        let live = CodePhase::Live {
            expires_at: Instant::now() + Duration::from_secs(600),
        };

        let allowed = [
            enrol_allowed(&PhoneStatus::NotEnrolled, CodePhase::Idle),
            enrol_allowed(&PhoneStatus::NotEnrolled, CodePhase::Expired),
            enrol_allowed(&PhoneStatus::Enrolled { attached: true }, CodePhase::Idle),
            enrol_allowed(&PhoneStatus::NotEnrolled, live),
            enrol_allowed(&not_listening(), CodePhase::Idle),
        ];

        assert_eq!(allowed, [true, true, true, false, false]);
    }

    #[test]
    fn un_enrol_is_offered_only_while_a_phone_is_enrolled() {
        let allowed = [
            revoke_allowed(&PhoneStatus::Enrolled { attached: false }),
            revoke_allowed(&PhoneStatus::Enrolled { attached: true }),
            revoke_allowed(&PhoneStatus::NotEnrolled),
            revoke_allowed(&not_listening()),
        ];

        assert_eq!(allowed, [true, true, false, false]);
    }
}
