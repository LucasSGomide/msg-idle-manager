//! The bar under the header bar where the user learns a newer version
//! exists, fetches it, and chooses the moment it installs (roadmap item 16
//! task 07, design rule 29).
//!
//! One line of text, a `What's new` link, one state-labelled button and a
//! dismiss cross — the same four places for every [`UpdateState`], only
//! their text, visibility and sensitivity ever changing (design rule 12). It
//! decides nothing about updates itself: every press is forwarded as an
//! event to the pure [`idle_manager_core::UpdatePolicy`], and [`UpdateNotice::render`]
//! draws whatever state comes back.

mod imp;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::UpdateState;

glib::wrapper! {
    /// A dismissible bar built from `ui/update-notice.ui`. Build one with
    /// [`UpdateNotice::new`], redraw it with [`UpdateNotice::render`], and
    /// wire its presses with [`UpdateNotice::connect_fetch_requested`],
    /// [`UpdateNotice::connect_restart_requested`] and
    /// [`UpdateNotice::connect_dismissed`].
    pub struct UpdateNotice(ObjectSubclass<imp::UpdateNotice>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UpdateNotice {
    /// Builds a hidden bar.
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Redraws the bar from `state` — the wireframe's table
    /// (`update-notice.md`), one arm per [`UpdateState`] variant.
    pub fn render(&self, state: &UpdateState) {
        self.imp().render(state);
    }

    /// Registers `handler` to run when the button is pressed while it means
    /// "fetch" — `Update` on `Available`, `Try again` on a named failure.
    /// Replaces any previous handler.
    pub fn connect_fetch_requested(&self, handler: impl Fn() + 'static) {
        self.imp()
            .on_fetch_requested
            .replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run when the button is pressed while it reads
    /// `Restart now` — only on `Ready`. Replaces any previous handler.
    pub fn connect_restart_requested(&self, handler: impl Fn() + 'static) {
        self.imp()
            .on_restart_requested
            .replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run when the dismiss cross is pressed, before
    /// the bar hides itself for the rest of the run. Replaces any previous
    /// handler.
    pub fn connect_dismissed(&self, handler: impl Fn() + 'static) {
        self.imp().on_dismissed.replace(Some(Box::new(handler)));
    }
}

impl Default for UpdateNotice {
    fn default() -> Self {
        Self::new()
    }
}
