//! The footer's template children and the sampling loop (architecture rules 10,
//! 12).

use std::cell::Cell;
use std::sync::Arc;
use std::time::Duration;

use gtk::CompositeTemplate;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{MemoryProbe, MemoryReading};

use super::{
    MEMORY_BUDGET_MIB, MEMORY_SAMPLE_INTERVAL_SECS, TOOLTIP, budget_line, format_figure,
    is_over_budget, running_label,
};

/// The composite-template backing object for [`super::MemoryFooter`].
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/idlemanager/IdleManager/ui/memory-footer.ui")]
pub struct MemoryFooter {
    #[template_child]
    figures: TemplateChild<gtk::Grid>,
    #[template_child]
    app_value: TemplateChild<gtk::Label>,
    #[template_child]
    running_label: TemplateChild<gtk::Label>,
    #[template_child]
    running_value: TemplateChild<gtk::Label>,
    #[template_child]
    total_value: TemplateChild<gtk::Label>,
    #[template_child]
    budget_label: TemplateChild<gtk::Label>,
    #[template_child]
    unavailable_label: TemplateChild<gtk::Label>,

    /// How many accounts are running, from the last sidebar sync. Held so a
    /// fresh sample can relabel the aggregate row without waiting for the next
    /// sync.
    running_count: Cell<usize>,
    /// Whether [`MemoryFooter::start_sampling`] has already been called, so a
    /// second call cannot start a second loop.
    sampling: Cell<bool>,
}

impl std::fmt::Debug for MemoryFooter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryFooter").finish_non_exhaustive()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for MemoryFooter {
    const NAME: &'static str = "IdleManagerMemoryFooter";
    type Type = super::MemoryFooter;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for MemoryFooter {
    fn constructed(&self) {
        self.parent_constructed();
        // One tooltip over the whole block (wireframe).
        self.obj().set_tooltip_text(Some(TOOLTIP));
        self.running_label.set_label(&running_label(0));
    }
}

impl WidgetImpl for MemoryFooter {}
impl BoxImpl for MemoryFooter {}

impl MemoryFooter {
    pub(super) fn set_running_count(&self, count: usize) {
        self.running_count.set(count);
        self.running_label.set_label(&running_label(count));
    }

    pub(super) fn start_sampling(&self, probe: Arc<dyn MemoryProbe>) {
        if self.sampling.replace(true) {
            return;
        }

        let footer = self.obj().downgrade();
        glib::spawn_future_local(async move {
            loop {
                let probe = Arc::clone(&probe);
                let outcome = gio::spawn_blocking(move || probe.sample()).await;

                let Some(footer) = footer.upgrade() else {
                    return;
                };
                match outcome {
                    Ok(Ok(reading)) => footer.imp().show_reading(reading),
                    Ok(Err(error)) => {
                        tracing::warn!(reason = %error, "memory sample failed; footer unavailable");
                        footer.imp().show_unavailable();
                        return;
                    }
                    Err(_) => {
                        tracing::error!("the memory sample task panicked; footer unavailable");
                        footer.imp().show_unavailable();
                        return;
                    }
                }

                glib::timeout_future(Duration::from_secs(MEMORY_SAMPLE_INTERVAL_SECS)).await;
            }
        });
    }

    fn show_reading(&self, reading: MemoryReading) {
        self.figures.set_visible(true);
        self.unavailable_label.set_visible(false);

        self.app_value
            .set_label(&format_figure(Some(reading.own_kib)));
        self.running_label
            .set_label(&running_label(self.running_count.get()));
        self.running_value
            .set_label(&format_figure(Some(reading.descendants_kib)));
        self.total_value
            .set_label(&format_figure(Some(reading.total_kib())));

        self.apply_budget_verdict(reading);
    }

    /// Tints the block and shows the budget line when the sample is over budget,
    /// and clears both the moment one comes back under — nothing latches, and
    /// there is nothing to dismiss (`FR.20.3`). No figure moves.
    fn apply_budget_verdict(&self, reading: MemoryReading) {
        let over = is_over_budget(reading, MEMORY_BUDGET_MIB);
        let footer = self.obj();

        if over {
            footer.add_css_class("over-budget");
            if let Some(budget_mib) = MEMORY_BUDGET_MIB {
                self.budget_label.set_label(&budget_line(budget_mib));
            }
            self.budget_label.set_visible(true);
        } else {
            footer.remove_css_class("over-budget");
            self.budget_label.set_visible(false);
        }
    }

    fn show_unavailable(&self) {
        self.figures.set_visible(false);
        self.budget_label.set_visible(false);
        self.unavailable_label.set_visible(true);
    }
}
