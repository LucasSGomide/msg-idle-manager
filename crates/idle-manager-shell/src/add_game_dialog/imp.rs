//! The dialog's template children, its two-stage flow, and its keyboard and
//! sensitivity wiring (architecture rule 12).

use std::cell::RefCell;

use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{Destinations, Preset, PresetCatalogue, WorkspaceId, account_name};

use super::Confirmed;

/// The label of the escape-hatch row, appended after every catalogue game.
const SOMETHING_ELSE: &str = "Something else…";

/// A handler run with the user's choice when they confirm.
type ConfirmHandler = Box<dyn Fn(&Confirmed)>;

/// The composite-template backing object for [`super::AddGameDialog`].
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/idlemanager/IdleManager/ui/add-game-dialog.ui")]
pub struct AddGameDialog {
    #[template_child]
    stages: TemplateChild<gtk::Stack>,
    #[template_child]
    game_list: TemplateChild<gtk::ListView>,
    #[template_child]
    empty_list_label: TemplateChild<gtk::Label>,
    #[template_child]
    failure_label: TemplateChild<gtk::Label>,
    #[template_child]
    chosen_game_label: TemplateChild<gtk::Label>,
    #[template_child]
    back_button: TemplateChild<gtk::Button>,
    #[template_child]
    name_entry: TemplateChild<gtk::Entry>,
    #[template_child]
    address_label: TemplateChild<gtk::Label>,
    #[template_child]
    address_entry: TemplateChild<gtk::Entry>,
    #[template_child]
    workspace_dropdown: TemplateChild<gtk::DropDown>,
    #[template_child]
    add_button: TemplateChild<gtk::Button>,
    #[template_child]
    cancel_button: TemplateChild<gtk::Button>,

    /// The catalogue games, in the order they appear in the list. The row after
    /// the last of these is the escape hatch.
    presets: RefCell<Vec<Preset>>,
    /// The game chosen on stage one, or `None` for the escape hatch. Only
    /// meaningful once `stages` shows its "details" page.
    chosen: RefCell<Option<Preset>>,
    /// The `Workspace` drop-down's rows, in the order they were loaded — index
    /// `i` there is `workspace_dropdown`'s row `i` (item 11 task 07).
    destinations: RefCell<Vec<WorkspaceId>>,
    pub(super) on_confirmed: RefCell<Option<ConfirmHandler>>,
}

impl std::fmt::Debug for AddGameDialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddGameDialog").finish_non_exhaustive()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for AddGameDialog {
    const NAME: &'static str = "IdleManagerAddGameDialog";
    type Type = super::AddGameDialog;
    type ParentType = gtk::Window;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for AddGameDialog {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();
        self.stages.set_visible_child_name("choose");

        for entry in [self.name_entry.get(), self.address_entry.get()] {
            let dialog = obj.downgrade();
            entry.connect_changed(move |_| {
                if let Some(dialog) = dialog.upgrade() {
                    dialog.imp().refresh_add_sensitivity();
                }
            });
        }

        let dialog = obj.downgrade();
        self.game_list.connect_activate(move |_, position| {
            if let Some(dialog) = dialog.upgrade() {
                dialog.imp().choose_at(position);
            }
        });

        let dialog = obj.downgrade();
        self.add_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog.upgrade() {
                dialog.imp().confirm();
            }
        });

        let dialog = obj.downgrade();
        self.cancel_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog.upgrade() {
                dialog.close();
            }
        });

        let dialog = obj.downgrade();
        self.back_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog.upgrade() {
                dialog.imp().stages.set_visible_child_name("choose");
                dialog.imp().refresh_add_sensitivity();
            }
        });

        let key_controller = gtk::EventControllerKey::new();
        let dialog = obj.downgrade();
        key_controller.connect_key_pressed(move |_, key, _, modifiers| {
            let Some(dialog) = dialog.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if key == gdk::Key::Escape {
                dialog.close();
                return glib::Propagation::Stop;
            }
            // `Alt`+`Left` steps stage two back to stage one (2.8's
            // wireframe); a no-op on stage one, where there is nowhere back
            // to go.
            if key == gdk::Key::Left && modifiers.contains(gdk::ModifierType::ALT_MASK) {
                dialog.imp().stages.set_visible_child_name("choose");
                dialog.imp().refresh_add_sensitivity();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        obj.add_controller(key_controller);
    }
}

impl WidgetImpl for AddGameDialog {}
impl WindowImpl for AddGameDialog {}

impl AddGameDialog {
    /// Reads `catalogue` and fills stage one: one row per game, then the escape
    /// hatch; a line under the list for any file that would not parse; and, when
    /// nothing parsed, a line naming the folder in place of the games.
    pub(super) fn load_catalogue(&self, catalogue: &dyn PresetCatalogue) {
        let reading = catalogue.read();

        let names = gtk::StringList::new(&[]);
        for preset in &reading.presets {
            names.append(&preset.display_name);
        }
        names.append(SOMETHING_ELSE);

        self.game_list
            .set_model(Some(&gtk::NoSelection::new(Some(names))));
        self.game_list.set_factory(Some(&self.row_factory()));

        let no_games = reading.presets.is_empty();
        self.presets.replace(reading.presets);

        show_failures(&self.failure_label, &reading.failures);
        self.show_empty_state(no_games, &reading.source);
    }

    /// Fills the `Workspace` drop-down from `destinations`, in the order
    /// given — named workspaces with room, then `Ungrouped` — and selects
    /// `default`, or the first row if `default` is not among them (never
    /// expected: the caller works `default` out from the same book, code
    /// standards rule 1).
    pub(super) fn load_destinations(&self, destinations: &Destinations, default: &WorkspaceId) {
        let names = gtk::StringList::new(&[]);
        let mut ids = Vec::new();
        let mut selected = 0;
        for (index, workspace) in destinations.workspaces().iter().enumerate() {
            names.append(workspace.name());
            if workspace.id() == default {
                selected = index;
            }
            ids.push(workspace.id().clone());
        }

        self.workspace_dropdown.set_model(Some(&names));
        self.workspace_dropdown
            .set_selected(u32::try_from(selected).unwrap_or(0));
        self.destinations.replace(ids);
    }

    fn show_empty_state(&self, empty: bool, source: &str) {
        if empty {
            self.empty_list_label.set_label(&format!(
                "No games are configured. Add preset files to {source}, then reopen this dialog."
            ));
        }
        self.empty_list_label.set_visible(empty);
    }

    /// The factory for stage one's rows: a label, and a hairline above it only
    /// on the escape-hatch row and only when there are games for it to be
    /// separated from (design rule 7).
    fn row_factory(&self) -> gtk::SignalListItemFactory {
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(|_, item| {
            let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
                return;
            };
            let row = gtk::Box::new(gtk::Orientation::Vertical, 0);
            let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
            separator.set_visible(false);
            separator.set_margin_bottom(6);
            let label = gtk::Label::builder().xalign(0.0).build();
            row.append(&separator);
            row.append(&label);
            item.set_child(Some(&row));
        });

        let dialog = self.obj().downgrade();
        factory.connect_bind(move |_, item| {
            let Some(dialog) = dialog.upgrade() else {
                return;
            };
            let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
                return;
            };
            let Some(text) = item.item().and_downcast::<gtk::StringObject>() else {
                return;
            };
            let Some(separator) = item.child().and_then(|row| row.first_child()) else {
                return;
            };
            let Some(label) = separator.next_sibling().and_downcast::<gtk::Label>() else {
                return;
            };

            let games = dialog.imp().presets.borrow().len();
            let is_escape_hatch = item.position() as usize >= games;

            label.set_label(&text.string());
            if is_escape_hatch {
                label.add_css_class("dim-label");
            } else {
                label.remove_css_class("dim-label");
            }
            separator.set_visible(is_escape_hatch && games > 0);
        });

        factory
    }

    /// A stage-one row was activated. A position within the games chooses that
    /// game; the row past them is the escape hatch.
    fn choose_at(&self, position: u32) {
        let chosen = self.presets.borrow().get(position as usize).cloned();
        self.advance_to_details(chosen);
    }

    /// Moves to stage two: the account-name field always, plus the address
    /// field on the escape-hatch path.
    fn advance_to_details(&self, preset: Option<Preset>) {
        let escape_hatch = preset.is_none();
        self.chosen_game_label.set_text(
            preset
                .as_ref()
                .map_or("A game the application does not know", |preset| {
                    preset.display_name.as_str()
                }),
        );
        self.chosen.replace(preset);

        self.address_label.set_visible(escape_hatch);
        self.address_entry.set_visible(escape_hatch);
        self.stages.set_visible_child_name("details");
        self.name_entry.grab_focus();
        self.refresh_add_sensitivity();
    }

    fn refresh_add_sensitivity(&self) {
        if self.stages.visible_child_name().as_deref() != Some("details") {
            self.add_button.set_sensitive(false);
            return;
        }

        let name_filled = account_name(&self.name_entry.text()).is_some();
        let ready = if self.chosen.borrow().is_none() {
            name_filled && !self.address_entry.text().trim().is_empty()
        } else {
            name_filled
        };
        self.add_button.set_sensitive(ready);
    }

    fn confirm(&self) {
        let name = account_name(&self.name_entry.text()).unwrap_or_default();
        let workspace = self
            .destinations
            .borrow()
            .get(self.workspace_dropdown.selected() as usize)
            .cloned()
            .unwrap_or_else(WorkspaceId::ungrouped);

        let confirmed = match self.chosen.borrow().clone() {
            Some(preset) => Confirmed::Preset {
                preset,
                account_name: name,
                workspace,
            },
            None => Confirmed::Custom {
                name,
                address: self.address_entry.text().trim().to_owned(),
                workspace,
            },
        };

        if let Some(handler) = self.on_confirmed.borrow().as_ref() {
            handler(&confirmed);
        }
        self.obj().close();
    }
}

/// Fills `label` with one line per failed file — `name: reason` — and shows it,
/// or hides it when nothing failed (design rule 8).
fn show_failures(label: &gtk::Label, failures: &[idle_manager_core::PresetFailure]) {
    if failures.is_empty() {
        label.set_visible(false);
        return;
    }

    // One line per failure: a multi-line `toml` error is trimmed to its first
    // line, which already names the position — the list loses a row, it does not
    // grow a paragraph (design rule 8).
    let lines: Vec<String> = failures
        .iter()
        .map(|failure| {
            let reason = failure.reason.lines().next().unwrap_or_default().trim();
            format!("{}: {reason}", failure.entry)
        })
        .collect();
    label.set_label(&lines.join("\n"));
    label.set_visible(true);
}
