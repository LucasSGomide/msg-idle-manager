//! The notice's template children, its pure `notice_text` mapping from an
//! [`UpdateState`] to what the bar shows, and the three buttons' wiring
//! (architecture rule 12).

use std::cell::RefCell;

use gtk::CompositeTemplate;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::UpdateState;

/// A handler run when the notice's one action button is pressed while it
/// means "fetch" — `Update` on [`UpdateState::Available`], `Try again` on a
/// named [`UpdateState::Failed`].
type FetchHandler = Box<dyn Fn()>;
/// A handler run when the same button is pressed while it means
/// `Restart now` — only on [`UpdateState::Ready`].
type RestartHandler = Box<dyn Fn()>;
/// A handler run when the dismiss cross is pressed.
type DismissHandler = Box<dyn Fn()>;

const CHECKING_FOR_UPDATES: &str = "Checking for updates…";
const CHECKING_THE_DOWNLOAD: &str = "Checking the download…";
const UPDATE_LABEL: &str = "Update";
const TRY_AGAIN_LABEL: &str = "Try again";
const RESTART_NOW_LABEL: &str = "Restart now";

/// What the bar shows for one [`UpdateState`] — everything [`notice_text`]
/// answers besides the state itself, which [`UpdateNotice::render`] also
/// needs to decide the bar's own visibility and the `What's new` link's URI
/// (design rule 12: only text, link visibility and the button's label and
/// sensitivity ever change; nothing moves).
pub(super) struct NoticeText {
    /// The one line the bar shows.
    pub(super) line: String,
    /// Whether `What's new` is shown.
    pub(super) link_shown: bool,
    /// The button's label and whether it is sensitive, or `None` to hide the
    /// button entirely.
    pub(super) button: Option<(&'static str, bool)>,
}

/// Maps `state` to the bar's line, link visibility and button — the
/// wireframe's table (`update-notice.md`), one arm per [`UpdateState`]
/// variant. Pure: no widget, no clock, no network (architecture rule 9's
/// reasoning carried onto the shell's own rendering).
pub(super) fn notice_text(state: &UpdateState) -> NoticeText {
    match state {
        UpdateState::Idle => NoticeText {
            line: String::new(),
            link_shown: false,
            button: None,
        },
        UpdateState::Checking { .. } => NoticeText {
            line: CHECKING_FOR_UPDATES.to_string(),
            link_shown: false,
            button: None,
        },
        UpdateState::UpToDate { current } => NoticeText {
            line: format!("You have the latest version, {current}."),
            link_shown: false,
            button: None,
        },
        UpdateState::Available { version, .. } => NoticeText {
            line: format!("Version {version} is available."),
            link_shown: true,
            button: Some((UPDATE_LABEL, true)),
        },
        UpdateState::Downloading { version, percent } => NoticeText {
            line: format!("Downloading version {version}… {percent}%"),
            link_shown: true,
            button: Some((UPDATE_LABEL, false)),
        },
        UpdateState::Verifying { .. } => NoticeText {
            line: CHECKING_THE_DOWNLOAD.to_string(),
            link_shown: true,
            button: Some((UPDATE_LABEL, false)),
        },
        UpdateState::Ready { version } => NoticeText {
            line: format!("Version {version} is ready. It installs when you quit Idle Manager."),
            link_shown: true,
            button: Some((RESTART_NOW_LABEL, true)),
        },
        UpdateState::Failed { version, reason } => NoticeText {
            // `reason` is already the exact sentence to show — the window
            // shapes it when it turns a channel error into the `CheckFailed`
            // or `Rejected` event, so this one arm serves both the
            // version-known and the check-itself rows of the wireframe's
            // table without needing to tell them apart here.
            line: reason.clone(),
            link_shown: version.is_some(),
            button: version.map(|_| (TRY_AGAIN_LABEL, true)),
        },
    }
}

/// Whether the bar shows at all for `state`. Automatic checking is silent
/// (design's `**States**` bullet: "Automatic check finds nothing or fails:
/// nothing on screen"); every other state either carries something the user
/// asked to see or something they must decide.
fn is_shown(state: &UpdateState) -> bool {
    !matches!(
        state,
        UpdateState::Idle | UpdateState::Checking { manual: false }
    )
}

/// The composite-template backing object for [`super::UpdateNotice`].
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/idlemanager/IdleManager/ui/update-notice.ui")]
pub struct UpdateNotice {
    #[template_child]
    message_label: TemplateChild<gtk::Label>,
    #[template_child]
    whats_new: TemplateChild<gtk::LinkButton>,
    #[template_child]
    action_button: TemplateChild<gtk::Button>,
    #[template_child]
    dismiss_button: TemplateChild<gtk::Button>,

    /// The state [`UpdateNotice::render`] last drew, so the action button's
    /// click can tell `Update`/`Try again` (fetch) apart from `Restart now`
    /// without the caller saying which — the label alone is not enough to
    /// act on safely if a redraw lands between render and the click.
    current_state: RefCell<UpdateState>,
    /// The last `Available` answer's `What's new` address. [`UpdateState`]
    /// carries `notes_url` only on `Available` — `Downloading`, `Verifying`,
    /// `Ready` and a named `Failed` all drop it, since the pure policy
    /// remembers only what deciding the next state needs — so the notice
    /// keeps the address itself to keep the link live across the whole flow.
    /// Cleared back to `None` on `Idle`, so a later `Available` never shows a
    /// stale address before its own `render` call.
    notes_url: RefCell<Option<String>>,

    pub(super) on_fetch_requested: RefCell<Option<FetchHandler>>,
    pub(super) on_restart_requested: RefCell<Option<RestartHandler>>,
    pub(super) on_dismissed: RefCell<Option<DismissHandler>>,
}

impl std::fmt::Debug for UpdateNotice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateNotice").finish_non_exhaustive()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for UpdateNotice {
    const NAME: &'static str = "IdleManagerUpdateNotice";
    type Type = super::UpdateNotice;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for UpdateNotice {
    fn constructed(&self) {
        self.parent_constructed();

        let notice = self.obj().downgrade();
        self.action_button.connect_clicked(move |_| {
            let Some(notice) = notice.upgrade() else {
                return;
            };
            let imp = notice.imp();
            let is_ready = matches!(&*imp.current_state.borrow(), UpdateState::Ready { .. });
            if is_ready {
                if let Some(handler) = imp.on_restart_requested.borrow().as_ref() {
                    handler();
                }
            } else if let Some(handler) = imp.on_fetch_requested.borrow().as_ref() {
                handler();
            }
        });

        // Dismissing hides the bar for the rest of the run even though a
        // dismissed `Ready` stays `Ready` in the policy (so quitting still
        // applies it): the handler runs first — it may itself call `render`
        // with a state that would normally show the bar — and this call
        // wins the last word (design rule 9's dismiss-only-by-hand, carried
        // onto this bar's own new pattern).
        let notice = self.obj().downgrade();
        self.dismiss_button.connect_clicked(move |_| {
            let Some(notice) = notice.upgrade() else {
                return;
            };
            if let Some(handler) = notice.imp().on_dismissed.borrow().as_ref() {
                handler();
            }
            notice.set_visible(false);
        });
    }
}

impl WidgetImpl for UpdateNotice {}
impl BoxImpl for UpdateNotice {}

impl UpdateNotice {
    /// Redraws the bar from `state`: visibility, the line, the link's
    /// visibility and address, and the button's label and sensitivity —
    /// exactly the wireframe's table. Called after every
    /// `UpdatePolicy::apply` answer; the notice decides nothing about
    /// updates itself (architecture rule 8).
    pub(super) fn render(&self, state: &UpdateState) {
        self.current_state.replace(state.clone());

        match state {
            UpdateState::Available { notes_url, .. } => {
                self.notes_url.replace(Some(notes_url.clone()));
            }
            UpdateState::Idle => {
                self.notes_url.replace(None);
            }
            _ => {}
        }

        let text = notice_text(state);
        self.obj().set_visible(is_shown(state));
        self.message_label.set_label(&text.line);

        self.whats_new.set_visible(text.link_shown);
        if text.link_shown
            && let Some(url) = self.notes_url.borrow().as_deref()
        {
            self.whats_new.set_uri(url);
        }

        match text.button {
            Some((label, sensitive)) => {
                self.action_button.set_visible(true);
                self.action_button.set_label(label);
                self.action_button.set_sensitive(sensitive);
            }
            None => self.action_button.set_visible(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use idle_manager_core::Version;

    use super::*;

    #[test]
    fn idle_shows_nothing() {
        let text = notice_text(&UpdateState::Idle);

        assert_eq!(text.line, "");
        assert!(!text.link_shown);
        assert!(text.button.is_none());
    }

    #[test]
    fn checking_reads_checking_for_updates_with_no_link_or_button() {
        let text = notice_text(&UpdateState::Checking { manual: true });

        assert_eq!(text.line, "Checking for updates…");
        assert!(!text.link_shown);
        assert!(text.button.is_none());
    }

    #[test]
    fn up_to_date_names_the_running_version_with_no_link_or_button() {
        let text = notice_text(&UpdateState::UpToDate {
            current: Version::new(0, 2, 0),
        });

        assert_eq!(text.line, "You have the latest version, 0.2.0.");
        assert!(!text.link_shown);
        assert!(text.button.is_none());
    }

    #[test]
    fn available_names_the_version_with_a_link_and_an_update_button() {
        let text = notice_text(&UpdateState::Available {
            version: Version::new(0, 3, 0),
            notes_url: "https://example.test".to_string(),
        });

        assert_eq!(text.line, "Version 0.3.0 is available.");
        assert!(text.link_shown);
        assert_eq!(text.button, Some(("Update", true)));
    }

    #[test]
    fn downloading_shows_the_percentage_with_an_insensitive_update_button() {
        let text = notice_text(&UpdateState::Downloading {
            version: Version::new(0, 3, 0),
            percent: 42,
        });

        assert_eq!(text.line, "Downloading version 0.3.0… 42%");
        assert!(text.link_shown);
        assert_eq!(text.button, Some(("Update", false)));
    }

    #[test]
    fn verifying_reads_checking_the_download_with_an_insensitive_update_button() {
        let text = notice_text(&UpdateState::Verifying {
            version: Version::new(0, 3, 0),
        });

        assert_eq!(text.line, "Checking the download…");
        assert!(text.link_shown);
        assert_eq!(text.button, Some(("Update", false)));
    }

    #[test]
    fn ready_names_the_version_with_a_restart_now_button() {
        let text = notice_text(&UpdateState::Ready {
            version: Version::new(0, 3, 0),
        });

        assert_eq!(
            text.line,
            "Version 0.3.0 is ready. It installs when you quit Idle Manager."
        );
        assert!(text.link_shown);
        assert_eq!(text.button, Some(("Restart now", true)));
    }

    #[test]
    fn a_failure_with_a_version_shows_its_reason_with_a_try_again_button() {
        let text = notice_text(&UpdateState::Failed {
            version: Some(Version::new(0, 3, 0)),
            reason: "The update could not be verified and was discarded.".to_string(),
        });

        assert_eq!(
            text.line,
            "The update could not be verified and was discarded."
        );
        assert!(text.link_shown);
        assert_eq!(text.button, Some(("Try again", true)));
    }

    #[test]
    fn a_check_failure_with_no_version_shows_its_reason_with_no_link_or_button() {
        let text = notice_text(&UpdateState::Failed {
            version: None,
            reason: "Could not check for updates: offline.".to_string(),
        });

        assert_eq!(text.line, "Could not check for updates: offline.");
        assert!(!text.link_shown);
        assert!(text.button.is_none());
    }
}
